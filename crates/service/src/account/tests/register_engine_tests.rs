use super::{
    build_sentinel_header_for_test, normalize_register_proxy_for_test,
    should_use_email_proxy_for_test,
    run_local_register_flow_for_test, RegisterEngineTestScenario,
};
use serde_json::Value;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

fn email_proxy_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvRestore(Option<OsString>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            std::env::set_var("CODEXMANAGER_REGISTER_USE_PROXY_FOR_EMAIL", value);
        } else {
            std::env::remove_var("CODEXMANAGER_REGISTER_USE_PROXY_FOR_EMAIL");
        }
    }
}

fn override_email_proxy_env(value: Option<&str>) -> EnvRestore {
    let previous = std::env::var_os("CODEXMANAGER_REGISTER_USE_PROXY_FOR_EMAIL");
    if let Some(value) = value {
        std::env::set_var("CODEXMANAGER_REGISTER_USE_PROXY_FOR_EMAIL", value);
    } else {
        std::env::remove_var("CODEXMANAGER_REGISTER_USE_PROXY_FOR_EMAIL");
    }
    EnvRestore(previous)
}

#[test]
fn register_engine_runs_generator_email_flow_to_importable_result() {
    let result = run_local_register_flow_for_test(RegisterEngineTestScenario::success())
        .expect("register flow");

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.email.as_deref(), Some("alpha123@generator.email"));
    assert!(result.payload.contains("\"refresh_token\""));
}

#[test]
fn register_engine_marks_otp_timeout_when_code_never_arrives() {
    let err = run_local_register_flow_for_test(RegisterEngineTestScenario::otp_timeout())
        .expect_err("otp timeout");

    assert!(err.contains("otp_timeout"));
}

#[test]
fn register_engine_normalizes_bare_proxy_url() {
    assert_eq!(
        normalize_register_proxy_for_test(Some("192.168.5.35:7890")),
        Some("http://192.168.5.35:7890".to_string())
    );
}

#[test]
fn register_engine_builds_sentinel_header_payload() {
    let header =
        build_sentinel_header_for_test("sentinel-token", "did-123", "username_password_create")
            .expect("header");
    let payload: Value = serde_json::from_str(header.as_str()).expect("sentinel header json");

    assert_eq!(
        payload.get("c").and_then(Value::as_str),
        Some("sentinel-token")
    );
    assert_eq!(payload.get("id").and_then(Value::as_str), Some("did-123"));
    assert_eq!(
        payload.get("flow").and_then(Value::as_str),
        Some("username_password_create")
    );
}

#[test]
fn register_engine_defaults_email_provider_to_direct_connection() {
    let _guard = email_proxy_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _restore = override_email_proxy_env(None);

    assert!(!should_use_email_proxy_for_test());
}

#[test]
fn register_engine_can_enable_email_provider_proxy_via_env() {
    let _guard = email_proxy_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _restore = override_email_proxy_env(Some("true"));

    assert!(should_use_email_proxy_for_test());
}
