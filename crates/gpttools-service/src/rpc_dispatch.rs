use gpttools_core::rpc::types::{
    AccountListResult, ApiKeyListResult, InitializeResult, JsonRpcRequest, JsonRpcResponse,
    RequestLogListResult, UsageListResult, UsageReadResult,
};
use gpttools_core::storage::{now_ts, Event, Storage};
use serde_json::Value;

use crate::{
    account_delete, account_list, account_update, apikey_create, apikey_delete, apikey_disable,
    apikey_enable, apikey_list, apikey_models, apikey_update_model, auth_login, auth_tokens,
    requestlog_clear, requestlog_list, usage_list, usage_read, usage_refresh,
};

pub(crate) fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => {
            if let Ok(path) = std::env::var("GPTTOOLS_DB_PATH") {
                if let Ok(storage) = Storage::open(path) {
                    let _ = storage.init();
                    let _ = storage.insert_event(&Event {
                        account_id: None,
                        event_type: "initialize".to_string(),
                        message: "service initialized".to_string(),
                        created_at: now_ts(),
                    });
                }
            }
            let result = InitializeResult {
                server_name: "gpttools-service".to_string(),
                version: gpttools_core::core_version().to_string(),
            };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "account/list" => {
            let items = account_list::read_accounts();
            let result = AccountListResult { items };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "account/delete" => {
            let account_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("accountId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = match account_delete::delete_account(account_id) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "account/update" => {
            let account_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("accountId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let sort = req
                .params
                .as_ref()
                .and_then(|v| v.get("sort"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let result = match account_update::update_account_sort(account_id, sort) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "account/login/start" => {
            let login_type = req
                .params
                .as_ref()
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("chatgpt");
            let open_browser = req
                .params
                .as_ref()
                .and_then(|v| v.get("openBrowser"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let note = req
                .params
                .as_ref()
                .and_then(|v| v.get("note"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let tags = req
                .params
                .as_ref()
                .and_then(|v| v.get("tags"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let group_name = req
                .params
                .as_ref()
                .and_then(|v| v.get("groupName"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let workspace_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("workspaceId"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
                .and_then(|v| if v.trim().is_empty() { None } else { Some(v) });
            let result = match auth_login::login_start(
                login_type,
                open_browser,
                note,
                tags,
                group_name,
                workspace_id,
            ) {
                Ok(result) => serde_json::to_value(result).unwrap_or(Value::Null),
                Err(err) => serde_json::json!({ "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "account/login/status" => {
            let login_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("loginId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = auth_login::login_status(login_id);
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "account/login/complete" => {
            let state = req
                .params
                .as_ref()
                .and_then(|v| v.get("state"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let code = req
                .params
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let redirect_uri = req
                .params
                .as_ref()
                .and_then(|v| v.get("redirectUri"))
                .and_then(|v| v.as_str());
            if state.is_empty() || code.is_empty() {
                JsonRpcResponse {
                    id: req.id,
                    result: serde_json::json!({"ok": false, "error": "missing code/state"}),
                }
            } else {
                let result = match auth_tokens::complete_login_with_redirect(
                    state,
                    code,
                    redirect_uri,
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(err) => serde_json::json!({ "ok": false, "error": err }),
                };
                JsonRpcResponse { id: req.id, result }
            }
        }
        "apikey/list" => {
            let result = ApiKeyListResult {
                items: apikey_list::read_api_keys(),
            };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "apikey/create" => {
            let name = req
                .params
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let model_slug = req
                .params
                .as_ref()
                .and_then(|v| v.get("modelSlug"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let reasoning_effort = req
                .params
                .as_ref()
                .and_then(|v| v.get("reasoningEffort"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let result = match apikey_create::create_api_key(name, model_slug, reasoning_effort) {
                Ok(result) => serde_json::to_value(result).unwrap_or(Value::Null),
                Err(err) => serde_json::json!({ "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "apikey/models" => {
            let result = match apikey_models::read_model_options() {
                Ok(result) => serde_json::to_value(result).unwrap_or(Value::Null),
                Err(err) => serde_json::json!({ "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "apikey/updateModel" => {
            let key_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let model_slug = req
                .params
                .as_ref()
                .and_then(|v| v.get("modelSlug"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let reasoning_effort = req
                .params
                .as_ref()
                .and_then(|v| v.get("reasoningEffort"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let result = match apikey_update_model::update_api_key_model(
                key_id,
                model_slug,
                reasoning_effort,
            ) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "apikey/delete" => {
            let key_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = match apikey_delete::delete_api_key(key_id) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "apikey/disable" => {
            let key_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = match apikey_disable::disable_api_key(key_id) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "account/usage/read" => {
            let account_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("accountId"))
                .and_then(|v| v.as_str());
            let result = UsageReadResult {
                snapshot: usage_read::read_usage_snapshot(account_id),
            };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "account/usage/list" => {
            let result = UsageListResult {
                items: usage_list::read_usage_snapshots(),
            };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "requestlog/list" => {
            let query = req
                .params
                .as_ref()
                .and_then(|v| v.get("query"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let limit = req
                .params
                .as_ref()
                .and_then(|v| v.get("limit"))
                .and_then(|v| v.as_i64());
            let result = RequestLogListResult {
                items: requestlog_list::read_request_logs(query, limit),
            };
            JsonRpcResponse {
                id: req.id,
                result: serde_json::to_value(result).unwrap_or(Value::Null),
            }
        }
        "requestlog/clear" => {
            let result = match requestlog_clear::clear_request_logs() {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "apikey/enable" => {
            let key_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = match apikey_enable::enable_api_key(key_id) {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        "account/usage/refresh" => {
            let account_id = req
                .params
                .as_ref()
                .and_then(|v| v.get("accountId"))
                .and_then(|v| v.as_str());
            let result = match account_id {
                Some(account_id) => usage_refresh::refresh_usage_for_account(account_id),
                None => usage_refresh::refresh_usage_for_all_accounts(),
            };
            let result = match result {
                Ok(_) => serde_json::json!({ "ok": true }),
                Err(err) => serde_json::json!({ "ok": false, "error": err }),
            };
            JsonRpcResponse { id: req.id, result }
        }
        _ => JsonRpcResponse {
            id: req.id,
            result: serde_json::json!({"error": "unknown_method"}),
        },
    }
}
