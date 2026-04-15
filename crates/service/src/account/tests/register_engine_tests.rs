use super::{
    build_sentinel_header_for_test, normalize_register_proxy_for_test,
    run_local_register_flow_for_test, RegisterEngineTestScenario,
};
use serde_json::Value;

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
