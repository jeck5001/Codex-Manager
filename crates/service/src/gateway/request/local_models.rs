use codexmanager_core::rpc::types::ModelOption;
use codexmanager_core::storage::now_ts;
use serde_json::json;
use tiny_http::Response;

const MODEL_CACHE_SCOPE_DEFAULT: &str = "default";
const MODELS_OWNED_BY: &str = "codexmanager";

fn build_openai_models_list(items: &[ModelOption]) -> String {
    let created = now_ts();
    let data = items
        .iter()
        .map(|item| {
            json!({
                "id": item.slug.as_str(),
                "object": "model",
                "created": created,
                "owned_by": MODELS_OWNED_BY,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "object": "list",
        "data": data,
    })
    .to_string()
}

fn fallback_model_options(model_for_log: Option<&str>) -> Vec<ModelOption> {
    let Some(slug) = model_for_log
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };
    vec![ModelOption {
        slug: slug.to_string(),
        display_name: slug.to_string(),
    }]
}

pub(super) struct LocalModelsRequestContext<'a> {
    pub(super) trace_id: &'a str,
    pub(super) key_id: &'a str,
    pub(super) protocol_type: &'a str,
    pub(super) original_path: &'a str,
    pub(super) path: &'a str,
    pub(super) response_adapter: super::ResponseAdapter,
    pub(super) request_method: &'a str,
    pub(super) model_for_log: Option<&'a str>,
    pub(super) reasoning_for_log: Option<&'a str>,
    pub(super) storage: &'a codexmanager_core::storage::Storage,
}

pub(super) fn maybe_respond_local_models(
    request: tiny_http::Request,
    context: LocalModelsRequestContext<'_>,
) -> Result<Option<tiny_http::Request>, String> {
    let is_models_list = context.request_method.eq_ignore_ascii_case("GET")
        && (context.path == "/v1/models" || context.path.starts_with("/v1/models?"));
    if !is_models_list {
        return Ok(Some(request));
    }

    let mut fallback_reason: Option<String> = None;
    let cached_items = match context
        .storage
        .get_model_options_cache(MODEL_CACHE_SCOPE_DEFAULT)
    {
        Ok(Some(record)) => {
            serde_json::from_str::<Vec<ModelOption>>(&record.items_json).unwrap_or_default()
        }
        Ok(None) => Vec::new(),
        Err(err) => {
            let message = format!("model options cache read failed: {err}");
            super::trace_log::log_attempt_result(
                context.trace_id,
                "-",
                None,
                503,
                Some(message.as_str()),
            );
            super::trace_log::log_request_final(
                context.trace_id,
                503,
                None,
                None,
                Some(message.as_str()),
                0,
            );
            super::record_gateway_request_outcome(context.path, 503, Some(context.protocol_type));
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
                    status_code: Some(503),
                    usage: super::request_log::RequestLogUsage::default(),
                    error: Some(message.as_str()),
                    duration_ms: None,
                },
            );
            let response =
                super::error_response::terminal_text_response(503, message, Some(context.trace_id));
            let _ = request.respond(response);
            return Ok(None);
        }
    };

    let items = if !cached_items.is_empty() {
        cached_items
    } else {
        match super::fetch_models_for_picker() {
            Ok(fetched) if !fetched.is_empty() => {
                if let Ok(items_json) = serde_json::to_string(&fetched) {
                    if let Err(err) = context.storage.upsert_model_options_cache(
                        MODEL_CACHE_SCOPE_DEFAULT,
                        items_json.as_str(),
                        now_ts(),
                    ) {
                        log::warn!(
                            "event=gateway_model_options_cache_upsert_failed scope={} err={}",
                            MODEL_CACHE_SCOPE_DEFAULT,
                            err
                        );
                    }
                }
                fetched
            }
            Ok(_) => {
                let message = "models refresh returned empty list".to_string();
                fallback_reason = Some(message);
                fallback_model_options(context.model_for_log)
            }
            Err(err) => {
                let message = format!("models refresh failed: {err}");
                fallback_reason = Some(message);
                fallback_model_options(context.model_for_log)
            }
        }
    };

    let output = build_openai_models_list(&items);
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
            usage: super::request_log::RequestLogUsage::default(),
            error: fallback_reason.as_deref(),
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

#[cfg(test)]
#[path = "tests/local_models_tests.rs"]
mod tests;
