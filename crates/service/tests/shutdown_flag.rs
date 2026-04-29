/// 函数 `shutdown_flag_can_toggle`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn shutdown_flag_can_toggle() {
    assert!(!codexmanager_service::shutdown_requested());
    codexmanager_service::request_shutdown("localhost:0");
    assert!(codexmanager_service::shutdown_requested());
    codexmanager_service::clear_shutdown_flag();
    assert!(!codexmanager_service::shutdown_requested());
}
