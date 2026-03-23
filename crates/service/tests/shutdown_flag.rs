#[test]
fn shutdown_flag_can_toggle() {
    assert!(!codexmanager_service::shutdown_requested());
    codexmanager_service::request_shutdown("localhost:0");
    assert!(codexmanager_service::shutdown_requested());
    codexmanager_service::clear_shutdown_flag();
    assert!(!codexmanager_service::shutdown_requested());
}
