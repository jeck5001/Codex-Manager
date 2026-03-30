use codexmanager_core::rpc::types::{
    InstalledPluginSummary, JsonRpcRequest, PluginRunLogSummary, PluginTaskSummary,
};
use codexmanager_core::storage::PluginInstall;
use serde_json::Value;
use std::collections::HashMap;

use crate::storage_helpers::open_storage;

fn error_result(message: impl Into<String>) -> Value {
    crate::error_codes::rpc_error_payload(message.into())
}

fn parse_permissions(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

pub(crate) fn handle_list_installed(req: &JsonRpcRequest) -> codexmanager_core::rpc::types::JsonRpcResponse {
    match list_installed_plugins() {
        Ok(items) => super::json_response(req, serde_json::json!({ "items": items })),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn handle_enable(
    req: &JsonRpcRequest,
    enabled: bool,
) -> codexmanager_core::rpc::types::JsonRpcResponse {
    let Some(plugin_id) = req
        .params
        .as_ref()
        .and_then(|value| value.get("pluginId").or_else(|| value.get("plugin_id")))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return super::json_response(req, error_result("missing pluginId"));
    };

    let Some(storage) = open_storage() else {
        return super::json_response(req, error_result("storage unavailable"));
    };
    if storage
        .update_plugin_install_status(
            &plugin_id,
            if enabled { "enabled" } else { "disabled" },
            None,
        )
        .is_err()
    {
        return super::json_response(req, error_result("update plugin status failed"));
    }
    super::json_response(req, serde_json::json!({ "ok": true }))
}

pub(crate) fn handle_task_list(req: &JsonRpcRequest) -> codexmanager_core::rpc::types::JsonRpcResponse {
    let plugin_id = req
        .params
        .as_ref()
        .and_then(|value| value.get("pluginId").or_else(|| value.get("plugin_id")))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    match list_plugin_tasks(plugin_id.as_deref()) {
        Ok(items) => super::json_response(req, serde_json::json!({ "items": items })),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn handle_log_list(req: &JsonRpcRequest) -> codexmanager_core::rpc::types::JsonRpcResponse {
    let plugin_id = req
        .params
        .as_ref()
        .and_then(|value| value.get("pluginId").or_else(|| value.get("plugin_id")))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let task_id = req
        .params
        .as_ref()
        .and_then(|value| value.get("taskId").or_else(|| value.get("task_id")))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let limit = req
        .params
        .as_ref()
        .and_then(|value| value.get("limit"))
        .and_then(|value| value.as_i64())
        .unwrap_or(50)
        .max(1);
    match list_plugin_run_logs(plugin_id.as_deref(), task_id.as_deref(), limit) {
        Ok(items) => super::json_response(req, serde_json::json!({ "items": items })),
        Err(err) => super::json_response(req, error_result(err)),
    }
}

pub(crate) fn list_installed_plugins() -> Result<Vec<InstalledPluginSummary>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let installs = storage.list_plugin_installs().map_err(|err| err.to_string())?;
    let tasks = storage.list_plugin_tasks(None).map_err(|err| err.to_string())?;
    let mut task_count_by_plugin: HashMap<String, (i64, i64)> = HashMap::new();
    for task in tasks {
        let entry = task_count_by_plugin.entry(task.plugin_id).or_insert((0, 0));
        entry.0 += 1;
        if task.enabled {
            entry.1 += 1;
        }
    }

    installs
        .into_iter()
        .map(|install| {
            let (task_count, enabled_task_count) = task_count_by_plugin
                .get(&install.plugin_id)
                .copied()
                .unwrap_or((0, 0));
            Ok(to_installed_plugin_summary(
                &install,
                task_count,
                enabled_task_count,
            ))
        })
        .collect()
}

pub(crate) fn list_plugin_tasks(
    plugin_id: Option<&str>,
) -> Result<Vec<PluginTaskSummary>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let installs = storage.list_plugin_installs().map_err(|err| err.to_string())?;
    let install_name_map: HashMap<String, String> = installs
        .into_iter()
        .map(|install| (install.plugin_id, install.name))
        .collect();
    storage
        .list_plugin_tasks(plugin_id)
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|task| {
            Ok(PluginTaskSummary {
                id: task.id,
                plugin_id: task.plugin_id.clone(),
                plugin_name: install_name_map
                    .get(&task.plugin_id)
                    .cloned()
                    .unwrap_or_else(|| task.plugin_id.clone()),
                name: task.name,
                description: task.description,
                entrypoint: task.entrypoint,
                schedule_kind: task.schedule_kind,
                interval_seconds: task.interval_seconds,
                enabled: task.enabled,
                next_run_at: task.next_run_at,
                last_run_at: task.last_run_at,
                last_status: task.last_status,
                last_error: task.last_error,
            })
        })
        .collect()
}

pub(crate) fn list_plugin_run_logs(
    plugin_id: Option<&str>,
    task_id: Option<&str>,
    limit: i64,
) -> Result<Vec<PluginRunLogSummary>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let installs = storage.list_plugin_installs().map_err(|err| err.to_string())?;
    let tasks = storage.list_plugin_tasks(None).map_err(|err| err.to_string())?;
    let install_name_map: HashMap<String, String> = installs
        .into_iter()
        .map(|install| (install.plugin_id, install.name))
        .collect();
    let task_name_map: HashMap<String, String> = tasks
        .into_iter()
        .map(|task| (task.id, task.name))
        .collect();

    storage
        .list_plugin_run_logs(plugin_id, task_id, limit)
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|log| {
            Ok(PluginRunLogSummary {
                id: log.id.unwrap_or_default(),
                plugin_id: log.plugin_id.clone(),
                plugin_name: install_name_map.get(&log.plugin_id).cloned(),
                task_id: log.task_id.clone(),
                task_name: log
                    .task_id
                    .as_ref()
                    .and_then(|task_id| task_name_map.get(task_id))
                    .cloned(),
                run_type: log.run_type,
                status: log.status,
                started_at: log.started_at,
                finished_at: log.finished_at,
                duration_ms: log.duration_ms,
                output: log
                    .output_json
                    .as_ref()
                    .and_then(|raw| serde_json::from_str(raw).ok()),
                error: log.error,
            })
        })
        .collect()
}

fn to_installed_plugin_summary(
    plugin: &PluginInstall,
    task_count: i64,
    enabled_task_count: i64,
) -> InstalledPluginSummary {
    InstalledPluginSummary {
        plugin_id: plugin.plugin_id.clone(),
        source_url: plugin.source_url.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        description: plugin.description.clone(),
        author: plugin.author.clone(),
        homepage_url: plugin.homepage_url.clone(),
        script_url: plugin.script_url.clone(),
        permissions: parse_permissions(&plugin.permissions_json),
        status: plugin.status.clone(),
        installed_at: plugin.installed_at,
        updated_at: plugin.updated_at,
        last_run_at: plugin.last_run_at,
        last_error: plugin.last_error.clone(),
        task_count,
        enabled_task_count,
    }
}
