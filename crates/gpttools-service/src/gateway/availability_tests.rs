
use super::should_failover_after_refresh;
use super::request_rewrite::{apply_request_overrides, compute_upstream_url};
use super::{
    account_token_exchange_lock,
    cooldown_reason_for_status, gateway_metrics_prometheus, is_html_content_type,
    is_upstream_challenge_response, normalize_models_path, normalize_upstream_base_url,
    resolve_openai_bearer_token, should_drop_incoming_header,
    should_drop_incoming_header_for_failover, should_try_openai_fallback, CooldownReason,
};
use gpttools_core::storage::{now_ts, Account, Storage, Token, UsageSnapshotRecord};
use reqwest::header::HeaderValue;
use std::sync::Arc;

#[test]
fn failover_on_missing_usage() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let account = Account {
        id: "acc-1".to_string(),
        label: "main".to_string(),
        issuer: "issuer".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        workspace_name: None,
        note: None,
        tags: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert");
    let record = UsageSnapshotRecord {
        account_id: "acc-1".to_string(),
        used_percent: None,
        window_minutes: Some(300),
        resets_at: None,
        secondary_used_percent: Some(10.0),
        secondary_window_minutes: Some(10080),
        secondary_resets_at: None,
        credits_json: None,
        captured_at: now_ts(),
    };
    storage.insert_usage_snapshot(&record).expect("insert usage");

    let should_failover = should_failover_after_refresh(&storage, "acc-1", Ok(()));
    assert!(should_failover);
}

#[test]
fn html_content_type_detection() {
    assert!(is_html_content_type("text/html; charset=utf-8"));
    assert!(is_html_content_type("TEXT/HTML"));
    assert!(!is_html_content_type("application/json"));
}

#[test]
fn compute_url_keeps_v1_for_models_on_codex_backend() {
    let (url, alt) = compute_upstream_url("https://chatgpt.com/backend-api/codex", "/v1/models");
    assert_eq!(url, "https://chatgpt.com/backend-api/codex/models");
    assert_eq!(
        alt.as_deref(),
        Some("https://chatgpt.com/backend-api/codex/v1/models")
    );
    let (url, alt) = compute_upstream_url("https://api.openai.com/v1", "/v1/models");
    assert_eq!(url, "https://api.openai.com/v1/models");
    assert!(alt.is_none());
}

#[test]
fn normalize_upstream_base_url_for_chatgpt_host() {
    assert_eq!(
        normalize_upstream_base_url("https://chatgpt.com"),
        "https://chatgpt.com/backend-api/codex"
    );
    assert_eq!(
        normalize_upstream_base_url("https://chat.openai.com/"),
        "https://chat.openai.com/backend-api/codex"
    );
}

#[test]
fn normalize_upstream_base_url_keeps_existing_backend_path() {
    assert_eq!(
        normalize_upstream_base_url("https://chatgpt.com/backend-api/codex/"),
        "https://chatgpt.com/backend-api/codex"
    );
    assert_eq!(
        normalize_upstream_base_url("https://api.openai.com/v1/"),
        "https://api.openai.com/v1"
    );
}

#[test]
fn apply_request_overrides_accepts_xhigh() {
    let body = br#"{"model":"gpt-5.3-codex","reasoning":{"effort":"medium"}}"#.to_vec();
    let updated = apply_request_overrides("/v1/responses", body, None, Some("xhigh"));
    let value: serde_json::Value = serde_json::from_slice(&updated).expect("json");
    assert_eq!(value["reasoning"]["effort"], "xhigh");
}

#[test]
fn apply_request_overrides_maps_extra_high_to_xhigh() {
    let body = br#"{"model":"gpt-5.3-codex"}"#.to_vec();
    let updated = apply_request_overrides("/v1/responses", body, None, Some("extra_high"));
    let value: serde_json::Value = serde_json::from_slice(&updated).expect("json");
    assert_eq!(value["reasoning"]["effort"], "xhigh");
}

#[test]
fn normalize_models_path_appends_client_version_when_missing() {
    assert_eq!(
        normalize_models_path("/v1/models"),
        "/v1/models?client_version=0.98.0"
    );
    assert_eq!(
        normalize_models_path("/v1/models?foo=1"),
        "/v1/models?foo=1&client_version=0.98.0"
    );
}

#[test]
fn normalize_models_path_keeps_existing_client_version() {
    assert_eq!(
        normalize_models_path("/v1/models?client_version=1.2.3"),
        "/v1/models?client_version=1.2.3"
    );
    assert_eq!(normalize_models_path("/v1/responses"), "/v1/responses");
}

#[test]
fn models_path_does_not_try_openai_fallback() {
    let content_type = HeaderValue::from_str("text/html; charset=utf-8").ok();
    assert!(!should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/models?client_version=0.98.0",
        content_type.as_ref()
    ));
    assert!(should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses",
        content_type.as_ref()
    ));
}

#[test]
fn cooldown_reason_maps_status() {
    assert_eq!(cooldown_reason_for_status(429), CooldownReason::RateLimited);
    assert_eq!(cooldown_reason_for_status(503), CooldownReason::Upstream5xx);
    assert_eq!(cooldown_reason_for_status(403), CooldownReason::Challenge);
    assert_eq!(cooldown_reason_for_status(400), CooldownReason::Upstream4xx);
    assert_eq!(cooldown_reason_for_status(200), CooldownReason::Default);
}

#[test]
fn token_exchange_lock_reuses_same_account_lock() {
    let first = account_token_exchange_lock("acc-1");
    let second = account_token_exchange_lock("acc-1");
    let third = account_token_exchange_lock("acc-2");
    assert!(Arc::ptr_eq(&first, &second));
    assert!(!Arc::ptr_eq(&first, &third));
}

#[test]
fn resolve_openai_bearer_token_uses_cached_storage_value() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let account = Account {
        id: "acc-1".to_string(),
        label: "main".to_string(),
        issuer: "".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        workspace_name: None,
        note: None,
        tags: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "acc-1".to_string(),
            id_token: "id-token".to_string(),
            access_token: "access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            api_key_access_token: Some("cached-api-key-token".to_string()),
            last_refresh: now_ts(),
        })
        .expect("insert token");
    let mut runtime_token = Token {
        account_id: "acc-1".to_string(),
        id_token: "runtime-id-token".to_string(),
        access_token: "runtime-access-token".to_string(),
        refresh_token: "runtime-refresh-token".to_string(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };

    let bearer =
        resolve_openai_bearer_token(&storage, &account, &mut runtime_token).expect("resolve");
    assert_eq!(bearer, "cached-api-key-token");
    assert_eq!(
        runtime_token.api_key_access_token.as_deref(),
        Some("cached-api-key-token")
    );
}

#[test]
fn metrics_prometheus_contains_expected_series() {
    let text = gateway_metrics_prometheus();
    assert!(text.contains("gpttools_gateway_requests_total "));
    assert!(text.contains("gpttools_gateway_requests_active "));
    assert!(text.contains("gpttools_gateway_account_inflight_total "));
    assert!(text.contains("gpttools_gateway_failover_attempts_total "));
    assert!(text.contains("gpttools_gateway_cooldown_marks_total "));
}

#[test]
fn challenge_detection_requires_html_or_429() {
    let html = HeaderValue::from_str("text/html; charset=utf-8").ok();
    let json = HeaderValue::from_str("application/json").ok();
    assert!(is_upstream_challenge_response(403, html.as_ref()));
    assert!(!is_upstream_challenge_response(403, json.as_ref()));
    assert!(is_upstream_challenge_response(429, json.as_ref()));
}

#[test]
fn drop_incoming_header_keeps_session_affinity_for_primary_attempt() {
    assert!(should_drop_incoming_header("ChatGPT-Account-Id"));
    assert!(should_drop_incoming_header("authorization"));
    assert!(should_drop_incoming_header("x-api-key"));
    assert!(!should_drop_incoming_header("session_id"));
    assert!(!should_drop_incoming_header("x-codex-turn-state"));
    assert!(!should_drop_incoming_header("Content-Type"));
}

#[test]
fn drop_incoming_header_for_failover_strips_session_affinity() {
    assert!(should_drop_incoming_header_for_failover("ChatGPT-Account-Id"));
    assert!(should_drop_incoming_header_for_failover("session_id"));
    assert!(should_drop_incoming_header_for_failover("x-codex-turn-state"));
    assert!(!should_drop_incoming_header_for_failover("Content-Type"));
}


