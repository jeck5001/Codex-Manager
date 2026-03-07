use codexmanager_core::storage::{now_ts, Storage};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn unique_temp_db_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("codexmanager-app-settings-test-{unique}.db"))
}

fn reset_runtime_defaults() {
    let _ = codexmanager_service::set_service_bind_mode(
        codexmanager_service::SERVICE_BIND_MODE_LOOPBACK,
    );
    let _ = codexmanager_service::app_settings_set(Some(&json!({
        "routeStrategy": "ordered",
        "cpaNoCookieHeaderModeEnabled": false,
        "upstreamProxyUrl": "",
        "envOverrides": {},
        "backgroundTasks": {
            "usagePollingEnabled": true,
            "usagePollIntervalSecs": 600,
            "gatewayKeepaliveEnabled": true,
            "gatewayKeepaliveIntervalSecs": 180,
            "tokenRefreshPollingEnabled": true,
            "tokenRefreshPollIntervalSecs": 60,
            "usageRefreshWorkers": 4,
            "httpWorkerFactor": 4,
            "httpWorkerMin": 8,
            "httpStreamWorkerFactor": 1,
            "httpStreamWorkerMin": 2
        }
    })));
}

fn with_temp_db(test: impl FnOnce(&PathBuf)) {
    let _guard = env_lock().lock().expect("env lock");
    let db_path = unique_temp_db_path();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var("CODEXMANAGER_DB_PATH", &db_path);
    codexmanager_service::initialize_storage_if_needed().expect("init storage");
    reset_runtime_defaults();

    test(&db_path);

    reset_runtime_defaults();
    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn app_settings_set_persists_snapshot_and_password_hash() {
    with_temp_db(|db_path| {
        let snapshot = codexmanager_service::app_settings_set(Some(&json!({
            "updateAutoCheck": false,
            "closeToTrayOnClose": true,
            "lowTransparency": true,
            "theme": "dark",
            "serviceAddr": "127.0.0.1:4999",
            "serviceListenMode": "all_interfaces",
            "routeStrategy": "rr",
            "cpaNoCookieHeaderModeEnabled": true,
            "upstreamProxyUrl": "http://127.0.0.1:7890",
            "backgroundTasks": {
                "usagePollingEnabled": false,
                "usagePollIntervalSecs": 900,
                "gatewayKeepaliveEnabled": false,
                "gatewayKeepaliveIntervalSecs": 240,
                "tokenRefreshPollingEnabled": true,
                "tokenRefreshPollIntervalSecs": 120,
                "usageRefreshWorkers": 6,
                "httpWorkerFactor": 5,
                "httpWorkerMin": 9,
                "httpStreamWorkerFactor": 2,
                "httpStreamWorkerMin": 3
            },
            "webAccessPassword": "secret-pass"
        })))
        .expect("save app settings");

        assert_eq!(
            snapshot
                .get("updateAutoCheck")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            snapshot
                .get("closeToTrayOnClose")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            snapshot.get("theme").and_then(|value| value.as_str()),
            Some("dark")
        );
        assert_eq!(
            snapshot
                .get("serviceListenMode")
                .and_then(|value| value.as_str()),
            Some(codexmanager_service::SERVICE_BIND_MODE_ALL_INTERFACES)
        );
        assert_eq!(
            snapshot
                .get("routeStrategy")
                .and_then(|value| value.as_str()),
            Some("balanced")
        );
        assert_eq!(
            snapshot
                .get("webAccessPasswordConfigured")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(codexmanager_service::verify_web_access_password(
            "secret-pass"
        ));

        let storage = Storage::open(db_path).expect("open storage");
        let stored_password = storage
            .get_app_setting(codexmanager_service::APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY)
            .expect("read password hash");
        assert!(stored_password
            .as_deref()
            .is_some_and(|value| value.starts_with("sha256$")));
    });
}

#[test]
fn sync_runtime_settings_from_storage_applies_saved_runtime_values() {
    with_temp_db(|db_path| {
        let storage = Storage::open(db_path).expect("open storage");
        storage
            .set_app_setting(
                codexmanager_service::APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY,
                "balanced",
                now_ts(),
            )
            .expect("save route strategy");
        storage
            .set_app_setting(
                codexmanager_service::APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
                "1",
                now_ts(),
            )
            .expect("save cpa mode");
        storage
            .set_app_setting(
                codexmanager_service::APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
                "http://127.0.0.1:8899",
                now_ts(),
            )
            .expect("save upstream proxy");
        storage
            .set_app_setting(
                codexmanager_service::APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
                &serde_json::to_string(&json!({
                    "usagePollingEnabled": false,
                    "usagePollIntervalSecs": 777,
                    "gatewayKeepaliveEnabled": true,
                    "gatewayKeepaliveIntervalSecs": 180,
                    "tokenRefreshPollingEnabled": true,
                    "tokenRefreshPollIntervalSecs": 60,
                    "usageRefreshWorkers": 4,
                    "httpWorkerFactor": 4,
                    "httpWorkerMin": 8,
                    "httpStreamWorkerFactor": 1,
                    "httpStreamWorkerMin": 2
                }))
                .expect("serialize background tasks"),
                now_ts(),
            )
            .expect("save background tasks");
        storage
            .set_app_setting(
                codexmanager_service::APP_SETTING_ENV_OVERRIDES_KEY,
                &serde_json::to_string(&json!({
                    "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS": "654321"
                }))
                .expect("serialize env overrides"),
                now_ts(),
            )
            .expect("save env overrides");
        drop(storage);

        codexmanager_service::sync_runtime_settings_from_storage();

        let snapshot =
            codexmanager_service::app_settings_get().expect("get app settings after sync");
        assert_eq!(
            snapshot
                .get("routeStrategy")
                .and_then(|value| value.as_str()),
            Some("balanced")
        );
        assert_eq!(
            snapshot
                .get("cpaNoCookieHeaderModeEnabled")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            snapshot
                .get("upstreamProxyUrl")
                .and_then(|value| value.as_str()),
            Some("http://127.0.0.1:8899")
        );
        assert_eq!(
            snapshot
                .get("backgroundTasks")
                .and_then(|value| value.get("usagePollingEnabled"))
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            snapshot
                .get("backgroundTasks")
                .and_then(|value| value.get("usagePollIntervalSecs"))
                .and_then(|value| value.as_u64()),
            Some(777)
        );
        assert_eq!(
            snapshot
                .get("envOverrides")
                .and_then(|value| value.get("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS"))
                .and_then(|value| value.as_str()),
            Some("654321")
        );
        assert_eq!(
            std::env::var("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS")
                .ok()
                .as_deref(),
            Some("654321")
        );
    });
}

#[test]
fn app_settings_set_persists_env_overrides_and_exposes_catalog() {
    with_temp_db(|db_path| {
        let snapshot = codexmanager_service::app_settings_set(Some(&json!({
            "envOverrides": {
                "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS": "321000",
                "CODEXMANAGER_UPSTREAM_BASE_URL": "https://chatgpt.com"
            }
        })))
        .expect("save env overrides");

        assert_eq!(
            snapshot
                .get("envOverrides")
                .and_then(|value| value.get("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS"))
                .and_then(|value| value.as_str()),
            Some("321000")
        );
        assert_eq!(
            std::env::var("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS")
                .ok()
                .as_deref(),
            Some("321000")
        );
        assert_eq!(
            std::env::var("CODEXMANAGER_UPSTREAM_BASE_URL")
                .ok()
                .as_deref(),
            Some("https://chatgpt.com")
        );
        assert!(snapshot
            .get("envOverrideCatalog")
            .and_then(|value| value.as_array())
            .is_some_and(|items| !items.is_empty()));
        assert!(snapshot
            .get("envOverrideReservedKeys")
            .and_then(|value| value.as_array())
            .is_some_and(|items| items
                .iter()
                .any(|item| item.as_str() == Some("CODEXMANAGER_ROUTE_STRATEGY"))));

        let storage = Storage::open(db_path).expect("open storage");
        let stored = storage
            .get_app_setting(codexmanager_service::APP_SETTING_ENV_OVERRIDES_KEY)
            .expect("read env overrides")
            .expect("env overrides exists");
        assert!(stored.contains("CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS"));
    });
}

#[test]
fn app_settings_set_rejects_reserved_and_bootstrap_env_override_keys() {
    with_temp_db(|_| {
        let reserved = codexmanager_service::app_settings_set(Some(&json!({
            "envOverrides": {
                "CODEXMANAGER_ROUTE_STRATEGY": "balanced"
            }
        })));
        assert!(reserved.is_err());

        let bootstrap = codexmanager_service::app_settings_set(Some(&json!({
            "envOverrides": {
                "CODEXMANAGER_DB_PATH": "D:/tmp/other.db"
            }
        })));
        assert!(bootstrap.is_err());
    });
}
