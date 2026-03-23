use super::*;

fn sample_api_key(
    protocol_type: &str,
    model_slug: Option<&str>,
    reasoning: Option<&str>,
) -> ApiKey {
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
        expires_at: None,
    }
}

#[test]
fn anthropic_key_keeps_empty_overrides() {
    let api_key = sample_api_key(crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE, None, None);
    let (model, reasoning) = resolve_effective_request_overrides(&api_key);
    assert_eq!(model, None);
    assert_eq!(reasoning, None);
}

#[test]
fn anthropic_key_applies_custom_model_and_reasoning() {
    let api_key = sample_api_key(
        crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE,
        Some("gpt-5.3-codex"),
        Some("extra_high"),
    );
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

#[test]
fn validate_api_key_allowed_model_accepts_allowed_model() {
    let result =
        validate_api_key_allowed_model(&["o3".to_string(), "o4-mini".to_string()], Some("o4-mini"));

    assert!(result.is_ok());
}

#[test]
fn validate_api_key_allowed_model_rejects_disallowed_model() {
    let error =
        validate_api_key_allowed_model(&["o3".to_string(), "o4-mini".to_string()], Some("gpt-4o"))
            .expect_err("disallowed model should be rejected");

    assert_eq!(error.status_code, 403);
    assert!(error.message.contains("gpt-4o"));
}

#[test]
fn validate_api_key_allowed_models_rejects_disallowed_requested_model_before_override() {
    let error = validate_api_key_allowed_models(&["o3".to_string()], Some("gpt-4o"), Some("o3"))
        .expect_err("disallowed requested model should be rejected before rewrite");

    assert_eq!(error.status_code, 403);
    assert!(error.message.contains("gpt-4o"));
}

#[test]
fn validate_api_key_allowed_models_allows_all_models_when_allowlist_is_empty() {
    let result = validate_api_key_allowed_models(&[], Some("gpt-4o"), Some("o3"));

    assert!(result.is_ok());
}
