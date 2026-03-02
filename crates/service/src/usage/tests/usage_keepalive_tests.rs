use super::is_keepalive_error_ignorable;

#[test]
fn keepalive_ignores_expected_idle_errors() {
    assert!(is_keepalive_error_ignorable("no available account"));
    assert!(is_keepalive_error_ignorable("storage unavailable"));
    assert!(!is_keepalive_error_ignorable("upstream timeout"));
}
