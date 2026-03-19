use super::{evaluate_snapshot, Availability};
use codexmanager_core::storage::UsageSnapshotRecord;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}

fn override_quota_protection_env(enabled: &str, threshold: &str) -> EnvRestore {
    let keys = [
        super::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
        super::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
    ];
    let previous = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    std::env::set_var(super::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED, enabled);
    std::env::set_var(
        super::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        threshold,
    );
    EnvRestore(previous)
}

fn snap(
    primary_used: Option<f64>,
    primary_window: Option<i64>,
    secondary_used: Option<f64>,
    secondary_window: Option<i64>,
) -> UsageSnapshotRecord {
    UsageSnapshotRecord {
        account_id: "acc-1".to_string(),
        used_percent: primary_used,
        window_minutes: primary_window,
        resets_at: None,
        secondary_used_percent: secondary_used,
        secondary_window_minutes: secondary_window,
        secondary_resets_at: None,
        credits_json: None,
        captured_at: 0,
    }
}

#[test]
fn availability_marks_missing_primary_unavailable() {
    let record = snap(None, Some(300), Some(10.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable(_)
    ));
}

#[test]
fn availability_marks_missing_secondary_available_when_both_secondary_fields_absent() {
    let record = snap(Some(10.0), Some(300), None, None);
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Available
    ));
}

#[test]
fn availability_marks_partial_secondary_missing_unavailable() {
    let record = snap(Some(10.0), Some(300), None, Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable(_)
    ));
}

#[test]
fn availability_marks_exhausted_secondary_unavailable() {
    let record = snap(Some(10.0), Some(300), Some(100.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable(_)
    ));
}

#[test]
fn availability_marks_ok_available() {
    let record = snap(Some(10.0), Some(300), Some(20.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Available
    ));
}

#[test]
fn availability_marks_primary_threshold_unavailable_when_quota_protection_enabled() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env = override_quota_protection_env("1", "10");

    let record = snap(Some(90.0), Some(300), Some(20.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable("usage_protected_primary")
    ));
}

#[test]
fn availability_marks_secondary_threshold_unavailable_when_quota_protection_enabled() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env = override_quota_protection_env("1", "5");

    let record = snap(Some(40.0), Some(300), Some(95.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable("usage_protected_secondary")
    ));
}
