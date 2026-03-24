use crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE;
use bytes::Bytes;
use codexmanager_core::storage::ApiKey;
use reqwest::Method;
use tiny_http::Request;

use super::{LocalValidationError, LocalValidationResult};

fn resolve_effective_request_overrides(api_key: &ApiKey) -> (Option<String>, Option<String>) {
    let normalized_model = api_key
        .model_slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let normalized_reasoning = api_key
        .reasoning_effort
        .as_deref()
        .and_then(crate::reasoning_effort::normalize_reasoning_effort)
        .map(str::to_string);

    (normalized_model, normalized_reasoning)
}

fn allow_openai_responses_path_rewrite(protocol_type: &str, normalized_path: &str) -> bool {
    protocol_type == crate::apikey_profile::PROTOCOL_OPENAI_COMPAT
        && (normalized_path.starts_with("/v1/chat/completions")
            || normalized_path.starts_with("/v1/completions"))
}

fn model_is_allowed(allowed_models: &[String], model: &str) -> bool {
    let trimmed = model.trim();
    !trimmed.is_empty() && allowed_models.iter().any(|item| item == trimmed)
}

fn validate_api_key_allowed_model(
    allowed_models: &[String],
    effective_model: Option<&str>,
) -> Result<(), LocalValidationError> {
    if allowed_models.is_empty() {
        return Ok(());
    }

    let Some(model) = effective_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if model_is_allowed(allowed_models, model) {
        return Ok(());
    }

    Err(LocalValidationError::new(
        403,
        format!("api key is not allowed to access model {model}"),
    ))
}

fn validate_api_key_allowed_models(
    allowed_models: &[String],
    requested_model: Option<&str>,
    effective_model: Option<&str>,
) -> Result<(), LocalValidationError> {
    if let Some(requested_model) = requested_model {
        validate_api_key_allowed_model(allowed_models, Some(requested_model))?;
    }

    if let Some(effective_model) = effective_model {
        validate_api_key_allowed_model(allowed_models, Some(effective_model))?;
    }

    Ok(())
}

pub(super) fn build_local_validation_result(
    request: &Request,
    trace_id: String,
    incoming_headers: super::super::IncomingHeaderSnapshot,
    storage: crate::storage_helpers::StorageHandle,
    mut body: Vec<u8>,
    api_key: ApiKey,
) -> Result<LocalValidationResult, LocalValidationError> {
    // 按当前策略取消每次请求都更新 api_keys.last_used_at，减少并发写入冲突。
    let normalized_path = super::super::normalize_models_path(request.url());
    let original_body = body.clone();
    let adapted = super::super::adapt_request_for_protocol(
        api_key.protocol_type.as_str(),
        &normalized_path,
        body,
    )
    .map_err(|err| LocalValidationError::new(400, err))?;
    let mut path = adapted.path;
    let mut response_adapter = adapted.response_adapter;
    let mut tool_name_restore_map = adapted.tool_name_restore_map;
    body = adapted.body;
    if api_key.protocol_type != PROTOCOL_ANTHROPIC_NATIVE
        && !normalized_path.starts_with("/v1/responses")
        && path.starts_with("/v1/responses")
        && !allow_openai_responses_path_rewrite(&api_key.protocol_type, &normalized_path)
    {
        // 中文注释：防回归保护：仅 anthropic_native 的 /v1/messages 允许改写到 /v1/responses；
        // 其余协议和路径一律保持原路径透传，避免客户端按 chat/completions 语义却拿到 responses 流格式。
        log::warn!(
            "event=gateway_protocol_adapt_guard protocol_type={} from_path={} to_path={} action=force_passthrough",
            api_key.protocol_type,
            normalized_path,
            path
        );
        path = normalized_path.clone();
        body = original_body;
        response_adapter = super::super::ResponseAdapter::Passthrough;
        tool_name_restore_map.clear();
    }
    // 中文注释：下游调用方的 stream 语义应在请求改写前确定；
    // 否则上游兼容改写（例如 /responses 强制 stream=true）会污染下游响应模式判断。
    let client_request_meta = super::super::parse_request_metadata(&body);
    let (effective_model, effective_reasoning) = resolve_effective_request_overrides(&api_key);
    let allowed_models = storage
        .find_api_key_allowed_models_by_id(&api_key.id)
        .map_err(|err| LocalValidationError::new(500, format!("storage read failed: {err}")))?
        .map(|raw| crate::apikey_allowed_models::parse_allowed_models(raw.as_str()))
        .unwrap_or_default();
    validate_api_key_allowed_models(
        allowed_models.as_slice(),
        client_request_meta.model.as_deref(),
        effective_model.as_deref(),
    )?;
    body = super::super::apply_request_overrides_with_prompt_cache_key(
        &path,
        body,
        effective_model.as_deref(),
        effective_reasoning.as_deref(),
        api_key.upstream_base_url.as_deref(),
        incoming_headers.conversation_id(),
    );

    let request_method = request.method().as_str().to_string();
    let method = Method::from_bytes(request_method.as_bytes())
        .map_err(|_| LocalValidationError::new(405, "unsupported method"))?;

    let request_meta = super::super::parse_request_metadata(&body);
    let model_for_log = request_meta.model.or(api_key.model_slug.clone());
    let reasoning_for_log = request_meta
        .reasoning_effort
        .or(api_key.reasoning_effort.clone());
    let is_stream = client_request_meta.is_stream;
    let has_prompt_cache_key = client_request_meta.has_prompt_cache_key;
    let request_shape = client_request_meta.request_shape;

    Ok(LocalValidationResult {
        trace_id,
        incoming_headers,
        storage,
        original_path: normalized_path,
        path,
        body: Bytes::from(body),
        is_stream,
        has_prompt_cache_key,
        request_shape,
        protocol_type: api_key.protocol_type,
        upstream_base_url: api_key.upstream_base_url,
        static_headers_json: api_key.static_headers_json,
        response_adapter,
        tool_name_restore_map,
        request_method,
        key_id: api_key.id,
        api_key_name: api_key.name,
        model_for_log,
        reasoning_for_log,
        method,
    })
}

#[cfg(test)]
#[path = "tests/request_tests.rs"]
mod tests;
