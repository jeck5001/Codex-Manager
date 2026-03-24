use super::{
    classify_usage_refresh_error, mark_usage_unreachable_if_needed,
    should_record_failure_event_with_state, usage_error_indicates_deactivated_account,
    FailureThrottleKey,
};
use codexmanager_core::storage::{now_ts, Account, Storage};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

static USAGE_REFRESH_ERROR_TEST_MUTEX: Mutex<()> = Mutex::new(());
static USAGE_REFRESH_ERROR_TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

fn usage_refresh_error_test_guard() -> MutexGuard<'static, ()> {
    USAGE_REFRESH_ERROR_TEST_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn new_usage_refresh_error_test_db_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}-{}-{}-{}.db",
        std::process::id(),
        now_ts(),
        USAGE_REFRESH_ERROR_TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    path
}

struct UsageRefreshErrorDbScope {
    previous_db_path: Option<String>,
    db_path: PathBuf,
}

impl Drop for UsageRefreshErrorDbScope {
    fn drop(&mut self) {
        match &self.previous_db_path {
            Some(value) => std::env::set_var("CODEXMANAGER_DB_PATH", value),
            None => std::env::remove_var("CODEXMANAGER_DB_PATH"),
        }
        crate::storage_helpers::clear_storage_cache_for_tests();
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_file(format!("{}-shm", self.db_path.display()));
        let _ = std::fs::remove_file(format!("{}-wal", self.db_path.display()));
    }
}

fn setup_usage_refresh_error_test_db(prefix: &str) -> UsageRefreshErrorDbScope {
    let db_path = new_usage_refresh_error_test_db_path(prefix);
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    crate::storage_helpers::clear_storage_cache_for_tests();
    let storage = Storage::open(&db_path).expect("open usage refresh error test db");
    storage.init().expect("init usage refresh error test db");
    UsageRefreshErrorDbScope {
        previous_db_path,
        db_path,
    }
}

fn insert_account(storage: &Storage, account_id: &str, status: &str) {
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: account_id.to_string(),
            label: account_id.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: status.to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
}

#[test]
fn usage_refresh_error_class_groups_by_status_code() {
    assert_eq!(
        classify_usage_refresh_error("usage endpoint status 500 Internal Server Error"),
        "usage_status_500"
    );
    assert_eq!(
        classify_usage_refresh_error("usage endpoint status 503 Service Unavailable"),
        "usage_status_503"
    );
}

#[test]
fn usage_refresh_error_class_catches_timeout_and_connection() {
    assert_eq!(
        classify_usage_refresh_error("request timeout while calling usage"),
        "timeout"
    );
    assert_eq!(
        classify_usage_refresh_error("connection reset by peer"),
        "connection"
    );
    assert_eq!(classify_usage_refresh_error("unknown error"), "other");
}

#[test]
fn usage_refresh_error_class_catches_deactivated_account() {
    let message = "HTTP 401: Your OpenAI account has been deactivated, please check your email for more information. If you feel this is an error, contact us through our help center at help.openai.com";
    assert!(usage_error_indicates_deactivated_account(message));
    assert_eq!(classify_usage_refresh_error(message), "account_deactivated");
}

#[test]
fn usage_refresh_error_class_catches_deactivated_workspace() {
    let message = "HTTP 403: This workspace has been deactivated and can no longer be used.";
    assert!(usage_error_indicates_deactivated_account(message));
    assert_eq!(
        classify_usage_refresh_error(message),
        "workspace_deactivated"
    );
}

#[test]
fn failure_event_throttle_dedupes_within_window() {
    let mut state = HashMap::new();
    let key = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "usage_status_500".to_string(),
    };

    assert!(should_record_failure_event_with_state(
        &mut state,
        key.clone(),
        100,
        60
    ));
    assert!(!should_record_failure_event_with_state(
        &mut state,
        key.clone(),
        120,
        60
    ));
    assert!(should_record_failure_event_with_state(
        &mut state, key, 161, 60
    ));
}

#[test]
fn failure_event_throttle_isolated_by_error_class() {
    let mut state = HashMap::new();
    let key_500 = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "usage_status_500".to_string(),
    };
    let key_timeout = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "timeout".to_string(),
    };

    assert!(should_record_failure_event_with_state(
        &mut state, key_500, 100, 60
    ));
    assert!(should_record_failure_event_with_state(
        &mut state,
        key_timeout,
        110,
        60
    ));
}

#[test]
fn mark_usage_unreachable_marks_rate_limited_cooldown_for_429() {
    let _guard = usage_refresh_error_test_guard();
    let _gateway_guard = crate::gateway::gateway_runtime_test_guard();
    crate::gateway::reload_runtime_config_from_env();
    let _db = setup_usage_refresh_error_test_db("usage-refresh-error-429");
    let storage = crate::storage_helpers::open_storage().expect("open storage");
    insert_account(&storage, "acc-429", "active");

    mark_usage_unreachable_if_needed(&storage, "acc-429", "usage endpoint status 429 Too Many Requests");

    let cooldowns = crate::gateway::list_account_cooldowns();
    let snapshot = cooldowns.get("acc-429").expect("429 cooldown snapshot");
    assert_eq!(snapshot.reason_code, "rate_limited");
    assert_eq!(snapshot.reason_label, "速率限制");
}

#[test]
fn mark_usage_unreachable_marks_upstream_5xx_cooldown_for_server_error() {
    let _guard = usage_refresh_error_test_guard();
    let _gateway_guard = crate::gateway::gateway_runtime_test_guard();
    crate::gateway::reload_runtime_config_from_env();
    let _db = setup_usage_refresh_error_test_db("usage-refresh-error-5xx");
    let storage = crate::storage_helpers::open_storage().expect("open storage");
    insert_account(&storage, "acc-5xx", "active");

    mark_usage_unreachable_if_needed(
        &storage,
        "acc-5xx",
        "usage endpoint status 503 Service Unavailable",
    );

    let cooldowns = crate::gateway::list_account_cooldowns();
    let snapshot = cooldowns.get("acc-5xx").expect("5xx cooldown snapshot");
    assert_eq!(snapshot.reason_code, "upstream_5xx");
    assert_eq!(snapshot.reason_label, "上游 5xx");
}

#[test]
fn mark_usage_unreachable_marks_network_cooldown_for_timeout() {
    let _guard = usage_refresh_error_test_guard();
    let _gateway_guard = crate::gateway::gateway_runtime_test_guard();
    crate::gateway::reload_runtime_config_from_env();
    let _db = setup_usage_refresh_error_test_db("usage-refresh-error-timeout");
    let storage = crate::storage_helpers::open_storage().expect("open storage");
    insert_account(&storage, "acc-timeout", "active");

    mark_usage_unreachable_if_needed(
        &storage,
        "acc-timeout",
        "request timeout while calling usage endpoint",
    );

    let cooldowns = crate::gateway::list_account_cooldowns();
    let snapshot = cooldowns.get("acc-timeout").expect("network cooldown snapshot");
    assert_eq!(snapshot.reason_code, "network");
    assert_eq!(snapshot.reason_label, "网络异常");
}
