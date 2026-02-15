use gpttools_core::storage::{ApiKey, Storage};
use reqwest::Method;
use tiny_http::Request;

use crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE;

use super::{LocalValidationError, LocalValidationResult};

const DEFAULT_ANTHROPIC_MODEL: &str = "gpt-5.3-codex";
const DEFAULT_ANTHROPIC_REASONING: &str = "high";

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

    if api_key.protocol_type == PROTOCOL_ANTHROPIC_NATIVE {
        return (
            normalized_model.or_else(|| Some(DEFAULT_ANTHROPIC_MODEL.to_string())),
            normalized_reasoning.or_else(|| Some(DEFAULT_ANTHROPIC_REASONING.to_string())),
        );
    }

    (normalized_model, normalized_reasoning)
}

pub(super) fn build_local_validation_result(
    request: &Request,
    storage: Storage,
    mut body: Vec<u8>,
    api_key: ApiKey,
) -> Result<LocalValidationResult, LocalValidationError> {
    // 按当前策略取消每次请求都更新 api_keys.last_used_at，减少并发写入冲突。
    let normalized_path = super::super::normalize_models_path(request.url());
    let adapted = super::super::adapt_request_for_protocol(
        api_key.protocol_type.as_str(),
        &normalized_path,
        body,
    )
    .map_err(|err| LocalValidationError::new(400, err))?;
    let path = adapted.path;
    body = adapted.body;
    let (effective_model, effective_reasoning) = resolve_effective_request_overrides(&api_key);
    body = super::super::apply_request_overrides(
        &path,
        body,
        effective_model.as_deref(),
        effective_reasoning.as_deref(),
    );

    let request_method = request.method().as_str().to_string();
    let method = Method::from_bytes(request_method.as_bytes())
        .map_err(|_| LocalValidationError::new(405, "unsupported method"))?;

    let model_for_log = super::super::extract_request_model(&body).or(api_key.model_slug.clone());
    let reasoning_for_log =
        super::super::extract_request_reasoning_effort(&body).or(api_key.reasoning_effort.clone());
    let is_stream = super::super::extract_request_stream(&body).unwrap_or(false);

    Ok(LocalValidationResult {
        storage,
        path,
        body,
        is_stream,
        response_adapter: adapted.response_adapter,
        request_method,
        key_id: api_key.id,
        model_for_log,
        reasoning_for_log,
        method,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_api_key(protocol_type: &str, model_slug: Option<&str>, reasoning: Option<&str>) -> ApiKey {
        ApiKey {
            id: "gk_test".to_string(),
            name: Some("test".to_string()),
            model_slug: model_slug.map(|value| value.to_string()),
            reasoning_effort: reasoning.map(|value| value.to_string()),
            client_type: "codex".to_string(),
            protocol_type: protocol_type.to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: "hash".to_string(),
            status: "active".to_string(),
            created_at: 0,
            last_used_at: None,
        }
    }

    #[test]
    fn anthropic_key_uses_default_model_and_reasoning() {
        let api_key = sample_api_key(PROTOCOL_ANTHROPIC_NATIVE, None, None);
        let (model, reasoning) = resolve_effective_request_overrides(&api_key);
        assert_eq!(model.as_deref(), Some(DEFAULT_ANTHROPIC_MODEL));
        assert_eq!(reasoning.as_deref(), Some(DEFAULT_ANTHROPIC_REASONING));
    }

    #[test]
    fn anthropic_key_keeps_custom_model_and_normalized_reasoning() {
        let api_key = sample_api_key(PROTOCOL_ANTHROPIC_NATIVE, Some("gpt-5.3-codex"), Some("extra_high"));
        let (model, reasoning) = resolve_effective_request_overrides(&api_key);
        assert_eq!(model.as_deref(), Some("gpt-5.3-codex"));
        assert_eq!(reasoning.as_deref(), Some("xhigh"));
    }

    #[test]
    fn openai_key_keeps_empty_overrides() {
        let api_key = sample_api_key("openai_compat", None, None);
        let (model, reasoning) = resolve_effective_request_overrides(&api_key);
        assert_eq!(model, None);
        assert_eq!(reasoning, None);
    }
}
