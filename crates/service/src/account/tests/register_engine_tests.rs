use super::{run_local_register_flow_for_test, RegisterEngineTestScenario};

#[test]
fn register_engine_runs_generator_email_flow_to_importable_result() {
    let result =
        run_local_register_flow_for_test(RegisterEngineTestScenario::success()).expect("register flow");

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
