#[test]
fn shutdown_flag_can_toggle() {
    assert_eq!(gpttools_service::shutdown_requested(), false);
    gpttools_service::request_shutdown("localhost:0");
    assert_eq!(gpttools_service::shutdown_requested(), true);
    gpttools_service::clear_shutdown_flag();
    assert_eq!(gpttools_service::shutdown_requested(), false);
}
