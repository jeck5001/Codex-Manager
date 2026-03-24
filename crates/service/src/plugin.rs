use codexmanager_core::rpc::types::PluginItem;
use codexmanager_core::storage::{now_ts, PluginRecord};
use rand::RngCore;
use serde_json::Value;

use crate::storage_helpers::open_storage;

const ALLOWED_PLUGIN_RUNTIMES: &[&str] = &["lua"];
const ALLOWED_HOOK_POINTS: &[&str] = &["pre_route", "post_route", "post_response"];
const DEFAULT_TIMEOUT_MS: i64 = 100;
const MAX_TIMEOUT_MS: i64 = 60_000;

pub(crate) fn list_plugins() -> Result<Vec<PluginItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .list_plugins()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(to_plugin_item)
        .collect()
}

pub(crate) fn upsert_plugin(
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    runtime: Option<String>,
    hook_points: Option<Value>,
    script_content: Option<String>,
    enabled: Option<bool>,
    timeout_ms: Option<i64>,
) -> Result<PluginItem, String> {
    let normalized_name = name.unwrap_or_default().trim().to_string();
    if normalized_name.is_empty() {
        return Err("plugin name required".to_string());
    }

    let normalized_runtime = normalize_runtime(runtime)?;
    let normalized_hook_points = normalize_hook_points(hook_points)?;

    let normalized_script_content = script_content.unwrap_or_default();
    if normalized_script_content.trim().is_empty() {
        return Err("plugin script content required".to_string());
    }

    let normalized_timeout_ms = normalize_timeout_ms(timeout_ms)?;
    crate::plugin_runtime::validate_plugin_script(
        normalized_runtime.as_str(),
        normalized_script_content.as_str(),
        normalized_timeout_ms,
    )?;
    let normalized_description = description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let now = now_ts();
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let plugin_id = normalize_id(id);
    let created_at = storage
        .find_plugin_by_id(plugin_id.as_str())
        .map_err(|err| err.to_string())?
        .map(|item| item.created_at)
        .unwrap_or(now);

    let record = PluginRecord {
        id: plugin_id,
        name: normalized_name,
        description: normalized_description,
        runtime: normalized_runtime,
        hook_points_json: serde_json::to_string(&normalized_hook_points)
            .map_err(|err| format!("serialize plugin hook points failed: {err}"))?,
        script_content: normalized_script_content,
        enabled: enabled.unwrap_or(true),
        timeout_ms: normalized_timeout_ms,
        created_at,
        updated_at: now,
    };
    storage
        .upsert_plugin(&record)
        .map_err(|err| err.to_string())?;
    crate::plugin_runtime::refresh_plugin_cache(&storage);
    to_plugin_item(record)
}

pub(crate) fn delete_plugin(plugin_id: &str) -> Result<(), String> {
    let normalized_id = plugin_id.trim();
    if normalized_id.is_empty() {
        return Err("plugin id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .delete_plugin(normalized_id)
        .map_err(|err| err.to_string())?;
    crate::plugin_runtime::refresh_plugin_cache(&storage);
    Ok(())
}

pub(crate) fn to_plugin_item(record: PluginRecord) -> Result<PluginItem, String> {
    let hook_points = serde_json::from_str::<Vec<String>>(&record.hook_points_json)
        .map_err(|err| format!("parse plugin hook points failed: {err}"))?;
    Ok(PluginItem {
        id: record.id,
        name: record.name,
        description: record.description,
        runtime: record.runtime,
        hook_points,
        script_content: record.script_content,
        enabled: record.enabled,
        timeout_ms: record.timeout_ms,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn normalize_runtime(runtime: Option<String>) -> Result<String, String> {
    let normalized = runtime
        .unwrap_or_else(|| "lua".to_string())
        .trim()
        .to_ascii_lowercase();
    if ALLOWED_PLUGIN_RUNTIMES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!("unsupported plugin runtime: {normalized}"))
    }
}

fn normalize_hook_points(hook_points: Option<Value>) -> Result<Vec<String>, String> {
    let value = hook_points.ok_or_else(|| "plugin hook points required".to_string())?;
    let array = value
        .as_array()
        .ok_or_else(|| "plugin hook points must be an array".to_string())?;
    let mut normalized = Vec::new();
    for item in array {
        let hook_point = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "plugin hook point must be a non-empty string".to_string())?
            .to_ascii_lowercase();
        if !ALLOWED_HOOK_POINTS.contains(&hook_point.as_str()) {
            return Err(format!("unsupported plugin hook point: {hook_point}"));
        }
        if !normalized.contains(&hook_point) {
            normalized.push(hook_point);
        }
    }
    if normalized.is_empty() {
        Err("plugin hook points required".to_string())
    } else {
        Ok(normalized)
    }
}

fn normalize_timeout_ms(timeout_ms: Option<i64>) -> Result<i64, String> {
    let value = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    if !(1..=MAX_TIMEOUT_MS).contains(&value) {
        return Err(format!(
            "plugin timeout must be between 1 and {MAX_TIMEOUT_MS} ms"
        ));
    }
    Ok(value)
}

fn normalize_id(id: Option<String>) -> String {
    let trimmed = id.unwrap_or_default().trim().to_string();
    if !trimmed.is_empty() {
        return trimmed;
    }

    let mut bytes = [0_u8; 6];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = String::from("plugin_");
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
