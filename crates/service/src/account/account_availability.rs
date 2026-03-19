use codexmanager_core::storage::UsageSnapshotRecord;

pub(crate) enum Availability {
    Available,
    Unavailable(&'static str),
}

pub(crate) const ENV_GATEWAY_QUOTA_PROTECTION_ENABLED: &str =
    "CODEXMANAGER_GATEWAY_QUOTA_PROTECTION_ENABLED";
pub(crate) const ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT: &str =
    "CODEXMANAGER_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT";
const DEFAULT_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT: u64 = 10;

pub(crate) fn current_quota_protection_enabled() -> bool {
    matches!(
        std::env::var(ENV_GATEWAY_QUOTA_PROTECTION_ENABLED)
            .ok()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}

pub(crate) fn current_quota_protection_threshold_percent() -> u64 {
    std::env::var(ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|value| value.min(100))
        .unwrap_or(DEFAULT_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT)
}

fn quota_protection_cutoff_used_percent() -> f64 {
    let threshold = current_quota_protection_threshold_percent() as f64;
    (100.0 - threshold).max(0.0)
}

pub(crate) fn usage_window_is_unavailable(value: f64) -> bool {
    if value >= 100.0 {
        return true;
    }
    current_quota_protection_enabled() && value >= quota_protection_cutoff_used_percent()
}

pub(crate) fn evaluate_snapshot(snap: &UsageSnapshotRecord) -> Availability {
    let primary_missing = snap.used_percent.is_none() || snap.window_minutes.is_none();
    let secondary_present =
        snap.secondary_used_percent.is_some() || snap.secondary_window_minutes.is_some();
    let secondary_missing =
        snap.secondary_used_percent.is_none() || snap.secondary_window_minutes.is_none();
    if primary_missing {
        return Availability::Unavailable("usage_missing_primary");
    }
    // 兼容仅返回单窗口额度的账号（如免费周额度）：secondary 完全缺失时视为可用。
    // 但只要 secondary 已出现部分字段，仍要求字段完整，避免异常数据误判可用。
    if secondary_present && secondary_missing {
        return Availability::Unavailable("usage_missing_secondary");
    }
    if let Some(value) = snap.used_percent {
        if usage_window_is_unavailable(value) {
            return Availability::Unavailable(if value >= 100.0 {
                "usage_exhausted_primary"
            } else {
                "usage_protected_primary"
            });
        }
    }
    if let Some(value) = snap.secondary_used_percent {
        if usage_window_is_unavailable(value) {
            return Availability::Unavailable(if value >= 100.0 {
                "usage_exhausted_secondary"
            } else {
                "usage_protected_secondary"
            });
        }
    }
    Availability::Available
}

#[cfg(test)]
#[path = "tests/account_availability_tests.rs"]
mod tests;
