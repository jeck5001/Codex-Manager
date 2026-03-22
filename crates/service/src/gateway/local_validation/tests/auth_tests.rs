use super::*;
use crate::storage_helpers::hash_platform_key;
use codexmanager_core::storage::{now_ts, ApiKey, Storage};

fn sample_api_key(status: &str, expires_at: Option<i64>) -> ApiKey {
    ApiKey {
        id: "gk_auth".to_string(),
        name: Some("auth".to_string()),
        model_slug: None,
        reasoning_effort: None,
        client_type: "codex".to_string(),
        protocol_type: "openai_compat".to_string(),
        auth_scheme: "authorization_bearer".to_string(),
        upstream_base_url: None,
        static_headers_json: None,
        key_hash: hash_platform_key("platform-key"),
        status: status.to_string(),
        created_at: now_ts(),
        last_used_at: None,
        expires_at,
    }
}

#[test]
fn load_active_api_key_rejects_expired_key_with_401_and_marks_status() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    storage
        .insert_api_key(&sample_api_key("active", Some(now_ts() - 1)))
        .expect("insert");

    let err = load_active_api_key(&storage, "platform-key", "/v1/responses", false)
        .expect_err("expired key should be rejected");
    assert_eq!(err.status_code, 401);
    assert_eq!(err.message, "api key expired");

    let current = storage
        .find_api_key_by_hash(&hash_platform_key("platform-key"))
        .expect("reload")
        .expect("key");
    assert_eq!(current.status, "expired");
}

#[test]
fn load_active_api_key_allows_unexpired_active_key() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    storage
        .insert_api_key(&sample_api_key("active", Some(now_ts() + 3600)))
        .expect("insert");

    let current = match load_active_api_key(&storage, "platform-key", "/v1/responses", false) {
        Ok(current) => current,
        Err(err) => panic!(
            "key should remain valid, got status={} message={}",
            err.status_code, err.message
        ),
    };
    assert_eq!(current.id, "gk_auth");
}
