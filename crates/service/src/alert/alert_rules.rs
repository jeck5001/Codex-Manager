use codexmanager_core::rpc::types::AlertRuleItem;
use codexmanager_core::storage::{now_ts, AlertRule};
use rand::RngCore;
use serde_json::Value;

use crate::storage_helpers::open_storage;

const ALLOWED_RULE_TYPES: &[&str] = &[
    "token_refresh_fail",
    "usage_threshold",
    "error_rate",
    "all_unavailable",
];

pub(crate) fn list_alert_rules() -> Result<Vec<AlertRuleItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .list_alert_rules()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(to_alert_rule_item)
        .collect()
}

pub(crate) fn upsert_alert_rule(
    id: Option<String>,
    name: Option<String>,
    rule_type: Option<String>,
    config: Option<Value>,
    enabled: Option<bool>,
) -> Result<AlertRuleItem, String> {
    let normalized_name = name.unwrap_or_default().trim().to_string();
    if normalized_name.is_empty() {
        return Err("alert rule name required".to_string());
    }

    let normalized_type = rule_type.unwrap_or_default().trim().to_ascii_lowercase();
    if !ALLOWED_RULE_TYPES.contains(&normalized_type.as_str()) {
        return Err(format!("unsupported alert rule type: {normalized_type}"));
    }

    let config_value = normalize_config(config)?;
    let now = now_ts();
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let rule_id = normalize_id(id, "ar");
    let created_at = storage
        .find_alert_rule_by_id(rule_id.as_str())
        .map_err(|err| err.to_string())?
        .map(|item| item.created_at)
        .unwrap_or(now);

    let record = AlertRule {
        id: rule_id,
        name: normalized_name,
        rule_type: normalized_type,
        config_json: serde_json::to_string(&config_value)
            .map_err(|err| format!("serialize alert rule config failed: {err}"))?,
        enabled: enabled.unwrap_or(true),
        created_at,
        updated_at: now,
    };
    storage
        .upsert_alert_rule(&record)
        .map_err(|err| err.to_string())?;
    to_alert_rule_item(record)
}

pub(crate) fn delete_alert_rule(rule_id: &str) -> Result<(), String> {
    if rule_id.trim().is_empty() {
        return Err("alert rule id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .delete_alert_rule(rule_id.trim())
        .map_err(|err| err.to_string())
}

pub(crate) fn to_alert_rule_item(rule: AlertRule) -> Result<AlertRuleItem, String> {
    Ok(AlertRuleItem {
        id: rule.id,
        name: rule.name,
        rule_type: rule.rule_type,
        config: serde_json::from_str(rule.config_json.as_str())
            .unwrap_or(Value::Object(Default::default())),
        enabled: rule.enabled,
        created_at: rule.created_at,
        updated_at: rule.updated_at,
    })
}

fn normalize_config(config: Option<Value>) -> Result<Value, String> {
    match config.unwrap_or_else(|| Value::Object(Default::default())) {
        Value::Object(map) => Ok(Value::Object(map)),
        _ => Err("alert rule config must be an object".to_string()),
    }
}

fn normalize_id(id: Option<String>, prefix: &str) -> String {
    let trimmed = id.unwrap_or_default().trim().to_string();
    if !trimmed.is_empty() {
        return trimmed;
    }

    let mut bytes = [0_u8; 6];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = format!("{prefix}_");
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
