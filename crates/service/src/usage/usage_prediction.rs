use std::collections::{HashMap, HashSet};

use codexmanager_core::rpc::types::UsagePredictionSummaryResult;
use codexmanager_core::storage::{Account, UsageSnapshotRecord};
use serde_json::Value;

use crate::app_settings::{
    current_gateway_quota_protection_enabled, current_gateway_quota_protection_threshold_percent,
};
use crate::storage_helpers::open_storage;

const MINUTES_PER_HOUR: i64 = 60;
const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
const ROUNDING_BIAS: i64 = 3;

#[derive(Debug, Default)]
struct WindowPredictionAggregate {
    margin_to_threshold_total: f64,
    remaining_total: f64,
    rate_total_per_hour: f64,
    signal_count: i64,
}

impl WindowPredictionAggregate {
    fn observe(&mut self, remaining_percent: f64, rate_per_hour: f64, threshold_percent: f64) {
        self.signal_count += 1;
        self.margin_to_threshold_total += (remaining_percent - threshold_percent).max(0.0);
        self.remaining_total += remaining_percent.max(0.0);
        if rate_per_hour.is_finite() && rate_per_hour > 0.0 {
            self.rate_total_per_hour += rate_per_hour;
        }
    }

    fn estimated_hours_to_threshold(&self) -> Option<f64> {
        if self.signal_count <= 0 {
            return None;
        }
        if self.margin_to_threshold_total <= 0.0 {
            return Some(0.0);
        }
        if self.rate_total_per_hour <= 0.0 {
            return None;
        }
        Some(self.margin_to_threshold_total / self.rate_total_per_hour)
    }

    fn estimated_hours_to_exhaustion(&self) -> Option<f64> {
        if self.signal_count <= 0 {
            return None;
        }
        if self.remaining_total <= 0.0 {
            return Some(0.0);
        }
        if self.rate_total_per_hour <= 0.0 {
            return None;
        }
        Some(self.remaining_total / self.rate_total_per_hour)
    }
}

pub(crate) fn read_usage_prediction_summary() -> Result<UsagePredictionSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts failed: {err}"))?;
    let usage_items = storage
        .latest_usage_snapshots_by_account()
        .map_err(|err| format!("list usage snapshots failed: {err}"))?;
    let token_account_ids = storage
        .list_tokens()
        .map_err(|err| format!("list tokens failed: {err}"))?
        .into_iter()
        .map(|token| token.account_id)
        .collect::<HashSet<_>>();

    Ok(compute_usage_prediction_summary(
        &accounts,
        &usage_items,
        &token_account_ids,
        current_gateway_quota_protection_enabled(),
        current_gateway_quota_protection_threshold_percent(),
    ))
}

pub(crate) fn compute_usage_prediction_summary(
    accounts: &[Account],
    usage_items: &[UsageSnapshotRecord],
    token_account_ids: &HashSet<String>,
    quota_protection_enabled: bool,
    quota_protection_threshold_percent: u64,
) -> UsagePredictionSummaryResult {
    let usage_map = usage_items
        .iter()
        .map(|item| (item.account_id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let threshold_percent = quota_protection_threshold_percent.min(100) as f64;

    let mut primary = WindowPredictionAggregate::default();
    let mut secondary = WindowPredictionAggregate::default();
    let mut ready_account_count = 0_i64;

    for account in accounts.iter().filter(|account| {
        account_status_is_routable(account.status.as_str())
            && token_account_ids.contains(&account.id)
    }) {
        let Some(snapshot) = usage_map.get(account.id.as_str()).copied() else {
            continue;
        };

        if snapshot_meets_ready_threshold(snapshot, quota_protection_threshold_percent) {
            ready_account_count += 1;
        }

        if primary_belongs_to_secondary(snapshot) {
            observe_window(
                &mut secondary,
                snapshot.used_percent,
                snapshot.window_minutes,
                threshold_percent,
            );
        } else {
            observe_window(
                &mut primary,
                snapshot.used_percent,
                snapshot.window_minutes,
                threshold_percent,
            );
        }

        observe_window(
            &mut secondary,
            snapshot.secondary_used_percent,
            snapshot.secondary_window_minutes,
            threshold_percent,
        );
    }

    let (estimated_hours_to_threshold, threshold_limited_by) = pick_limiting_window(
        primary.estimated_hours_to_threshold(),
        secondary.estimated_hours_to_threshold(),
    );
    let (estimated_hours_to_pool_exhaustion, pool_limited_by) = pick_limiting_window(
        primary.estimated_hours_to_exhaustion(),
        secondary.estimated_hours_to_exhaustion(),
    );

    UsagePredictionSummaryResult {
        quota_protection_enabled,
        quota_protection_threshold_percent: threshold_percent.round() as i64,
        ready_account_count,
        estimated_hours_to_threshold,
        estimated_hours_to_pool_exhaustion,
        threshold_limited_by,
        pool_limited_by,
    }
}

fn observe_window(
    aggregate: &mut WindowPredictionAggregate,
    used_percent: Option<f64>,
    window_minutes: Option<i64>,
    threshold_percent: f64,
) {
    let Some(used_percent) = normalize_percent(used_percent) else {
        return;
    };
    let Some(window_minutes) = window_minutes.filter(|value| *value > 0) else {
        return;
    };

    let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);
    let window_hours = window_minutes as f64 / MINUTES_PER_HOUR as f64;
    if window_hours <= 0.0 {
        return;
    }
    let rate_per_hour = used_percent / window_hours;
    aggregate.observe(remaining_percent, rate_per_hour, threshold_percent);
}

fn pick_limiting_window(
    primary_hours: Option<f64>,
    secondary_hours: Option<f64>,
) -> (Option<f64>, Option<String>) {
    match (primary_hours, secondary_hours) {
        (Some(primary), Some(secondary)) if primary <= secondary => {
            (Some(primary), Some("primary".to_string()))
        }
        (Some(_), Some(secondary)) => (Some(secondary), Some("secondary".to_string())),
        (Some(primary), None) => (Some(primary), Some("primary".to_string())),
        (None, Some(secondary)) => (Some(secondary), Some("secondary".to_string())),
        (None, None) => (None, None),
    }
}

fn normalize_percent(value: Option<f64>) -> Option<f64> {
    value.map(|parsed| parsed.clamp(0.0, 100.0))
}

fn account_status_is_routable(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "inactive" | "disabled" | "unavailable" | "deactivated"
    )
}

fn snapshot_meets_ready_threshold(
    snapshot: &UsageSnapshotRecord,
    remain_percent_threshold: u64,
) -> bool {
    let threshold = remain_percent_threshold as f64;
    let Some(primary_used) = snapshot.used_percent else {
        return false;
    };
    if snapshot.window_minutes.is_none() || remaining_percent(primary_used) < threshold {
        return false;
    }

    match (
        snapshot.secondary_used_percent,
        snapshot.secondary_window_minutes,
    ) {
        (None, None) => true,
        (Some(secondary_used), Some(_)) => remaining_percent(secondary_used) >= threshold,
        _ => false,
    }
}

fn remaining_percent(used_percent: f64) -> f64 {
    (100.0 - used_percent).clamp(0.0, 100.0)
}

fn primary_belongs_to_secondary(snapshot: &UsageSnapshotRecord) -> bool {
    let has_secondary_signal =
        snapshot.secondary_used_percent.is_some() || snapshot.secondary_window_minutes.is_some();
    !has_secondary_signal
        && (is_long_window(snapshot.window_minutes)
            || is_free_plan_usage(snapshot.credits_json.as_deref()))
}

fn is_long_window(window_minutes: Option<i64>) -> bool {
    window_minutes.is_some_and(|value| value > MINUTES_PER_DAY + ROUNDING_BIAS)
}

fn is_free_plan_usage(raw: Option<&str>) -> bool {
    let Some(value) = parse_credits(raw) else {
        return false;
    };
    extract_plan_type_recursive(&value)
        .map(|value| value.contains("free"))
        .unwrap_or(false)
}

fn parse_credits(raw: Option<&str>) -> Option<Value> {
    let text = raw?.trim();
    if text.is_empty() {
        return None;
    }
    serde_json::from_str(text).ok()
}

fn extract_plan_type_recursive(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => items.iter().find_map(extract_plan_type_recursive),
        Value::Object(map) => {
            for key in [
                "plan_type",
                "planType",
                "subscription_tier",
                "subscriptionTier",
                "tier",
                "account_type",
                "accountType",
                "type",
            ] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    let normalized = text.trim().to_ascii_lowercase();
                    if !normalized.is_empty() {
                        return Some(normalized);
                    }
                }
            }
            map.values().find_map(extract_plan_type_recursive)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_usage_prediction_summary;
    use codexmanager_core::storage::{now_ts, Account, UsageSnapshotRecord};
    use std::collections::HashSet;

    fn account(id: &str, status: &str) -> Account {
        Account {
            id: id.to_string(),
            label: id.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: status.to_string(),
            created_at: now_ts(),
            updated_at: now_ts(),
        }
    }

    fn snapshot(
        account_id: &str,
        used_percent: Option<f64>,
        window_minutes: Option<i64>,
        secondary_used_percent: Option<f64>,
        secondary_window_minutes: Option<i64>,
        credits_json: Option<&str>,
    ) -> UsageSnapshotRecord {
        UsageSnapshotRecord {
            account_id: account_id.to_string(),
            used_percent,
            window_minutes,
            resets_at: None,
            secondary_used_percent,
            secondary_window_minutes,
            secondary_resets_at: None,
            credits_json: credits_json.map(|value| value.to_string()),
            captured_at: now_ts(),
        }
    }

    #[test]
    fn prediction_summary_estimates_threshold_and_pool_exhaustion() {
        let accounts = vec![account("acc-1", "active")];
        let usage_items = vec![snapshot("acc-1", Some(60.0), Some(300), None, None, None)];
        let token_account_ids = ["acc-1".to_string()].into_iter().collect::<HashSet<_>>();

        let result =
            compute_usage_prediction_summary(&accounts, &usage_items, &token_account_ids, true, 20);

        let threshold_hours = result
            .estimated_hours_to_threshold
            .expect("estimated hours to threshold");
        let pool_hours = result
            .estimated_hours_to_pool_exhaustion
            .expect("estimated hours to pool exhaustion");

        assert_eq!(result.ready_account_count, 1);
        assert_eq!(result.threshold_limited_by.as_deref(), Some("primary"));
        assert_eq!(result.pool_limited_by.as_deref(), Some("primary"));
        assert!((threshold_hours - 1.666_666).abs() < 0.01);
        assert!((pool_hours - 3.333_333).abs() < 0.01);
    }

    #[test]
    fn prediction_summary_routes_free_single_window_accounts_to_secondary() {
        let accounts = vec![
            account("free-1", "active"),
            account("disabled-1", "disabled"),
        ];
        let usage_items = vec![
            snapshot(
                "free-1",
                Some(30.0),
                Some(10_080),
                None,
                None,
                Some(r#"{"planType":"free"}"#),
            ),
            snapshot("disabled-1", Some(10.0), Some(300), None, None, None),
        ];
        let token_account_ids = ["free-1".to_string(), "disabled-1".to_string()]
            .into_iter()
            .collect::<HashSet<_>>();

        let result =
            compute_usage_prediction_summary(&accounts, &usage_items, &token_account_ids, true, 20);

        assert_eq!(result.ready_account_count, 1);
        assert_eq!(result.threshold_limited_by.as_deref(), Some("secondary"));
        assert_eq!(result.pool_limited_by.as_deref(), Some("secondary"));
        assert!(result.estimated_hours_to_threshold.is_some());
    }
}
