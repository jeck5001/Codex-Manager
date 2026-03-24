use super::*;
use codexmanager_core::storage::{now_ts, Storage};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::MutexGuard;
use totp_rs::{Algorithm, Secret, TOTP};

static TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn new_test_db_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}-{}-{}-{}.db",
        std::process::id(),
        now_ts(),
        TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    path
}

struct TestDbScope {
    _env_lock: MutexGuard<'static, ()>,
    _db_guard: EnvGuard,
    db_path: PathBuf,
}

impl Drop for TestDbScope {
    fn drop(&mut self) {
        crate::storage_helpers::clear_storage_cache_for_tests();
        remove_sqlite_test_artifacts(&self.db_path);
    }
}

fn setup_test_db(prefix: &str) -> (TestDbScope, Storage) {
    let env_lock = crate::lock_utils::process_env_test_guard();
    crate::storage_helpers::clear_storage_cache_for_tests();
    let db_path = new_test_db_path(prefix);
    let db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");
    (
        TestDbScope {
            _env_lock: env_lock,
            _db_guard: db_guard,
            db_path,
        },
        storage,
    )
}

fn remove_sqlite_test_artifacts(db_path: &PathBuf) {
    let _ = fs::remove_file(db_path);
    let shm_path = PathBuf::from(format!("{}-shm", db_path.display()));
    let wal_path = PathBuf::from(format!("{}-wal", db_path.display()));
    let _ = fs::remove_file(shm_path);
    let _ = fs::remove_file(wal_path);
}

fn current_web_auth_totp(secret: &str) -> String {
    let secret_bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("decode 2fa secret");
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("CodexManager".to_string()),
        "Web Access".to_string(),
    )
    .expect("build totp")
    .generate_current()
    .expect("generate current totp")
}

struct RegisteredAlertWebhook {
    url: String,
}

impl RegisteredAlertWebhook {
    fn new(url: impl Into<String>, sender: std::sync::mpsc::Sender<String>) -> Self {
        let url = url.into();
        crate::alert_sender::register_test_webhook(&url, sender);
        Self { url }
    }
}

impl Drop for RegisteredAlertWebhook {
    fn drop(&mut self) {
        crate::alert_sender::unregister_test_webhook(&self.url);
    }
}

#[test]
fn login_complete_requires_params() {
    let req = JsonRpcRequest {
        id: 1,
        method: "account/login/complete".to_string(),
        params: None,
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));

    let req = JsonRpcRequest {
        id: 2,
        method: "account/login/complete".to_string(),
        params: Some(serde_json::json!({ "code": "x" })),
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));

    let req = JsonRpcRequest {
        id: 3,
        method: "account/login/complete".to_string(),
        params: Some(serde_json::json!({ "state": "y" })),
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));
}

#[test]
fn web_auth_two_factor_rpc_supports_setup_verify_recovery_and_disable() {
    let (_db_scope, _storage) = setup_test_db("web-auth-2fa-rpc");

    assert!(crate::set_web_access_password(Some("P@ssw0rd!")).expect("set password"));

    let setup_resp = handle_request(JsonRpcRequest {
        id: 101,
        method: "webAuth/2fa/setup".to_string(),
        params: None,
    });
    let secret = setup_resp
        .result
        .get("secret")
        .and_then(|value| value.as_str())
        .expect("2fa secret");
    let setup_token = setup_resp
        .result
        .get("setupToken")
        .and_then(|value| value.as_str())
        .expect("setup token");
    let recovery_code = setup_resp
        .result
        .get("recoveryCodes")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|value| value.as_str())
        .expect("recovery code")
        .to_string();

    let verify_resp = handle_request(JsonRpcRequest {
        id: 102,
        method: "webAuth/2fa/verify".to_string(),
        params: Some(serde_json::json!({
            "setupToken": setup_token,
            "code": current_web_auth_totp(secret),
        })),
    });
    assert_eq!(
        verify_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        verify_resp
            .result
            .get("recoveryCodesRemaining")
            .and_then(|value| value.as_u64()),
        Some(8)
    );

    let status_resp = handle_request(JsonRpcRequest {
        id: 103,
        method: "webAuth/status".to_string(),
        params: None,
    });
    assert_eq!(
        status_resp
            .result
            .get("twoFactorEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let recovery_resp = handle_request(JsonRpcRequest {
        id: 104,
        method: "webAuth/2fa/verify".to_string(),
        params: Some(serde_json::json!({
            "recoveryCode": recovery_code,
        })),
    });
    assert_eq!(
        recovery_resp
            .result
            .get("method")
            .and_then(|value| value.as_str()),
        Some("recovery_code")
    );
    assert_eq!(
        recovery_resp
            .result
            .get("recoveryCodesRemaining")
            .and_then(|value| value.as_u64()),
        Some(7)
    );

    let disable_resp = handle_request(JsonRpcRequest {
        id: 105,
        method: "webAuth/2fa/disable".to_string(),
        params: Some(serde_json::json!({
            "code": current_web_auth_totp(secret),
        })),
    });
    assert_eq!(
        disable_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        disable_resp
            .result
            .get("recoveryCodesRemaining")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
}

#[test]
fn clearing_web_access_password_also_clears_two_factor_state() {
    let (_db_scope, _storage) = setup_test_db("web-auth-2fa-clear");

    assert!(crate::set_web_access_password(Some("P@ssw0rd!")).expect("set password"));

    let setup = crate::web_auth_two_factor_setup().expect("setup 2fa");
    let secret = setup
        .get("secret")
        .and_then(|value| value.as_str())
        .expect("setup secret");
    let setup_token = setup
        .get("setupToken")
        .and_then(|value| value.as_str())
        .expect("setup token");

    crate::web_auth_two_factor_verify(setup_token, &current_web_auth_totp(secret))
        .expect("verify 2fa");
    assert!(crate::web_auth_two_factor_enabled());

    assert!(!crate::set_web_access_password(None).expect("clear password"));
    assert!(!crate::web_access_password_configured());
    assert!(!crate::web_auth_two_factor_enabled());

    let status = crate::web_auth_status_value().expect("status");
    assert_eq!(
        status
            .get("passwordConfigured")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        status
            .get("twoFactorEnabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        status
            .get("recoveryCodesRemaining")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
}

#[test]
fn healthcheck_config_rpc_supports_get_and_set() {
    crate::usage_refresh::clear_session_probe_state_for_tests();
    let (_db_scope, _storage) = setup_test_db("healthcheck-config-rpc");

    let initial_resp = handle_request(JsonRpcRequest {
        id: 4,
        method: "healthcheck/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        initial_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 5,
        method: "healthcheck/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": true,
            "intervalSecs": 1800,
            "sampleSize": 4,
        })),
    });
    assert_eq!(
        set_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_resp
            .result
            .get("intervalSecs")
            .and_then(|value| value.as_u64()),
        Some(1800)
    );
    assert_eq!(
        set_resp
            .result
            .get("sampleSize")
            .and_then(|value| value.as_u64()),
        Some(4)
    );

    let restore_resp = handle_request(JsonRpcRequest {
        id: 6,
        method: "healthcheck/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": false,
            "intervalSecs": 300,
            "sampleSize": 2,
        })),
    });
    assert_eq!(
        restore_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    crate::usage_refresh::clear_session_probe_state_for_tests();
}

#[test]
fn healthcheck_run_rpc_returns_empty_summary_without_probe_candidates() {
    crate::usage_refresh::clear_session_probe_state_for_tests();
    let (_db_scope, _storage) = setup_test_db("healthcheck-run-rpc");

    let run_resp = handle_request(JsonRpcRequest {
        id: 7,
        method: "healthcheck/run".to_string(),
        params: None,
    });
    assert_eq!(
        run_resp
            .result
            .get("sampledAccounts")
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert_eq!(
        run_resp
            .result
            .get("failureCount")
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert!(run_resp
        .result
        .get("startedAt")
        .and_then(|value| value.as_i64())
        .is_some());

    let config_resp = handle_request(JsonRpcRequest {
        id: 8,
        method: "healthcheck/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        config_resp
            .result
            .get("recentRun")
            .and_then(|value| value.get("sampledAccounts"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    crate::usage_refresh::clear_session_probe_state_for_tests();
}

#[test]
fn dashboard_health_rpc_includes_recent_healthcheck_after_run() {
    crate::usage_refresh::clear_session_probe_state_for_tests();
    let (_db_scope, _storage) = setup_test_db("dashboard-health-recent-healthcheck");

    let run_resp = handle_request(JsonRpcRequest {
        id: 81,
        method: "healthcheck/run".to_string(),
        params: None,
    });
    let recent_run = run_resp.result.clone();
    assert_eq!(
        recent_run
            .get("sampledAccounts")
            .and_then(|value| value.as_i64()),
        Some(0)
    );

    let dashboard_resp = handle_request(JsonRpcRequest {
        id: 82,
        method: "dashboard/health".to_string(),
        params: None,
    });
    assert_eq!(
        dashboard_resp
            .result
            .get("recentHealthcheck")
            .and_then(|value| value.get("startedAt"))
            .and_then(|value| value.as_i64()),
        recent_run.get("startedAt").and_then(|value| value.as_i64())
    );
    assert_eq!(
        dashboard_resp
            .result
            .get("recentHealthcheck")
            .and_then(|value| value.get("sampledAccounts"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert_eq!(
        dashboard_resp
            .result
            .get("recentHealthcheck")
            .and_then(|value| value.get("failureCount"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );

    crate::usage_refresh::clear_session_probe_state_for_tests();
}

#[test]
fn gateway_retry_policy_rpc_supports_get_set_and_snapshot() {
    let _retry_guard = crate::gateway::retry_policy_test_guard();
    crate::gateway::reset_retry_policy_for_tests();
    let (_db_scope, _storage) = setup_test_db("gateway-retry-policy-rpc");

    let initial_resp = handle_request(JsonRpcRequest {
        id: 9,
        method: "gateway/retryPolicy/get".to_string(),
        params: None,
    });
    assert_eq!(
        initial_resp
            .result
            .get("maxRetries")
            .and_then(|value| value.as_u64()),
        Some(3)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 10,
        method: "gateway/retryPolicy/set".to_string(),
        params: Some(serde_json::json!({
            "maxRetries": 5,
            "backoffStrategy": "fixed",
            "retryableStatusCodes": [429, 502]
        })),
    });
    assert_eq!(
        set_resp
            .result
            .get("maxRetries")
            .and_then(|value| value.as_u64()),
        Some(5)
    );
    assert_eq!(
        set_resp
            .result
            .get("backoffStrategy")
            .and_then(|value| value.as_str()),
        Some("fixed")
    );
    assert_eq!(
        set_resp
            .result
            .get("retryableStatusCodes")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_u64())
                    .collect::<Vec<_>>()
            }),
        Some(vec![429, 502])
    );

    let snapshot = crate::app_settings_get().expect("app settings snapshot");
    assert_eq!(
        snapshot
            .get("retryPolicyMaxRetries")
            .and_then(|value| value.as_u64()),
        Some(5)
    );
    assert_eq!(
        snapshot
            .get("retryPolicyBackoffStrategy")
            .and_then(|value| value.as_str()),
        Some("fixed")
    );
    assert_eq!(
        snapshot
            .get("retryPolicyRetryableStatusCodes")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_u64())
                    .collect::<Vec<_>>()
            }),
        Some(vec![429, 502])
    );

    crate::gateway::reset_retry_policy_for_tests();
}

#[test]
fn audit_list_read_operation_does_not_create_audit_log() {
    let (_db_scope, storage) = setup_test_db("audit-list-read");

    let create_resp = handle_request(JsonRpcRequest {
        id: 90,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "审计只读校验",
        })),
    });
    assert!(
        create_resp
            .result
            .get("id")
            .and_then(|value| value.as_str())
            .is_some(),
        "apikey/create should succeed and create the baseline audit log"
    );

    let initial_count = storage
        .count_audit_logs_filtered(codexmanager_core::storage::AuditLogFilterInput {
            action: None,
            object_type: None,
            object_id: None,
            time_from: None,
            time_to: None,
        })
        .expect("count initial audit logs");
    assert_eq!(initial_count, 1);

    let list_resp = handle_request(JsonRpcRequest {
        id: 91,
        method: "audit/list".to_string(),
        params: Some(serde_json::json!({
            "page": 1,
            "pageSize": 10,
        })),
    });
    assert_eq!(
        list_resp
            .result
            .get("total")
            .and_then(|value| value.as_i64()),
        Some(1)
    );

    let count_after_read = storage
        .count_audit_logs_filtered(codexmanager_core::storage::AuditLogFilterInput {
            action: None,
            object_type: None,
            object_id: None,
            time_from: None,
            time_to: None,
        })
        .expect("count audit logs after audit/list");
    assert_eq!(count_after_read, initial_count);
}

#[test]
fn apikey_rpc_supports_expires_at_and_renew() {
    let (_db_scope, storage) = setup_test_db("apikey-rpc-expires-at");

    let expires_at = now_ts() + 3600;
    let create_resp = handle_request(JsonRpcRequest {
        id: 10,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "临时分享",
            "expiresAt": expires_at,
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let list_resp = handle_request(JsonRpcRequest {
        id: 11,
        method: "apikey/list".to_string(),
        params: None,
    });
    let created = list_resp
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(|value| value.as_str()) == Some(key_id.as_str())
            })
        })
        .expect("created key in list");
    assert_eq!(
        created.get("expiresAt").and_then(|value| value.as_i64()),
        Some(expires_at)
    );
    assert_eq!(
        created.get("status").and_then(|value| value.as_str()),
        Some("active")
    );

    storage
        .update_api_key_status(&key_id, "expired")
        .expect("mark key expired");

    let renewed_expires_at = now_ts() + 7200;
    let renew_resp = handle_request(JsonRpcRequest {
        id: 12,
        method: "apikey/renew".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "expiresAt": renewed_expires_at,
        })),
    });
    assert_eq!(
        renew_resp
            .result
            .get("ok")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let list_after_renew = handle_request(JsonRpcRequest {
        id: 13,
        method: "apikey/list".to_string(),
        params: None,
    });
    let renewed = list_after_renew
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(|value| value.as_str()) == Some(key_id.as_str())
            })
        })
        .expect("renewed key in list");
    assert_eq!(
        renewed.get("expiresAt").and_then(|value| value.as_i64()),
        Some(renewed_expires_at)
    );
    assert_eq!(
        renewed.get("status").and_then(|value| value.as_str()),
        Some("active")
    );
}

#[test]
fn apikey_rpc_supports_rate_limit_get_and_set() {
    let (_db_scope, _storage) = setup_test_db("apikey-rpc-rate-limit");

    let create_resp = handle_request(JsonRpcRequest {
        id: 20,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "限流测试",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let set_resp = handle_request(JsonRpcRequest {
        id: 21,
        method: "apikey/rateLimit/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "rpm": 10,
            "tpm": 1000,
            "dailyLimit": 50,
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 22,
        method: "apikey/rateLimit/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        get_resp.result.get("rpm").and_then(|value| value.as_i64()),
        Some(10)
    );
    assert_eq!(
        get_resp.result.get("tpm").and_then(|value| value.as_i64()),
        Some(1000)
    );
    assert_eq!(
        get_resp
            .result
            .get("dailyLimit")
            .and_then(|value| value.as_i64()),
        Some(50)
    );
}

#[test]
fn apikey_response_cache_rpc_supports_get_and_set() {
    let (_db_scope, _storage) = setup_test_db("apikey-rpc-response-cache");

    let create_resp = handle_request(JsonRpcRequest {
        id: 23,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "缓存测试",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let initial_get_resp = handle_request(JsonRpcRequest {
        id: 24,
        method: "apikey/responseCache/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        initial_get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 25,
        method: "apikey/responseCache/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "enabled": true,
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let enabled_get_resp = handle_request(JsonRpcRequest {
        id: 26,
        method: "apikey/responseCache/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        enabled_get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[test]
fn gateway_cache_rpc_supports_get_set_stats_and_clear() {
    let (_db_scope, _storage) = setup_test_db("gateway-cache-rpc");

    crate::gateway::clear_response_cache();
    crate::gateway::set_response_cache_enabled(false);

    let get_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "gateway/cache/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "gateway/cache/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": true,
            "ttlSecs": 120,
            "maxEntries": 12,
        })),
    });
    assert_eq!(
        set_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_resp
            .result
            .get("ttlSecs")
            .and_then(|value| value.as_u64()),
        Some(120)
    );
    assert_eq!(
        set_resp
            .result
            .get("maxEntries")
            .and_then(|value| value.as_u64()),
        Some(12)
    );

    let stats_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "gateway/cache/stats".to_string(),
        params: None,
    });
    assert_eq!(
        stats_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        stats_resp
            .result
            .get("entryCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );

    let clear_resp = handle_request(JsonRpcRequest {
        id: 33,
        method: "gateway/cache/clear".to_string(),
        params: None,
    });
    assert_eq!(
        clear_resp
            .result
            .get("entryCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );

    crate::gateway::set_response_cache_enabled(false);
    crate::gateway::clear_response_cache();
}

#[test]
fn stats_cost_model_pricing_rpc_supports_get_and_set() {
    let (_db_scope, _storage) = setup_test_db("stats-cost-model-pricing");

    let set_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "stats/cost/modelPricing/set".to_string(),
        params: Some(serde_json::json!({
            "items": [
                {
                    "modelSlug": "o3",
                    "inputPricePer1k": 0.02,
                    "outputPricePer1k": 0.08
                },
                {
                    "modelSlug": "gpt-4o",
                    "inputPricePer1k": 0.005,
                    "outputPricePer1k": 0.015
                }
            ]
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "stats/cost/modelPricing/get".to_string(),
        params: None,
    });
    let items = get_resp
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("pricing items");
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0].get("modelSlug").and_then(|value| value.as_str()),
        Some("gpt-4o")
    );
    assert_eq!(
        items[1]
            .get("outputPricePer1k")
            .and_then(|value| value.as_f64()),
        Some(0.08)
    );

    let clear_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "stats/cost/modelPricing/set".to_string(),
        params: Some(serde_json::json!({ "items": [] })),
    });
    assert_eq!(
        clear_resp
            .result
            .get("ok")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    let cleared = handle_request(JsonRpcRequest {
        id: 33,
        method: "stats/cost/modelPricing/get".to_string(),
        params: None,
    });
    assert_eq!(
        cleared
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(0)
    );
}

#[test]
fn stats_cost_summary_rpc_aggregates_custom_range() {
    let (_db_scope, storage) = setup_test_db("stats-cost-summary");

    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 1,
            key_id: Some("key-a".to_string()),
            account_id: Some("acc-a".to_string()),
            model: Some("o3".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(20),
            output_tokens: Some(40),
            total_tokens: Some(140),
            reasoning_output_tokens: Some(6),
            estimated_cost_usd: Some(1.5),
            created_at: 1_700_000_000,
        })
        .expect("insert stat 1");
    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 2,
            key_id: Some("key-b".to_string()),
            account_id: Some("acc-b".to_string()),
            model: Some("gpt-4o".to_string()),
            input_tokens: Some(80),
            cached_input_tokens: Some(10),
            output_tokens: Some(30),
            total_tokens: Some(100),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.8),
            created_at: 1_700_086_400,
        })
        .expect("insert stat 2");

    let resp = handle_request(JsonRpcRequest {
        id: 40,
        method: "stats/cost/summary".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });

    assert_eq!(
        resp.result
            .get("total")
            .and_then(|value| value.get("requestCount"))
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        resp.result
            .get("byKey")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(
        resp.result
            .get("byKey")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("keyId"))
            .and_then(|value| value.as_str()),
        Some("key-a")
    );
    assert_eq!(
        resp.result
            .get("byModel")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("model"))
            .and_then(|value| value.as_str()),
        Some("o3")
    );
    assert_eq!(
        resp.result.get("preset").and_then(|value| value.as_str()),
        Some("custom")
    );
}

#[test]
fn stats_cost_export_rpc_returns_csv_content() {
    let (_db_scope, storage) = setup_test_db("stats-cost-export");

    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 1,
            key_id: Some("key-export".to_string()),
            account_id: Some("acc-export".to_string()),
            model: Some("o3".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(20),
            output_tokens: Some(40),
            total_tokens: Some(140),
            reasoning_output_tokens: Some(6),
            estimated_cost_usd: Some(1.5),
            created_at: 1_700_000_000,
        })
        .expect("insert export stat");

    let resp = handle_request(JsonRpcRequest {
        id: 41,
        method: "stats/cost/export".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });

    assert_eq!(
        resp.result.get("fileName").and_then(|value| value.as_str()),
        Some("codexmanager-costs-custom.csv")
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("csv content");
    assert!(content.contains("section,dimension,dimensionValue"));
    assert!(content.contains("byKey,keyId,key-export"));
}

#[test]
fn stats_trends_rpc_returns_requests_models_and_heatmap() {
    let (_db_scope, storage) = setup_test_db("stats-trends");

    for (id, model, status_code, created_at) in [
        (1, "o3", 200, 1_700_000_000),
        (2, "o3", 500, 1_700_000_600),
        (3, "gpt-4o", 200, 1_700_086_400),
    ] {
        storage
            .insert_request_log(&codexmanager_core::storage::RequestLog {
                trace_id: Some(format!("trend-{id}")),
                key_id: Some("gk-trend".to_string()),
                account_id: Some("acc-trend".to_string()),
                initial_account_id: Some("acc-trend".to_string()),
                attempted_account_ids_json: Some(r#"["acc-trend"]"#.to_string()),
                route_strategy: Some("balanced".to_string()),
                requested_model: Some(model.to_string()),
                model_fallback_path_json: Some(format!(r#"["{model}"]"#)),
                request_path: "/v1/responses".to_string(),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some(model.to_string()),
                reasoning_effort: Some("medium".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
                status_code: Some(status_code),
                duration_ms: Some(120),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: if status_code >= 400 {
                    Some("upstream error".to_string())
                } else {
                    None
                },
                created_at,
            })
            .expect("insert request log");
    }

    let requests_resp = handle_request(JsonRpcRequest {
        id: 42,
        method: "stats/trends/requests".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
            "granularity": "day",
        })),
    });
    assert_eq!(
        requests_resp
            .result
            .get("granularity")
            .and_then(|value| value.as_str()),
        Some("day")
    );
    assert_eq!(
        requests_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(
        requests_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("successRate"))
            .and_then(|value| value.as_f64()),
        Some(50.0)
    );

    let models_resp = handle_request(JsonRpcRequest {
        id: 43,
        method: "stats/trends/models".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });
    assert_eq!(
        models_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("model"))
            .and_then(|value| value.as_str()),
        Some("o3")
    );

    let heatmap_resp = handle_request(JsonRpcRequest {
        id: 44,
        method: "stats/trends/heatmap".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });
    assert_eq!(
        heatmap_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
}

#[test]
fn requestlog_export_rpc_returns_filtered_csv_content() {
    let (_db_scope, storage) = setup_test_db("requestlog-export");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-1".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-2".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(502),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(10),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.05),
            error: Some("upstream error".to_string()),
            created_at: 1_700_000_100,
        })
        .expect("insert request log");

    let resp = handle_request(JsonRpcRequest {
        id: 42,
        method: "requestlog/export".to_string(),
        params: Some(serde_json::json!({
            "format": "csv",
            "query": "status:502",
            "statusFilter": "5xx",
        })),
    });

    assert_eq!(
        resp.result.get("fileName").and_then(|value| value.as_str()),
        Some("codexmanager-requestlogs-5xx.csv")
    );
    assert_eq!(
        resp.result
            .get("recordCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("csv content");
    assert!(content.contains("traceId,keyId,accountId"));
    assert!(content.contains("trc-export-2"));
    assert!(!content.contains("trc-export-1"));
}

#[test]
fn requestlog_export_rpc_supports_key_model_and_time_filters() {
    let (_db_scope, storage) = setup_test_db("requestlog-export-extended-filters");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-a".to_string()),
            key_id: Some("gk-export-a".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log a");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-b".to_string()),
            key_id: Some("gk-export-b".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("gpt-4o".to_string()),
            model_fallback_path_json: Some(r#"["gpt-4o"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-4o".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(6),
            total_tokens: Some(16),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.08),
            error: None,
            created_at: 1_700_000_200,
        })
        .expect("insert request log b");

    let resp = handle_request(JsonRpcRequest {
        id: 43,
        method: "requestlog/export".to_string(),
        params: Some(serde_json::json!({
            "format": "json",
            "keyIds": ["gk-export-a", "gk-export-b"],
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });

    assert_eq!(
        resp.result
            .get("recordCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("json content");
    assert!(content.contains("trc-export-b"));
    assert!(!content.contains("trc-export-a"));
}

#[test]
fn requestlog_list_and_summary_support_extended_filters() {
    let (_db_scope, storage) = setup_test_db("requestlog-list-summary-extended-filters");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-list-a".to_string()),
            key_id: Some("gk-list-a".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log a");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-list-b".to_string()),
            key_id: Some("gk-list-b".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("gpt-4o".to_string()),
            model_fallback_path_json: Some(r#"["gpt-4o"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-4o".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(502),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(10),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.05),
            error: Some("upstream error".to_string()),
            created_at: 1_700_000_200,
        })
        .expect("insert request log b");

    let list_resp = handle_request(JsonRpcRequest {
        id: 44,
        method: "requestlog/list".to_string(),
        params: Some(serde_json::json!({
            "page": 1,
            "pageSize": 20,
            "statusFilter": "5xx",
            "keyIds": ["gk-list-a", "gk-list-b"],
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });
    assert_eq!(
        list_resp
            .result
            .get("total")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        list_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let summary_resp = handle_request(JsonRpcRequest {
        id: 45,
        method: "requestlog/summary".to_string(),
        params: Some(serde_json::json!({
            "statusFilter": "5xx",
            "keyIds": ["gk-list-a", "gk-list-b"],
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });
    assert_eq!(
        summary_resp
            .result
            .get("totalCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        summary_resp
            .result
            .get("filteredCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        summary_resp
            .result
            .get("errorCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
}

#[test]
fn apikey_rpc_supports_model_fallback_get_and_set() {
    let (_db_scope, _storage) = setup_test_db("apikey-rpc-model-fallback");

    let create_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "降级测试",
            "modelSlug": "o3",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let set_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "apikey/modelFallback/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "modelChain": ["o3", "o4-mini", "gpt-4o"],
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "apikey/modelFallback/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        get_resp
            .result
            .get("modelChain")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            "o3".to_string(),
            "o4-mini".to_string(),
            "gpt-4o".to_string()
        ])
    );
}

#[test]
fn apikey_rpc_supports_allowed_models_get_and_set() {
    let (_db_scope, _storage) = setup_test_db("apikey-rpc-allowed-models");

    let create_resp = handle_request(JsonRpcRequest {
        id: 33,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "白名单测试",
            "modelSlug": "o3",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let set_resp = handle_request(JsonRpcRequest {
        id: 34,
        method: "apikey/allowedModels/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "allowedModels": ["o3", "o4-mini", "o3"],
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 35,
        method: "apikey/allowedModels/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        get_resp
            .result
            .get("allowedModels")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            }),
        Some(vec!["o3".to_string(), "o4-mini".to_string()])
    );
}

#[test]
fn alert_rpc_supports_rule_channel_history_and_channel_test() {
    let (_db_scope, _storage) = setup_test_db("alert-rpc");
    let _alert_guard = crate::alert_sender::alert_sender_test_guard();
    let (payload_tx, payload_rx) = std::sync::mpsc::channel::<String>();
    let webhook_url = format!("mock://alert-rpc-{}", now_ts());
    let _webhook_registration = RegisteredAlertWebhook::new(webhook_url.clone(), payload_tx);

    let upsert_channel_resp = handle_request(JsonRpcRequest {
        id: 40,
        method: "alert/channels/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "本地 Webhook",
            "type": "webhook",
            "enabled": true,
            "config": {
                "url": webhook_url,
            }
        })),
    });
    let channel_id = upsert_channel_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("channel id")
        .to_string();

    let upsert_rule_resp = handle_request(JsonRpcRequest {
        id: 41,
        method: "alert/rules/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "额度超过 90%",
            "type": "usage_threshold",
            "enabled": true,
            "config": {
                "thresholdPercent": 90,
                "channelIds": [channel_id.clone()],
                "cooldownSecs": 1800,
            }
        })),
    });
    assert_eq!(
        upsert_rule_resp
            .result
            .get("ruleType")
            .and_then(|value| value.as_str()),
        Some("usage_threshold")
    );

    let list_channels_resp = handle_request(JsonRpcRequest {
        id: 42,
        method: "alert/channels/list".to_string(),
        params: None,
    });
    assert_eq!(
        list_channels_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let list_rules_resp = handle_request(JsonRpcRequest {
        id: 43,
        method: "alert/rules/list".to_string(),
        params: None,
    });
    assert_eq!(
        list_rules_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let test_resp = handle_request(JsonRpcRequest {
        id: 44,
        method: "alert/channels/test".to_string(),
        params: Some(serde_json::json!({
            "id": channel_id,
        })),
    });
    assert_eq!(
        test_resp
            .result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("sent")
    );

    let webhook_payload = payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("receive webhook payload");
    assert!(webhook_payload.contains("CodexManager"));
    assert!(webhook_payload.contains("codexmanager.alert.test"));

    let history_resp = handle_request(JsonRpcRequest {
        id: 45,
        method: "alert/history/list".to_string(),
        params: Some(serde_json::json!({
            "limit": 10,
        })),
    });
    assert_eq!(
        history_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );
    assert_eq!(
        history_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("test_success")
    );
}

#[test]
fn plugin_rpc_supports_upsert_list_and_delete() {
    let (_db_scope, storage) = setup_test_db("plugin-rpc");

    let upsert_resp = handle_request(JsonRpcRequest {
        id: 46,
        method: "plugin/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "额度保护插件",
            "description": "阻止高风险请求",
            "runtime": "lua",
            "hookPoints": ["pre_route", "post_response", "pre_route"],
            "scriptContent": "return { allow = true }",
            "enabled": true,
            "timeoutMs": 250,
        })),
    });
    let plugin_id = upsert_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("plugin id")
        .to_string();
    assert_eq!(
        upsert_resp
            .result
            .get("runtime")
            .and_then(|value| value.as_str()),
        Some("lua")
    );
    assert_eq!(
        upsert_resp
            .result
            .get("hookPoints")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );

    let stored = storage
        .find_plugin_by_id(&plugin_id)
        .expect("find plugin")
        .expect("plugin exists");
    assert_eq!(stored.timeout_ms, 250);
    assert_eq!(stored.hook_points_json, r#"["pre_route","post_response"]"#);

    let list_resp = handle_request(JsonRpcRequest {
        id: 47,
        method: "plugin/list".to_string(),
        params: None,
    });
    assert_eq!(
        list_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let delete_resp = handle_request(JsonRpcRequest {
        id: 48,
        method: "plugin/delete".to_string(),
        params: Some(serde_json::json!({
            "id": plugin_id,
        })),
    });
    assert_eq!(
        delete_resp
            .result
            .get("ok")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let list_after_delete = handle_request(JsonRpcRequest {
        id: 49,
        method: "plugin/list".to_string(),
        params: None,
    });
    assert_eq!(
        list_after_delete
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(0)
    );
}

#[test]
fn plugin_rpc_rejects_invalid_runtime_and_hook_point() {
    let (_db_scope, _storage) = setup_test_db("plugin-rpc-invalid");

    let invalid_runtime_resp = handle_request(JsonRpcRequest {
        id: 50,
        method: "plugin/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "非法运行时插件",
            "runtime": "wasm",
            "hookPoints": ["pre_route"],
            "scriptContent": "return {}",
        })),
    });
    let invalid_runtime_error = invalid_runtime_resp
        .result
        .get("error")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    assert!(invalid_runtime_error.contains("unsupported plugin runtime"));

    let invalid_hook_point_resp = handle_request(JsonRpcRequest {
        id: 51,
        method: "plugin/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "非法钩子插件",
            "runtime": "lua",
            "hookPoints": ["before_route"],
            "scriptContent": "return {}",
        })),
    });
    let invalid_hook_point_error = invalid_hook_point_resp
        .result
        .get("error")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    assert!(invalid_hook_point_error.contains("unsupported plugin hook point"));
}

#[test]
fn alert_engine_usage_threshold_dedupes_and_recovers() {
    let (_db_scope, storage) = setup_test_db("alert-engine-usage-threshold");
    let _alert_guard = crate::alert_sender::alert_sender_test_guard();
    let (payload_tx, payload_rx) = std::sync::mpsc::channel::<String>();
    let webhook_url = format!("mock://alert-usage-threshold-{}", now_ts());
    let _webhook_registration = RegisteredAlertWebhook::new(webhook_url.clone(), payload_tx);

    let account = codexmanager_core::storage::Account {
        id: "acc-alert-1".to_string(),
        label: "主账号".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct-alert".to_string()),
        workspace_id: Some("org-alert".to_string()),
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");
    storage
        .insert_usage_snapshot(&codexmanager_core::storage::UsageSnapshotRecord {
            account_id: account.id.clone(),
            used_percent: Some(95.0),
            window_minutes: Some(60),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now_ts(),
        })
        .expect("insert usage snapshot");

    let channel_resp = handle_request(JsonRpcRequest {
        id: 60,
        method: "alert/channels/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "阈值 Webhook",
            "type": "webhook",
            "enabled": true,
            "config": { "url": webhook_url },
        })),
    });
    let channel_id = channel_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("channel id")
        .to_string();

    let _rule_resp = handle_request(JsonRpcRequest {
        id: 61,
        method: "alert/rules/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "额度超过 90%",
            "type": "usage_threshold",
            "enabled": true,
            "config": {
                "thresholdPercent": 90,
                "channelIds": [channel_id],
                "cooldownSecs": 1800,
            }
        })),
    });

    crate::alert_engine::run_alert_checks_once().expect("first alert check");
    assert!(payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("first webhook payload")
        .contains("thresholdPercent"));
    let first_history = storage.list_alert_history(10).expect("first history");
    assert_eq!(first_history.len(), 1);
    assert_eq!(first_history[0].status, "triggered");

    crate::alert_engine::run_alert_checks_once().expect("second alert check");
    let deduped_history = storage.list_alert_history(10).expect("deduped history");
    assert_eq!(
        deduped_history.len(),
        1,
        "duplicate trigger should be suppressed"
    );

    storage
        .insert_usage_snapshot(&codexmanager_core::storage::UsageSnapshotRecord {
            account_id: account.id.clone(),
            used_percent: Some(40.0),
            window_minutes: Some(60),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now_ts(),
        })
        .expect("insert recovered usage snapshot");
    crate::alert_engine::run_alert_checks_once().expect("recovery alert check");
    assert!(payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("recovery webhook payload")
        .contains("回落"));

    let final_history = storage.list_alert_history(10).expect("final history");
    assert_eq!(final_history.len(), 2);
    assert_eq!(final_history[0].status, "recovered");
}

#[test]
fn alert_engine_all_unavailable_triggers_and_recovers() {
    let (_db_scope, storage) = setup_test_db("alert-engine-all-unavailable");
    let _alert_guard = crate::alert_sender::alert_sender_test_guard();
    let (payload_tx, payload_rx) = std::sync::mpsc::channel::<String>();
    let webhook_url = format!("mock://alert-all-unavailable-{}", now_ts());
    let _webhook_registration = RegisteredAlertWebhook::new(webhook_url.clone(), payload_tx);

    let account = codexmanager_core::storage::Account {
        id: "acc-alert-2".to_string(),
        label: "备用账号".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct-alert-2".to_string()),
        workspace_id: Some("org-alert-2".to_string()),
        group_name: None,
        sort: 0,
        status: "unavailable".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    let channel_resp = handle_request(JsonRpcRequest {
        id: 62,
        method: "alert/channels/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "全部不可用 Webhook",
            "type": "webhook",
            "enabled": true,
            "config": { "url": webhook_url },
        })),
    });
    let channel_id = channel_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("channel id")
        .to_string();

    let _rule_resp = handle_request(JsonRpcRequest {
        id: 63,
        method: "alert/rules/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "全部账号不可用",
            "type": "all_unavailable",
            "enabled": true,
            "config": {
                "channelIds": [channel_id],
                "cooldownSecs": 600,
            }
        })),
    });

    crate::alert_engine::run_alert_checks_once().expect("all unavailable alert check");
    assert!(payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("all unavailable webhook payload")
        .contains("availableAccounts"));

    storage
        .update_account_status("acc-alert-2", "active")
        .expect("recover account");
    crate::alert_engine::run_alert_checks_once().expect("all unavailable recovery check");
    assert!(payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("all unavailable recovery payload")
        .contains("恢复"));

    let history = storage
        .list_alert_history(10)
        .expect("all unavailable history");
    assert!(history.iter().any(|item| item.status == "triggered"));
    assert!(history.iter().any(|item| item.status == "recovered"));
}

#[test]
fn healthcheck_run_triggers_token_refresh_fail_alerts_when_probe_fails() {
    struct HealthcheckAlertTestGuard {
        webhook_url: String,
    }

    impl Drop for HealthcheckAlertTestGuard {
        fn drop(&mut self) {
            crate::alert_sender::unregister_test_webhook(&self.webhook_url);
            crate::gateway::clear_probe_models_override_for_tests();
            crate::usage_refresh::clear_session_probe_state_for_tests();
        }
    }

    crate::usage_refresh::clear_session_probe_state_for_tests();
    let (_db_scope, storage) = setup_test_db("healthcheck-alert-integration");
    let _gateway_guard = crate::gateway::probe_models_test_guard();
    let _alert_guard = crate::alert_sender::alert_sender_test_guard();
    let (payload_tx, payload_rx) = std::sync::mpsc::channel::<String>();
    let webhook_url = format!("mock://healthcheck-alert-{}", now_ts());
    crate::alert_sender::register_test_webhook(&webhook_url, payload_tx);
    crate::gateway::set_probe_models_override_for_tests(Err(
        "usage endpoint status 401: token expired".to_string(),
    ));
    let _cleanup = HealthcheckAlertTestGuard {
        webhook_url: webhook_url.clone(),
    };

    let now = now_ts();
    storage
        .insert_account(&codexmanager_core::storage::Account {
            id: "acc-healthcheck-alert".to_string(),
            label: "巡检告警账号".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("acct-healthcheck-alert".to_string()),
            workspace_id: Some("org-healthcheck-alert".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-healthcheck-alert".to_string(),
            id_token: "header.payload.sig".to_string(),
            access_token: "healthcheck-token".to_string(),
            refresh_token: "healthcheck-refresh".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let channel_resp = handle_request(JsonRpcRequest {
        id: 64,
        method: "alert/channels/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "巡检异常 Webhook",
            "type": "webhook",
            "enabled": true,
            "config": { "url": webhook_url.clone() },
        })),
    });
    let channel_id = channel_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("channel id")
        .to_string();

    let _rule_resp = handle_request(JsonRpcRequest {
        id: 65,
        method: "alert/rules/upsert".to_string(),
        params: Some(serde_json::json!({
            "name": "巡检刷新失败",
            "type": "token_refresh_fail",
            "enabled": true,
            "config": {
                "threshold": 1,
                "windowMinutes": 60,
                "channelIds": [channel_id],
                "cooldownSecs": 600,
            }
        })),
    });

    let run_resp = handle_request(JsonRpcRequest {
        id: 66,
        method: "healthcheck/run".to_string(),
        params: None,
    });
    assert_eq!(
        run_resp
            .result
            .get("sampledAccounts")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        run_resp
            .result
            .get("failureCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );

    let alert_payload = payload_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("healthcheck alert webhook payload");
    assert!(alert_payload.contains("token_refresh_fail"));
    assert!(alert_payload.contains("巡检刷新失败"));

    let account = storage
        .find_account_by_id("acc-healthcheck-alert")
        .expect("find account")
        .expect("stored account");
    assert_eq!(account.status, "unavailable");

    let history = storage.list_alert_history(10).expect("alert history");
    assert!(history.iter().any(|item| item.status == "triggered"));
    assert!(history
        .iter()
        .any(|item| item.rule_name.as_deref() == Some("巡检刷新失败")));
}
