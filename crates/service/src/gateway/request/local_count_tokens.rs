use serde_json::{json, Value};
use tiny_http::Response;

use crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE;

fn accumulate_text_len(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.iter().map(accumulate_text_len).sum(),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.chars().count();
            }
            if let Some(content) = map.get("content") {
                return accumulate_text_len(content);
            }
            if let Some(input) = map.get("input") {
                return accumulate_text_len(input);
            }
            map.values().map(accumulate_text_len).sum()
        }
        _ => 0,
    }
}

fn estimate_input_tokens_from_anthropic_messages(body: &[u8]) -> Result<u64, String> {
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| "invalid claude request json".to_string())?;
    let Some(object) = payload.as_object() else {
        return Err("claude request body must be an object".to_string());
    };

    let mut char_count = 0usize;
    if let Some(system) = object.get("system") {
        char_count += accumulate_text_len(system);
    }
    if let Some(messages) = object.get("messages").and_then(Value::as_array) {
        for message in messages {
            if let Some(content) = message.get("content") {
                char_count += accumulate_text_len(content);
            }
        }
    }

    // 中文注释：count_tokens 仅用于本地预算估计，采用稳定的轻量估算（约 4 chars/token）。
    let estimated = ((char_count as u64) / 4).max(1);
    Ok(estimated)
}

pub(super) struct LocalCountTokensRequestContext<'a> {
    pub(super) trace_id: &'a str,
    pub(super) key_id: &'a str,
    pub(super) protocol_type: &'a str,
    pub(super) original_path: &'a str,
    pub(super) path: &'a str,
    pub(super) response_adapter: super::ResponseAdapter,
    pub(super) request_method: &'a str,
    pub(super) body: &'a [u8],
    pub(super) model_for_log: Option<&'a str>,
    pub(super) reasoning_for_log: Option<&'a str>,
    pub(super) storage: &'a codexmanager_core::storage::Storage,
}

pub(super) fn maybe_respond_local_count_tokens(
    request: tiny_http::Request,
    context: LocalCountTokensRequestContext<'_>,
) -> Result<Option<tiny_http::Request>, String> {
    let is_anthropic_count_tokens = context.protocol_type == PROTOCOL_ANTHROPIC_NATIVE
        && context.request_method.eq_ignore_ascii_case("POST")
        && (context.path == "/v1/messages/count_tokens"
            || context.path.starts_with("/v1/messages/count_tokens?"));
    if !is_anthropic_count_tokens {
        return Ok(Some(request));
    }

    match estimate_input_tokens_from_anthropic_messages(context.body) {
        Ok(input_tokens) => {
            let output = json!({ "input_tokens": input_tokens }).to_string();
            super::trace_log::log_attempt_result(context.trace_id, "-", None, 200, None);
            super::trace_log::log_request_final(context.trace_id, 200, None, None, None, 0);
            super::record_gateway_request_outcome(context.path, 200, Some(context.protocol_type));
            super::write_request_log(
                context.storage,
                super::request_log::RequestLogTraceContext {
                    trace_id: Some(context.trace_id),
                    original_path: Some(context.original_path),
                    adapted_path: Some(context.path),
                    response_adapter: Some(context.response_adapter),
                },
                super::request_log::RequestLogEntry {
                    key_id: Some(context.key_id),
                    account_id: None,
                    request_path: context.path,
                    method: context.request_method,
                    model: context.model_for_log,
                    reasoning_effort: context.reasoning_for_log,
                    upstream_url: None,
                    status_code: Some(200),
                    usage: super::request_log::RequestLogUsage {
                        input_tokens: Some(input_tokens.min(i64::MAX as u64) as i64),
                        cached_input_tokens: Some(0),
                        output_tokens: Some(0),
                        total_tokens: Some(input_tokens.min(i64::MAX as u64) as i64),
                        reasoning_output_tokens: Some(0),
                    },
                    error: None,
                    duration_ms: None,
                },
            );
            let response = super::error_response::with_trace_id_header(
                Response::from_string(output)
                    .with_status_code(200)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            b"content-type".as_slice(),
                            b"application/json".as_slice(),
                        )
                        .map_err(|_| "build content-type header failed".to_string())?,
                    ),
                Some(context.trace_id),
            );
            let _ = request.respond(response);
            Ok(None)
        }
        Err(err) => {
            super::trace_log::log_attempt_result(
                context.trace_id,
                "-",
                None,
                400,
                Some(err.as_str()),
            );
            super::trace_log::log_request_final(
                context.trace_id,
                400,
                None,
                None,
                Some(err.as_str()),
                0,
            );
            super::record_gateway_request_outcome(context.path, 400, Some(context.protocol_type));
            super::write_request_log(
                context.storage,
                super::request_log::RequestLogTraceContext {
                    trace_id: Some(context.trace_id),
                    original_path: Some(context.original_path),
                    adapted_path: Some(context.path),
                    response_adapter: Some(context.response_adapter),
                },
                super::request_log::RequestLogEntry {
                    key_id: Some(context.key_id),
                    account_id: None,
                    request_path: context.path,
                    method: context.request_method,
                    model: context.model_for_log,
                    reasoning_effort: context.reasoning_for_log,
                    upstream_url: None,
                    status_code: Some(400),
                    usage: super::request_log::RequestLogUsage::default(),
                    error: Some(err.as_str()),
                    duration_ms: None,
                },
            );
            let response = super::error_response::terminal_text_response(
                400,
                err.clone(),
                Some(context.trace_id),
            );
            let _ = request.respond(response);
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "tests/local_count_tokens_tests.rs"]
mod tests;
