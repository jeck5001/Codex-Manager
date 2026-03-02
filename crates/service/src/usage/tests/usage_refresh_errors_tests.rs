use super::{
    classify_usage_refresh_error, should_record_failure_event_with_state, FailureThrottleKey,
};
use std::collections::HashMap;

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
