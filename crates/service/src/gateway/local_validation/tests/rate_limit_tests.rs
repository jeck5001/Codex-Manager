use super::*;
use codexmanager_core::storage::{now_ts, ApiKey, Storage};

fn sample_api_key() -> ApiKey {
    ApiKey {
        id: "gk_rate_limit".to_string(),
        name: Some("rate-limit".to_string()),
        model_slug: None,
        reasoning_effort: None,
        client_type: "codex".to_string(),
        protocol_type: "openai_compat".to_string(),
        auth_scheme: "authorization_bearer".to_string(),
        upstream_base_url: None,
        static_headers_json: None,
        key_hash: "hash-rate-limit".to_string(),
        status: "active".to_string(),
        created_at: now_ts(),
        last_used_at: None,
        expires_at: None,
    }
}

#[test]
fn rate_limit_check_skips_keys_without_config() {
    clear_state_for_tests();
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let api_key = sample_api_key();
    storage.insert_api_key(&api_key).expect("insert key");

    let result = check_api_key_rate_limit(
        &storage,
        &api_key,
        br#"{"input":"hello"}"#,
        "/v1/responses",
        false,
    );
    assert!(result.is_ok(), "unconfigured key should not be limited");
}

#[test]
fn rate_limit_check_enforces_rpm_limit() {
    clear_state_for_tests();
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let api_key = sample_api_key();
    storage.insert_api_key(&api_key).expect("insert key");
    storage
        .upsert_api_key_rate_limit(&api_key.id, Some(10), None, None)
        .expect("set rate limit");

    for _ in 0..10 {
        check_api_key_rate_limit(
            &storage,
            &api_key,
            br#"{"input":"hello"}"#,
            "/v1/responses",
            false,
        )
        .expect("request within rpm limit");
    }

    let err = check_api_key_rate_limit(
        &storage,
        &api_key,
        br#"{"input":"hello"}"#,
        "/v1/responses",
        false,
    )
    .expect_err("11th request should be rate limited");
    assert_eq!(err.status_code, 429);
    assert_eq!(err.message, "api key rpm limit exceeded");
    assert!(err.retry_after_secs.unwrap_or_default() >= 1);
}

#[test]
fn rate_limit_check_enforces_tpm_limit() {
    clear_state_for_tests();
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let api_key = sample_api_key();
    storage.insert_api_key(&api_key).expect("insert key");
    storage
        .upsert_api_key_rate_limit(&api_key.id, None, Some(20), None)
        .expect("set rate limit");

    let err = check_api_key_rate_limit(
        &storage,
        &api_key,
        br#"{"input":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        "/v1/responses",
        false,
    )
    .expect_err("large request should exceed tpm");
    assert_eq!(err.status_code, 429);
    assert_eq!(err.message, "api key tpm limit exceeded");
    assert!(err.retry_after_secs.unwrap_or_default() >= 1);
}

#[test]
fn rate_limit_check_enforces_daily_limit() {
    clear_state_for_tests();
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let api_key = sample_api_key();
    storage.insert_api_key(&api_key).expect("insert key");
    storage
        .upsert_api_key_rate_limit(&api_key.id, None, None, Some(2))
        .expect("set rate limit");

    for _ in 0..2 {
        check_api_key_rate_limit(
            &storage,
            &api_key,
            br#"{"input":"hello"}"#,
            "/v1/responses",
            false,
        )
        .expect("request within daily limit");
    }

    let err = check_api_key_rate_limit(
        &storage,
        &api_key,
        br#"{"input":"hello"}"#,
        "/v1/responses",
        false,
    )
    .expect_err("3rd request should exceed daily limit");
    assert_eq!(err.status_code, 429);
    assert_eq!(err.message, "api key daily limit exceeded");
    assert!(err.retry_after_secs.unwrap_or_default() >= 1);
}
