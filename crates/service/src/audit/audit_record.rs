use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use codexmanager_core::storage::{
    now_ts, Account, AlertChannel, AlertRule, AuditLog, PluginRecord, Storage,
};
use serde_json::{json, Map, Value};

const AUDIT_OPERATOR_HEADER_KEY: &str = "_operator";

pub(crate) struct PendingAuditLog {
    method: String,
    action: String,
    object_type: String,
    object_id: Option<String>,
    operator: String,
    params: Value,
    before: Option<Value>,
}

pub(crate) fn attach_operator_to_request(req: &mut JsonRpcRequest, operator: Option<&str>) {
    let Some(operator) = operator.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let mut params = req
        .params
        .take()
        .unwrap_or_else(|| Value::Object(Map::new()));
    if !params.is_object() {
        params = Value::Object(Map::new());
    }
    if let Some(object) = params.as_object_mut() {
        object.insert(
            AUDIT_OPERATOR_HEADER_KEY.to_string(),
            Value::String(operator.to_string()),
        );
    }
    req.params = Some(params);
}

pub(crate) fn prepare_rpc_audit(req: &JsonRpcRequest) -> Option<PendingAuditLog> {
    let (action, object_type) = classify_auditable_method(req.method.as_str())?;
    let operator = extract_operator(req.params.as_ref());
    let object_id = extract_object_id(req.method.as_str(), req.params.as_ref());
    let params = sanitize_value(req.params.as_ref().cloned().unwrap_or(Value::Null));
    let before = capture_snapshot(
        req.method.as_str(),
        req.params.as_ref(),
        object_id.as_deref(),
    );

    Some(PendingAuditLog {
        method: req.method.clone(),
        action: action.to_string(),
        object_type: object_type.to_string(),
        object_id,
        operator,
        params,
        before,
    })
}

pub(crate) fn finalize_rpc_audit(pending: Option<PendingAuditLog>, resp: &JsonRpcResponse) {
    let Some(pending) = pending else {
        return;
    };
    if response_failed(&resp.result) {
        return;
    }

    let object_id = pending
        .object_id
        .clone()
        .or_else(|| extract_object_id_from_result(&resp.result));
    let after = capture_snapshot(&pending.method, None, object_id.as_deref())
        .or_else(|| Some(sanitize_value(resp.result.clone())));
    let changes = json!({
        "method": pending.method,
        "params": pending.params,
        "before": pending.before,
        "after": after,
    });

    let item = AuditLog {
        id: 0,
        action: pending.action,
        object_type: pending.object_type,
        object_id,
        operator: pending.operator,
        changes_json: serde_json::to_string(&changes).unwrap_or_else(|_| "{}".to_string()),
        created_at: now_ts(),
    };
    persist_audit_log(item);
}

fn response_failed(result: &Value) -> bool {
    result.get("error").is_some()
        || matches!(
            result.get("ok").and_then(|value| value.as_bool()),
            Some(false)
        )
}

fn extract_operator(params: Option<&Value>) -> String {
    params
        .and_then(|value| value.get(AUDIT_OPERATOR_HEADER_KEY))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn classify_auditable_method(method: &str) -> Option<(&'static str, &'static str)> {
    match method {
        "appSettings/set" => Some(("update", "app_settings")),
        "service/listenConfig/set" => Some(("set", "service_settings")),
        "stats/cost/modelPricing/set" => Some(("set", "model_pricing")),
        "requestlog/clear" => Some(("clear", "request_log")),
        "healthcheck/config/set" => Some(("set", "healthcheck")),
        "healthcheck/run" => Some(("run", "healthcheck")),
        "webAuth/password/set" => Some(("set", "web_auth")),
        "webAuth/password/clear" => Some(("clear", "web_auth")),
        "webAuth/2fa/setup" => Some(("setup", "web_auth")),
        "webAuth/2fa/verify" => Some(("verify", "web_auth")),
        "webAuth/2fa/disable" => Some(("disable", "web_auth")),
        "gateway/routeStrategy/set"
        | "gateway/retryPolicy/set"
        | "gateway/manualAccount/set"
        | "gateway/headerPolicy/set"
        | "gateway/backgroundTasks/set"
        | "gateway/cache/config/set"
        | "gateway/upstreamProxy/set"
        | "gateway/transport/set" => Some(("set", "gateway_settings")),
        "gateway/manualAccount/clear" | "gateway/cache/clear" => {
            Some(("clear", "gateway_settings"))
        }
        "gateway/freeProxy/sync" => Some(("sync", "gateway_settings")),
        "alert/rules/upsert" => Some(("upsert", "alert_rule")),
        "alert/rules/delete" => Some(("delete", "alert_rule")),
        "alert/channels/upsert" => Some(("upsert", "alert_channel")),
        "alert/channels/delete" => Some(("delete", "alert_channel")),
        "plugin/upsert" => Some(("upsert", "plugin")),
        "plugin/delete" => Some(("delete", "plugin")),
        "apikey/create" => Some(("create", "api_key")),
        "apikey/delete" => Some(("delete", "api_key")),
        "apikey/disable" | "apikey/enable" | "apikey/renew" | "apikey/updateModel" => {
            Some(("update", "api_key"))
        }
        "apikey/rateLimit/set"
        | "apikey/modelFallback/set"
        | "apikey/allowedModels/set"
        | "apikey/responseCache/set" => Some(("set", "api_key")),
        "account/delete"
        | "account/deleteMany"
        | "account/deleteUnavailableFree"
        | "account/deleteBanned" => Some(("delete", "account")),
        "account/update"
        | "account/updateMany"
        | "account/updateManyTags"
        | "account/subscription/mark"
        | "account/payment/officialPromoLink/set"
        | "account/teamManager/upload"
        | "account/teamManager/uploadMany" => Some(("update", "account")),
        "account/import" | "account/register/import" | "account/register/importByEmail" => {
            Some(("import", "account"))
        }
        "account/register/start" | "account/register/task" => Some(("create", "account")),
        "account/register/batch/start" | "account/register/outlookBatch/start" => {
            Some(("start", "register_batch"))
        }
        "account/register/batch/cancel" | "account/register/outlookBatch/cancel" => {
            Some(("cancel", "register_batch"))
        }
        "account/register/task/cancel" => Some(("cancel", "register_task")),
        "account/register/task/delete" => Some(("delete", "register_task")),
        "account/register/task/retry" => Some(("retry", "register_task")),
        "account/register/emailServices/create" => Some(("create", "email_service")),
        "account/register/emailServices/update"
        | "account/register/emailServices/setEnabled"
        | "account/register/emailServices/reorder"
        | "account/register/emailServices/outlookBatchImport" => Some(("update", "email_service")),
        "account/register/emailServices/delete"
        | "account/register/emailServices/outlookBatchDelete" => Some(("delete", "email_service")),
        _ => None,
    }
}

fn extract_object_id(method: &str, params: Option<&Value>) -> Option<String> {
    let source = params?.as_object()?;
    let keys = match method {
        "alert/rules/upsert"
        | "alert/rules/delete"
        | "alert/channels/upsert"
        | "alert/channels/delete"
        | "plugin/upsert"
        | "plugin/delete" => &["id"][..],
        "apikey/create" => &["id"],
        "apikey/delete"
        | "apikey/disable"
        | "apikey/enable"
        | "apikey/renew"
        | "apikey/updateModel"
        | "apikey/rateLimit/set"
        | "apikey/modelFallback/set"
        | "apikey/allowedModels/set"
        | "apikey/responseCache/set" => &["id", "keyId"],
        "account/delete" | "account/update" | "account/subscription/mark" => &["accountId", "id"],
        "account/register/task/cancel"
        | "account/register/task/delete"
        | "account/register/task/retry" => &["taskUuid"],
        "account/register/batch/start"
        | "account/register/batch/cancel"
        | "account/register/outlookBatch/start"
        | "account/register/outlookBatch/cancel" => &["batchId"],
        "account/register/emailServices/create"
        | "account/register/emailServices/update"
        | "account/register/emailServices/delete"
        | "account/register/emailServices/setEnabled"
        | "account/register/emailServices/reorder" => &["serviceId", "id"],
        _ => &[
            "id",
            "accountId",
            "keyId",
            "serviceId",
            "batchId",
            "taskUuid",
        ],
    };

    keys.iter().find_map(|key| {
        source
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    })
}

fn extract_object_id_from_result(result: &Value) -> Option<String> {
    let source = result.as_object()?;
    for key in [
        "id",
        "accountId",
        "keyId",
        "serviceId",
        "batchId",
        "taskUuid",
    ] {
        if let Some(value) = source
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
    }
    None
}

fn capture_snapshot(
    method: &str,
    params: Option<&Value>,
    object_id: Option<&str>,
) -> Option<Value> {
    if method.starts_with("account/") {
        return capture_account_snapshot(params, object_id);
    }
    if method.starts_with("apikey/") {
        return capture_api_key_snapshot(object_id);
    }
    if method.starts_with("alert/rules/") {
        return capture_alert_rule_snapshot(object_id);
    }
    if method.starts_with("alert/channels/") {
        return capture_alert_channel_snapshot(object_id);
    }
    if method.starts_with("plugin/") {
        return capture_plugin_snapshot(object_id);
    }
    if method == "stats/cost/modelPricing/set" {
        return crate::stats_model_pricing::read_model_pricing()
            .ok()
            .and_then(|value| serde_json::to_value(value).ok())
            .map(sanitize_value);
    }
    if method == "healthcheck/config/set" || method == "healthcheck/run" {
        return serde_json::to_value(crate::usage_refresh::current_healthcheck_config())
            .ok()
            .map(sanitize_value);
    }
    if matches!(
        method,
        "appSettings/set"
            | "service/listenConfig/set"
            | "webAuth/password/set"
            | "webAuth/password/clear"
            | "webAuth/2fa/setup"
            | "webAuth/2fa/verify"
            | "webAuth/2fa/disable"
    ) || method.starts_with("gateway/")
    {
        return Some(settings_snapshot());
    }
    None
}

fn capture_account_snapshot(params: Option<&Value>, object_id: Option<&str>) -> Option<Value> {
    let storage = crate::storage_helpers::open_storage()?;
    if let Some(ids) = extract_string_array(params, "accountIds") {
        let items = ids
            .iter()
            .filter_map(|id| storage.find_account_by_id(id).ok().flatten())
            .map(account_json)
            .collect::<Vec<_>>();
        return Some(Value::Array(items));
    }
    object_id.and_then(|id| {
        storage
            .find_account_by_id(id)
            .ok()
            .flatten()
            .map(account_json)
    })
}

fn capture_api_key_snapshot(object_id: Option<&str>) -> Option<Value> {
    let storage = crate::storage_helpers::open_storage()?;
    let key_id = object_id?;
    let key = storage.find_api_key_by_id(key_id).ok().flatten()?;
    let allowed_models = storage
        .find_api_key_allowed_models_by_id(key_id)
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_str::<Value>(&value).ok())
        .unwrap_or(Value::Null);
    let rate_limit = storage
        .find_api_key_rate_limit_by_id(key_id)
        .ok()
        .flatten()
        .map(|value| {
            json!({
                "rpm": value.rpm,
                "tpm": value.tpm,
                "dailyLimit": value.daily_limit,
            })
        })
        .unwrap_or(Value::Null);
    let model_fallback = storage
        .find_api_key_model_fallback_by_id(key_id)
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_str::<Value>(&value.model_chain_json).ok())
        .unwrap_or(Value::Null);
    let response_cache = storage
        .find_api_key_response_cache_config_by_id(key_id)
        .ok()
        .flatten()
        .map(|value| json!({ "enabled": value.enabled }))
        .unwrap_or(Value::Null);

    Some(json!({
        "id": key.id,
        "name": key.name,
        "modelSlug": key.model_slug,
        "reasoningEffort": key.reasoning_effort,
        "clientType": key.client_type,
        "protocolType": key.protocol_type,
        "authScheme": key.auth_scheme,
        "upstreamBaseUrl": key.upstream_base_url,
        "hasStaticHeaders": key.static_headers_json.as_ref().map(|value| !value.trim().is_empty()).unwrap_or(false),
        "status": key.status,
        "createdAt": key.created_at,
        "lastUsedAt": key.last_used_at,
        "expiresAt": key.expires_at,
        "allowedModels": allowed_models,
        "rateLimit": rate_limit,
        "modelFallback": model_fallback,
        "responseCache": response_cache,
    }))
}

fn capture_alert_rule_snapshot(object_id: Option<&str>) -> Option<Value> {
    let storage = crate::storage_helpers::open_storage()?;
    let item = storage.find_alert_rule_by_id(object_id?).ok().flatten()?;
    Some(alert_rule_json(item))
}

fn capture_alert_channel_snapshot(object_id: Option<&str>) -> Option<Value> {
    let storage = crate::storage_helpers::open_storage()?;
    let item = storage
        .find_alert_channel_by_id(object_id?)
        .ok()
        .flatten()?;
    Some(alert_channel_json(item))
}

fn capture_plugin_snapshot(object_id: Option<&str>) -> Option<Value> {
    let storage = crate::storage_helpers::open_storage()?;
    let item = storage.find_plugin_by_id(object_id?).ok().flatten()?;
    Some(plugin_json(item))
}

fn extract_string_array(params: Option<&Value>, key: &str) -> Option<Vec<String>> {
    let array = params?.get(key)?.as_array()?;
    let items = array
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn settings_snapshot() -> Value {
    let map = crate::app_settings::list_app_settings_map();
    let mut object = Map::new();
    for (key, value) in map {
        object.insert(key.clone(), sanitize_setting_value(&key, value));
    }
    Value::Object(object)
}

fn sanitize_setting_value(key: &str, value: String) -> Value {
    let normalized = key.to_ascii_lowercase();
    if normalized.contains("password")
        || normalized.contains("token")
        || normalized.ends_with("_api_key")
        || normalized.ends_with("api_key")
    {
        Value::String("[redacted]".to_string())
    } else {
        Value::String(value)
    }
}

fn account_json(item: Account) -> Value {
    json!({
        "id": item.id,
        "label": item.label,
        "issuer": item.issuer,
        "chatgptAccountId": item.chatgpt_account_id,
        "workspaceId": item.workspace_id,
        "groupName": item.group_name,
        "sort": item.sort,
        "status": item.status,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at,
    })
}

fn alert_rule_json(item: AlertRule) -> Value {
    json!({
        "id": item.id,
        "name": item.name,
        "type": item.rule_type,
        "config": serde_json::from_str::<Value>(&item.config_json).unwrap_or(Value::Null),
        "enabled": item.enabled,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at,
    })
}

fn alert_channel_json(item: AlertChannel) -> Value {
    json!({
        "id": item.id,
        "name": item.name,
        "type": item.channel_type,
        "config": serde_json::from_str::<Value>(&item.config_json).unwrap_or(Value::Null),
        "enabled": item.enabled,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at,
    })
}

fn plugin_json(item: PluginRecord) -> Value {
    json!({
        "id": item.id,
        "name": item.name,
        "description": item.description,
        "runtime": item.runtime,
        "hookPoints": serde_json::from_str::<Value>(&item.hook_points_json)
            .unwrap_or_else(|_| Value::Array(Vec::new())),
        "scriptContent": item.script_content,
        "enabled": item.enabled,
        "timeoutMs": item.timeout_ms,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at,
    })
}

fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = Map::new();
            for (key, value) in map {
                if key == AUDIT_OPERATOR_HEADER_KEY || key == "addr" {
                    continue;
                }
                sanitized.insert(key.clone(), sanitize_field_value(&key, value));
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_value).collect()),
        other => other,
    }
}

fn sanitize_field_value(key: &str, value: Value) -> Value {
    let normalized = key.to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "key"
            | "secret"
            | "password"
            | "passwordhash"
            | "access_token"
            | "refresh_token"
            | "api_key"
    ) {
        return Value::String("[redacted]".to_string());
    }
    if normalized.contains("token") && !normalized.ends_with("count") {
        return Value::String("[redacted]".to_string());
    }
    sanitize_value(value)
}

#[cfg(test)]
fn persist_audit_log(item: AuditLog) {
    persist_audit_log_sync(item);
}

#[cfg(not(test))]
fn persist_audit_log(item: AuditLog) {
    let Ok(db_path) = std::env::var("CODEXMANAGER_DB_PATH") else {
        return;
    };
    std::thread::spawn(move || {
        let Ok(storage) = Storage::open(&db_path) else {
            return;
        };
        let _ = storage.insert_audit_log(&item);
    });
}

#[cfg(test)]
fn persist_audit_log_sync(item: AuditLog) {
    let Ok(db_path) = std::env::var("CODEXMANAGER_DB_PATH") else {
        return;
    };
    let Ok(storage) = Storage::open(&db_path) else {
        return;
    };
    let _ = storage.insert_audit_log(&item);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_request;

    #[test]
    fn finalize_rpc_audit_records_account_update_with_operator() {
        let env_lock = crate::lock_utils::process_env_test_guard();
        let db_path = std::env::temp_dir().join(format!(
            "audit-record-{}-{}.db",
            std::process::id(),
            now_ts()
        ));
        std::env::set_var(
            "CODEXMANAGER_DB_PATH",
            db_path.to_string_lossy().to_string(),
        );
        let storage = Storage::open(&db_path).expect("open");
        storage.init().expect("init");
        storage
            .insert_account(&Account {
                id: "acc-audit".to_string(),
                label: "审计账号".to_string(),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: 1,
                status: "disabled".to_string(),
                created_at: now_ts(),
                updated_at: now_ts(),
            })
            .expect("insert account");

        let mut req = JsonRpcRequest {
            id: 1,
            method: "account/update".to_string(),
            params: Some(json!({
                "accountId": "acc-audit",
                "status": "active",
            })),
        };
        attach_operator_to_request(&mut req, Some("desktop-app"));
        let _resp = handle_request(req);

        let rows = storage
            .list_audit_logs_paginated_filtered(
                codexmanager_core::storage::AuditLogFilterInput {
                    action: Some("update"),
                    object_type: Some("account"),
                    object_id: Some("acc-audit"),
                    time_from: None,
                    time_to: None,
                },
                0,
                10,
            )
            .expect("list audit logs");
        assert_eq!(rows.len(), 1);
        let changes: Value = serde_json::from_str(&rows[0].changes_json).expect("parse changes");
        assert_eq!(rows[0].operator, "desktop-app");
        assert_eq!(
            changes
                .get("before")
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str()),
            Some("disabled")
        );
        assert_eq!(
            changes
                .get("after")
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str()),
            Some("active")
        );

        std::env::remove_var("CODEXMANAGER_DB_PATH");
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
        drop(env_lock);
    }
}
