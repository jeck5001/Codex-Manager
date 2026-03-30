use codexmanager_core::rpc::types::{
    InstalledPluginSummary, JsonRpcRequest, JsonRpcResponse, PluginCatalogEntry, PluginCatalogTask,
    PluginTaskSummary,
};
use codexmanager_core::storage::{now_ts, PluginInstall, PluginTask};
use serde_json::Value;

use crate::storage_helpers::open_storage;

const BUILTIN_MARKET_SOURCE_URL: &str = "builtin://codexmanager";
const BUILTIN_CLEANUP_TASK_INTERVAL_SECS: i64 = 60;

pub(crate) fn handle_catalog_list(req: &JsonRpcRequest) -> JsonRpcResponse {
    match catalog_list_result(req) {
        Ok(value) => super::json_response(req, value),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn handle_install(req: &JsonRpcRequest) -> JsonRpcResponse {
    match install_or_update_plugin(req, false) {
        Ok(value) => super::json_response(req, value),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn handle_update(req: &JsonRpcRequest) -> JsonRpcResponse {
    match install_or_update_plugin(req, true) {
        Ok(value) => super::json_response(req, value),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn handle_uninstall(req: &JsonRpcRequest) -> JsonRpcResponse {
    let Some(plugin_id) = string_param(req, "pluginId").or_else(|| string_param(req, "plugin_id"))
    else {
        return super::json_response(req, error_result("missing pluginId"));
    };

    let Some(storage) = open_storage() else {
        return super::json_response(req, error_result("storage unavailable"));
    };
    if storage.delete_plugin_install(&plugin_id).is_err() {
        return super::json_response(req, error_result("uninstall plugin failed"));
    }
    super::json_response(req, serde_json::json!({ "ok": true }))
}

fn error_result(message: impl Into<String>) -> Value {
    crate::error_codes::rpc_error_payload(message.into())
}

fn string_param(req: &JsonRpcRequest, key: &str) -> Option<String> {
    req.params
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn catalog_list_result(req: &JsonRpcRequest) -> Result<Value, String> {
    let source_url = source_url_from_request(req);
    let items = fetch_catalog_entries(source_url.as_deref())?;
    Ok(serde_json::json!({
        "sourceUrl": source_url.unwrap_or_default(),
        "items": items,
    }))
}

fn source_url_from_request(req: &JsonRpcRequest) -> Option<String> {
    string_param(req, "sourceUrl")
        .or_else(|| string_param(req, "source_url"))
        .or_else(current_market_source_url)
}

pub(crate) fn current_market_source_url() -> Option<String> {
    crate::app_settings::list_app_settings_map()
        .get(crate::app_settings::APP_SETTING_PLUGIN_MARKET_SOURCE_URL_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn fetch_catalog_entries(
    source_url: Option<&str>,
) -> Result<Vec<PluginCatalogEntry>, String> {
    if let Some(source_url) = source_url {
        let normalized = source_url.trim();
        if !normalized.is_empty() {
            match super::runtime::fetch_text(normalized) {
                Ok(text) => {
                    let value: Value = serde_json::from_str(&text)
                        .map_err(|err| format!("parse catalog response failed: {err}"))?;
                    let items = if let Some(items) =
                        value.get("items").and_then(|value| value.as_array())
                    {
                        items.clone()
                    } else if let Some(items) = value.as_array() {
                        items.clone()
                    } else {
                        Vec::new()
                    };
                    let normalized_items = items
                        .into_iter()
                        .filter_map(|item| parse_catalog_entry_value(&item, Some(normalized)).ok())
                        .collect::<Vec<_>>();
                    if !normalized_items.is_empty() {
                        return Ok(normalized_items);
                    }
                }
                Err(err) => {
                    log::warn!("fetch plugin catalog failed, fallback builtin: {err}");
                }
            }
        }
    }
    Ok(builtin_catalog_entries())
}

fn builtin_catalog_entries() -> Vec<PluginCatalogEntry> {
    vec![PluginCatalogEntry {
        id: "cleanup-banned-accounts".to_string(),
        name: "清理封禁账号".to_string(),
        version: "1.0.0".to_string(),
        description: Some("一键清理所有状态为 banned 的账号，适合做批量收尾整理。".to_string()),
        author: Some("CodexManager".to_string()),
        homepage_url: None,
        script_url: None,
        script_body: Some(
            r#"
fn run(context) {
    log("开始清理封禁账号：" + context["plugin"]["name"]);
    let result = cleanup_banned_accounts();
    log("清理完成，删除 " + result["deleted"].to_string() + " 个账号");
    result
}
"#
            .to_string(),
        ),
        permissions: vec!["accounts:cleanup".to_string()],
        tasks: vec![PluginCatalogTask {
            id: "run".to_string(),
            name: "定时自动清理".to_string(),
            description: Some("每 60 秒自动清理一次所有封禁账号".to_string()),
            entrypoint: "run".to_string(),
            schedule_kind: "interval".to_string(),
            interval_seconds: Some(BUILTIN_CLEANUP_TASK_INTERVAL_SECS),
            enabled: true,
        }],
        source_url: Some(BUILTIN_MARKET_SOURCE_URL.to_string()),
    }]
}

pub(crate) fn sync_builtin_cleanup_task_schedule() {
    let Some(storage) = open_storage() else {
        return;
    };
    let Some(install) = storage
        .find_plugin_install("cleanup-banned-accounts")
        .ok()
        .flatten()
    else {
        return;
    };
    if install.source_url.as_deref() != Some(BUILTIN_MARKET_SOURCE_URL) {
        return;
    }
    let Some(task) = storage
        .find_plugin_task("cleanup-banned-accounts::run")
        .ok()
        .flatten()
    else {
        return;
    };
    if task.schedule_kind != "manual" {
        return;
    }
    let Some(entry) = builtin_catalog_entries().into_iter().next() else {
        return;
    };
    let Some(task_entry) = entry.tasks.into_iter().next() else {
        return;
    };
    let next_run_at = if install.status == "enabled" && task.enabled {
        task_entry
            .interval_seconds
            .filter(|value| *value > 0)
            .map(|value| now_ts() + value)
    } else {
        None
    };
    let Ok(task_json) = serde_json::to_string(&task_entry) else {
        return;
    };
    let _ = storage.update_plugin_task_definition(
        &task.id,
        &task_entry.name,
        task_entry.description.as_deref(),
        &task_entry.entrypoint,
        &task_entry.schedule_kind,
        task_entry.interval_seconds,
        task.enabled,
        next_run_at,
        &task_json,
    );
}

pub(crate) fn parse_catalog_entry_value(
    value: &Value,
    source_url: Option<&str>,
) -> Result<PluginCatalogEntry, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "plugin entry must be an object".to_string())?;
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing plugin id".to_string())?;
    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(id)
        .to_string();
    let version = obj
        .get("version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("0.0.0")
        .to_string();
    let description = obj
        .get("description")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let author = obj
        .get("author")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let homepage_url = obj
        .get("homepageUrl")
        .or_else(|| obj.get("homepage_url"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let script_url = obj
        .get("scriptUrl")
        .or_else(|| obj.get("script_url"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let script_body = obj
        .get("scriptBody")
        .or_else(|| obj.get("script_body"))
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let permissions = obj
        .get("permissions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|text| text.trim().to_string()))
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tasks = obj
        .get("tasks")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(parse_catalog_task_value)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(PluginCatalogEntry {
        id: id.to_string(),
        name,
        version,
        description,
        author,
        homepage_url,
        script_url,
        script_body,
        permissions,
        tasks,
        source_url: source_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::builtin_catalog_entries;

    #[test]
    fn builtin_catalog_only_exposes_cleanup_banned_accounts_plugin() {
        let items = builtin_catalog_entries();
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.id, "cleanup-banned-accounts");
        assert!(item
            .permissions
            .iter()
            .any(|item| item == "accounts:cleanup"));
        assert_eq!(item.tasks.len(), 1);
        assert_eq!(item.tasks[0].entrypoint, "run");
        assert_eq!(item.tasks[0].schedule_kind, "interval");
        assert_eq!(
            item.tasks[0].interval_seconds,
            Some(super::BUILTIN_CLEANUP_TASK_INTERVAL_SECS)
        );
    }
}

fn parse_catalog_task_value(value: &Value) -> Result<PluginCatalogTask, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "task entry must be an object".to_string())?;
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing task id".to_string())?;
    Ok(PluginCatalogTask {
        id: id.to_string(),
        name: obj
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(id)
            .to_string(),
        description: obj
            .get("description")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        entrypoint: obj
            .get("entrypoint")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("run")
            .to_string(),
        schedule_kind: obj
            .get("scheduleKind")
            .or_else(|| obj.get("schedule_kind"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("manual")
            .to_string(),
        interval_seconds: obj
            .get("intervalSeconds")
            .or_else(|| obj.get("interval_seconds"))
            .and_then(Value::as_i64),
        enabled: obj.get("enabled").and_then(Value::as_bool).unwrap_or(true),
    })
}

fn build_plugin_tasks(entry: &PluginCatalogEntry, now: i64) -> Result<Vec<PluginTask>, String> {
    entry
        .tasks
        .iter()
        .map(|task| {
            let task_json = serde_json::to_string(task)
                .map_err(|err| format!("serialize task manifest failed: {err}"))?;
            let next_run_at = if task.schedule_kind == "manual" {
                None
            } else {
                task.interval_seconds.and_then(|interval| {
                    if interval > 0 {
                        Some(now + interval)
                    } else {
                        None
                    }
                })
            };
            Ok(PluginTask {
                id: format!("{}::{}", entry.id, task.id),
                plugin_id: entry.id.clone(),
                name: task.name.clone(),
                description: task.description.clone(),
                entrypoint: task.entrypoint.clone(),
                schedule_kind: task.schedule_kind.clone(),
                interval_seconds: task.interval_seconds,
                enabled: task.enabled,
                next_run_at,
                last_run_at: None,
                last_status: None,
                last_error: None,
                task_json,
                created_at: now,
                updated_at: now,
            })
        })
        .collect()
}

fn install_or_update_plugin(req: &JsonRpcRequest, is_update: bool) -> Result<Value, String> {
    let entry_value = req
        .params
        .as_ref()
        .and_then(|value| value.get("entry"))
        .cloned();
    let source_url = source_url_from_request(req);
    let entry = if let Some(value) = entry_value {
        parse_catalog_entry_value(&value, source_url.as_deref())?
    } else {
        let plugin_id = string_param(req, "pluginId")
            .or_else(|| string_param(req, "plugin_id"))
            .ok_or_else(|| "missing pluginId".to_string())?;
        let items = fetch_catalog_entries(source_url.as_deref())?;
        items
            .into_iter()
            .find(|item| item.id == plugin_id)
            .ok_or_else(|| format!("plugin not found: {plugin_id}"))?
    };

    let script_body = entry
        .script_body
        .clone()
        .or_else(|| entry.script_url.as_ref().map(|_| String::new()))
        .unwrap_or_default();
    let script_body = if script_body.trim().is_empty() {
        if let Some(script_url) = entry.script_url.as_deref() {
            super::runtime::fetch_text(script_url)?
        } else {
            return Err(format!("plugin script missing: {}", entry.id));
        }
    } else {
        script_body
    };

    let existing_install = if is_update {
        let Some(storage) = open_storage() else {
            return Err("storage unavailable".to_string());
        };
        let Some(existing) = storage
            .find_plugin_install(&entry.id)
            .map_err(|err| err.to_string())?
        else {
            return Err(format!("plugin not installed: {}", entry.id));
        };
        Some(existing)
    } else {
        None
    };

    let permissions_json = serde_json::to_string(&entry.permissions)
        .map_err(|err| format!("serialize permissions failed: {err}"))?;
    let manifest_json = serde_json::to_string(&entry)
        .map_err(|err| format!("serialize plugin manifest failed: {err}"))?;
    let installed_at = now_ts();
    let tasks = build_plugin_tasks(&entry, installed_at)?;
    let plugin = PluginInstall {
        plugin_id: entry.id.clone(),
        source_url: entry.source_url.clone().or(source_url),
        name: entry.name.clone(),
        version: entry.version.clone(),
        description: entry.description.clone(),
        author: entry.author.clone(),
        homepage_url: entry.homepage_url.clone(),
        script_url: entry.script_url.clone(),
        script_body,
        permissions_json,
        manifest_json,
        status: existing_install
            .as_ref()
            .map(|plugin| plugin.status.clone())
            .unwrap_or_else(|| "disabled".to_string()),
        installed_at: existing_install
            .as_ref()
            .map(|plugin| plugin.installed_at)
            .unwrap_or(installed_at),
        updated_at: installed_at,
        last_run_at: existing_install
            .as_ref()
            .and_then(|plugin| plugin.last_run_at),
        last_error: existing_install
            .as_ref()
            .and_then(|plugin| plugin.last_error.clone()),
    };

    let Some(storage) = open_storage() else {
        return Err("storage unavailable".to_string());
    };
    if storage.replace_plugin_install(&plugin, &tasks).is_err() {
        return Err(if is_update {
            "update plugin failed".to_string()
        } else {
            "install plugin failed".to_string()
        });
    }

    let install_summary = to_installed_plugin_summary(&plugin, &tasks);
    Ok(serde_json::json!({
        "plugin": install_summary,
        "tasks": tasks_to_summaries(&plugin, &tasks),
    }))
}

fn to_installed_plugin_summary(
    plugin: &PluginInstall,
    tasks: &[PluginTask],
) -> InstalledPluginSummary {
    let task_count = tasks.len() as i64;
    let enabled_task_count = tasks.iter().filter(|task| task.enabled).count() as i64;
    InstalledPluginSummary {
        plugin_id: plugin.plugin_id.clone(),
        source_url: plugin.source_url.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        description: plugin.description.clone(),
        author: plugin.author.clone(),
        homepage_url: plugin.homepage_url.clone(),
        script_url: plugin.script_url.clone(),
        permissions: serde_json::from_str::<Vec<String>>(&plugin.permissions_json)
            .unwrap_or_default(),
        status: plugin.status.clone(),
        installed_at: plugin.installed_at,
        updated_at: plugin.updated_at,
        last_run_at: plugin.last_run_at,
        last_error: plugin.last_error.clone(),
        task_count,
        enabled_task_count,
    }
}

fn tasks_to_summaries(plugin: &PluginInstall, tasks: &[PluginTask]) -> Vec<PluginTaskSummary> {
    tasks
        .iter()
        .map(|task| PluginTaskSummary {
            id: task.id.clone(),
            plugin_id: plugin.plugin_id.clone(),
            plugin_name: plugin.name.clone(),
            name: task.name.clone(),
            description: task.description.clone(),
            entrypoint: task.entrypoint.clone(),
            schedule_kind: task.schedule_kind.clone(),
            interval_seconds: task.interval_seconds,
            enabled: task.enabled,
            next_run_at: task.next_run_at,
            last_run_at: task.last_run_at,
            last_status: task.last_status.clone(),
            last_error: task.last_error.clone(),
        })
        .collect()
}
