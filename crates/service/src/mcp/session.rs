use codexmanager_core::rpc::types::AccountListParams;
use serde_json::{json, Value};
use std::time::Duration;

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const CHAT_COMPLETION_PATH: &str = "/v1/chat/completions";
const MCP_API_KEY_ENV: &str = "CODEXMANAGER_MCP_API_KEY";

pub(crate) fn handle_jsonrpc_message(payload: &[u8]) -> Option<Value> {
    let request: Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(err) => {
            return Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {err}"),
            ))
        }
    };

    handle_jsonrpc_request(request)
}

pub(crate) fn handle_jsonrpc_request(request: Value) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str)?;
    let Some(id) = request.get("id").cloned() else {
        return None;
    };

    if matches!(method, "initialize" | "tools/list" | "tools/call") {
        if let Err(message) = ensure_server_enabled() {
            return Some(error_response(id, -32001, &message));
        }
    }

    match method {
        "initialize" => Some(success_response(
            id,
            json!({
                "protocolVersion": requested_protocol_version(&request)
                    .unwrap_or_else(|| DEFAULT_PROTOCOL_VERSION.to_string()),
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "codexmanager-mcp",
                    "title": "CodexManager MCP",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )),
        "ping" => Some(success_response(id, json!({}))),
        "tools/list" => Some(success_response(id, json!({ "tools": tool_definitions() }))),
        "tools/call" => Some(success_response(
            id,
            handle_tool_call(request.get("params").unwrap_or(&Value::Null)),
        )),
        "notifications/initialized" => None,
        _ => Some(error_response(
            id,
            -32601,
            &format!("method not found: {method}"),
        )),
    }
}

pub(crate) fn ensure_server_enabled() -> Result<(), String> {
    crate::portable::bootstrap_current_process();
    crate::storage_helpers::initialize_storage()
        .map_err(|err| format!("initialize storage failed: {err}"))?;
    crate::sync_runtime_settings_from_storage();
    if crate::current_mcp_enabled() {
        Ok(())
    } else {
        Err("MCP Server 已在设置中禁用；请先在设置页开启后再连接".to_string())
    }
}

fn requested_protocol_version(request: &Value) -> Option<String> {
    request
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "chat_completion",
            "description": "复用 CodexManager 网关发起一次聊天补全请求。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "apiKey": {
                        "type": "string",
                        "description": "平台 API Key；未传时回退到环境变量 CODEXMANAGER_MCP_API_KEY。"
                    },
                    "model": { "type": "string" },
                    "messages": {
                        "type": "array",
                        "items": { "type": "object" }
                    },
                    "stream": { "type": "boolean" }
                },
                "required": ["model", "messages"],
                "additionalProperties": true
            }
        }),
        json!({
            "name": "list_models",
            "description": "列出当前网关可见的模型列表。",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "name": "list_accounts",
            "description": "列出账号状态概览，不返回敏感 token。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": { "type": "string" },
                    "query": { "type": "string" }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "get_usage",
            "description": "读取聚合后的账号用量概览。",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
    ]
}

fn handle_tool_call(params: &Value) -> Value {
    let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
        return tool_error("missing tool name");
    };
    let arguments = params
        .get("arguments")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));

    match execute_tool_call(tool_name, &arguments) {
        Ok(value) => tool_success(value),
        Err(err) => tool_error(&err),
    }
}

fn execute_tool_call(tool_name: &str, arguments: &Value) -> Result<Value, String> {
    match tool_name {
        "chat_completion" => execute_chat_completion(arguments),
        "list_models" => serde_json::to_value(crate::apikey_models::read_model_options(
            arguments
                .get("refreshRemote")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )?)
        .map_err(|err| format!("serialize list_models result failed: {err}")),
        "list_accounts" => {
            let params = AccountListParams {
                page: 1,
                page_size: 500,
                query: arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                filter: arguments
                    .get("filter")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                group_filter: arguments
                    .get("groupFilter")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            };
            serde_json::to_value(crate::account_list::read_accounts(params, false)?)
                .map_err(|err| format!("serialize list_accounts result failed: {err}"))
        }
        "get_usage" => {
            serde_json::to_value(crate::usage_aggregate::read_usage_aggregate_summary()?)
                .map_err(|err| format!("serialize get_usage result failed: {err}"))
        }
        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

fn execute_chat_completion(arguments: &Value) -> Result<Value, String> {
    let api_key = resolve_chat_completion_api_key(arguments)?;
    let payload = build_chat_completion_payload(arguments)?;
    execute_chat_completion_gateway(&payload, api_key.as_str())
}

fn resolve_chat_completion_api_key(arguments: &Value) -> Result<String, String> {
    if let Some(api_key) = arguments
        .get("apiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(api_key.to_string());
    }

    if let Ok(api_key) = std::env::var(MCP_API_KEY_ENV) {
        let trimmed = api_key.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err(format!(
        "missing api key: pass arguments.apiKey or set {MCP_API_KEY_ENV}"
    ))
}

fn build_chat_completion_payload(arguments: &Value) -> Result<Value, String> {
    let Some(model) = arguments
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err("missing required string field: model".to_string());
    };

    if !arguments.get("messages").is_some_and(Value::is_array) {
        return Err("missing required array field: messages".to_string());
    }

    if arguments
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err("chat_completion over MCP currently supports stream=false only".to_string());
    }

    let Some(mut payload) = arguments.as_object().cloned() else {
        return Err("chat_completion arguments must be an object".to_string());
    };
    payload.remove("apiKey");
    payload.insert("model".to_string(), Value::String(model.to_string()));
    payload.insert("stream".to_string(), Value::Bool(false));
    Ok(Value::Object(payload))
}

fn execute_chat_completion_gateway(payload: &Value, api_key: &str) -> Result<Value, String> {
    #[cfg(test)]
    if let Some(result) = try_execute_chat_completion_override(payload, api_key)? {
        return Ok(result);
    }

    execute_chat_completion_gateway_inner(payload, api_key)
}

fn execute_chat_completion_gateway_inner(payload: &Value, api_key: &str) -> Result<Value, String> {
    crate::portable::bootstrap_current_process();
    crate::gateway::reload_runtime_config_from_env();
    crate::storage_helpers::initialize_storage()
        .map_err(|err| format!("initialize storage failed: {err}"))?;
    crate::sync_runtime_settings_from_storage();

    let backend = crate::http::backend_runtime::start_backend_server()
        .map_err(|err| format!("start backend server failed: {err}"))?;
    let result = send_chat_completion_request(&backend.addr, payload, api_key);
    crate::http::backend_runtime::wake_backend_shutdown(&backend.addr);
    let _ = backend.join.join();
    result
}

fn send_chat_completion_request(
    addr: &str,
    payload: &Value,
    api_key: &str,
) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|err| format!("build MCP gateway client failed: {err}"))?;
    let url = format!("http://{addr}{CHAT_COMPLETION_PATH}");
    let response = client
        .post(url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key.trim()),
        )
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|err| format!("gateway chat_completion request failed: {err}"))?;

    let status = response.status();
    let content_type = header_value(&response, reqwest::header::CONTENT_TYPE);
    let actual_model = header_value(&response, "X-CodexManager-Actual-Model");
    let cache = header_value(&response, "X-CodexManager-Cache");
    let trace_id = header_value(&response, "X-CodexManager-Trace-Id");
    let body = response
        .text()
        .map_err(|err| format!("read gateway response body failed: {err}"))?;

    if !status.is_success() {
        return Err(format!(
            "gateway chat_completion failed: status={} body={}",
            status.as_u16(),
            truncate_for_error(body.as_str())
        ));
    }

    let response_body = if content_type
        .as_deref()
        .is_some_and(|value| value.contains("json"))
    {
        serde_json::from_str(&body).map_err(|err| format!("decode gateway json failed: {err}"))?
    } else {
        Value::String(body)
    };

    Ok(json!({
        "response": response_body,
        "gateway": {
            "status": status.as_u16(),
            "contentType": content_type,
            "actualModel": actual_model,
            "cache": cache,
            "traceId": trace_id
        }
    }))
}

fn header_value(
    response: &reqwest::blocking::Response,
    name: impl reqwest::header::AsHeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn truncate_for_error(body: &str) -> String {
    const MAX_CHARS: usize = 400;
    let mut truncated = String::new();
    for (idx, ch) in body.chars().enumerate() {
        if idx >= MAX_CHARS {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

#[cfg(test)]
type ChatCompletionOverride = dyn Fn(&Value, &str) -> Result<Value, String> + Send + Sync + 'static;

#[cfg(test)]
fn try_execute_chat_completion_override(
    payload: &Value,
    api_key: &str,
) -> Result<Option<Value>, String> {
    let guard = chat_completion_override_slot()
        .lock()
        .expect("lock chat_completion override");
    match guard.as_ref() {
        Some(handler) => handler(payload, api_key).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
fn chat_completion_override_slot() -> &'static std::sync::Mutex<Option<Box<ChatCompletionOverride>>>
{
    static SLOT: std::sync::OnceLock<std::sync::Mutex<Option<Box<ChatCompletionOverride>>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
pub(crate) struct ChatCompletionOverrideGuard;

#[cfg(test)]
impl Drop for ChatCompletionOverrideGuard {
    fn drop(&mut self) {
        *chat_completion_override_slot()
            .lock()
            .expect("lock chat_completion override") = None;
    }
}

#[cfg(test)]
pub(crate) fn install_chat_completion_override<F>(handler: F) -> ChatCompletionOverrideGuard
where
    F: Fn(&Value, &str) -> Result<Value, String> + Send + Sync + 'static,
{
    *chat_completion_override_slot()
        .lock()
        .expect("lock chat_completion override") = Some(Box::new(handler));
    ChatCompletionOverrideGuard
}

fn tool_success(value: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|_| value.to_string())
            }
        ],
        "structuredContent": value,
        "isError": false
    })
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "isError": true
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{handle_jsonrpc_message, handle_jsonrpc_request};
    use serde_json::json;

    #[test]
    fn session_handler_reports_parse_errors_without_transport() {
        let response = handle_jsonrpc_message(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list""#)
            .expect("parse response");
        assert_eq!(response["error"]["code"], -32700);
    }

    #[test]
    fn session_handler_ignores_initialized_notification_without_transport() {
        let response = handle_jsonrpc_request(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        assert!(response.is_none());
    }
}
