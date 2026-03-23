use std::collections::HashMap;

use codexmanager_core::{
    rpc::types::{
        AccountListParams, DashboardAccountStatusBucket, DashboardGatewayMetricsResult,
        DashboardHealthResult, RequestLogSummary, UsageSnapshotResult,
    },
    storage::now_ts,
};

use crate::{account_list, requestlog_list, usage_list};

const HEALTH_WINDOW_MINUTES: i64 = 5;
const HEALTH_WINDOW_SECS: i64 = HEALTH_WINDOW_MINUTES * 60;
const DASHBOARD_REQUEST_LOG_LIMIT: i64 = 20_000;

pub(crate) fn read_dashboard_health() -> Result<DashboardHealthResult, String> {
    let generated_at = now_ts();
    let accounts = account_list::read_accounts(AccountListParams::default(), false)?.items;
    let usage_items = usage_list::read_usage_snapshots()?;
    let request_logs = requestlog_list::read_request_logs(None, Some(DASHBOARD_REQUEST_LOG_LIMIT))?;
    let recent_latency_samples =
        crate::gateway::recent_gateway_latency_samples(generated_at - HEALTH_WINDOW_SECS);

    Ok(DashboardHealthResult {
        generated_at,
        account_status_buckets: build_account_status_buckets(&accounts, &usage_items, generated_at),
        gateway_metrics: build_gateway_metrics(
            &request_logs,
            &recent_latency_samples,
            generated_at,
        ),
        recent_healthcheck: crate::usage_refresh::last_session_probe_result(),
    })
}

fn build_account_status_buckets(
    accounts: &[codexmanager_core::rpc::types::AccountSummary],
    usage_items: &[UsageSnapshotResult],
    now_ts: i64,
) -> Vec<DashboardAccountStatusBucket> {
    let usage_map = usage_items
        .iter()
        .filter_map(|item| {
            item.account_id
                .as_deref()
                .map(|account_id| (account_id, item))
        })
        .collect::<HashMap<_, _>>();

    let mut counts = HashMap::<&'static str, i64>::from([
        ("online", 0),
        ("cooldown", 0),
        ("unavailable", 0),
        ("disabled", 0),
        ("quota_exhausted", 0),
    ]);

    for account in accounts {
        let status_key =
            classify_account_status(account, usage_map.get(account.id.as_str()).copied(), now_ts);
        *counts.entry(status_key).or_insert(0) += 1;
    }

    let total = accounts.len() as i64;
    [
        ("online", "在线"),
        ("cooldown", "冷却中"),
        ("unavailable", "不可用"),
        ("disabled", "已禁用"),
        ("quota_exhausted", "额度耗尽"),
    ]
    .into_iter()
    .map(|(key, label)| {
        let count = counts.get(key).copied().unwrap_or(0);
        DashboardAccountStatusBucket {
            key: key.to_string(),
            label: label.to_string(),
            count,
            percent: percentage(count, total),
        }
    })
    .collect()
}

fn classify_account_status(
    account: &codexmanager_core::rpc::types::AccountSummary,
    usage: Option<&UsageSnapshotResult>,
    now_ts: i64,
) -> &'static str {
    let status = account.status.trim().to_ascii_lowercase();
    if matches!(status.as_str(), "disabled" | "deactivated") {
        return "disabled";
    }

    if account.cooldown_until.is_some_and(|until| until > now_ts) {
        return "cooldown";
    }

    if matches!(status.as_str(), "inactive" | "unavailable") {
        return "unavailable";
    }

    if usage_exhausted(usage) {
        return "quota_exhausted";
    }

    "online"
}

fn usage_exhausted(usage: Option<&UsageSnapshotResult>) -> bool {
    let Some(usage) = usage else {
        return false;
    };

    let availability_status = usage
        .availability_status
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if availability_status == "unavailable" {
        return true;
    }

    let primary_exhausted = usage
        .used_percent
        .is_some_and(|value| value.is_finite() && value >= 100.0);
    let secondary_exhausted = usage
        .secondary_used_percent
        .is_some_and(|value| value.is_finite() && value >= 100.0);
    if usage.secondary_used_percent.is_some() {
        return primary_exhausted && secondary_exhausted;
    }
    primary_exhausted
}

fn build_gateway_metrics(
    request_logs: &[RequestLogSummary],
    latency_samples: &[i64],
    now_ts: i64,
) -> DashboardGatewayMetricsResult {
    let window_start = now_ts - HEALTH_WINDOW_SECS;
    let recent_logs = request_logs
        .iter()
        .filter(|item| item.created_at >= window_start)
        .collect::<Vec<_>>();

    let total_requests = recent_logs.len() as i64;
    let success_requests = recent_logs.iter().filter(|item| is_success(item)).count() as i64;
    let error_requests = recent_logs.iter().filter(|item| is_error(item)).count() as i64;
    let mut durations = if latency_samples.is_empty() {
        recent_logs
            .iter()
            .filter_map(|item| item.duration_ms)
            .filter(|value| *value >= 0)
            .collect::<Vec<_>>()
    } else {
        latency_samples
            .iter()
            .copied()
            .filter(|value| *value >= 0)
            .collect::<Vec<_>>()
    };
    durations.sort_unstable();

    DashboardGatewayMetricsResult {
        window_minutes: HEALTH_WINDOW_MINUTES,
        total_requests,
        success_requests,
        error_requests,
        qps: round2(total_requests as f64 / HEALTH_WINDOW_SECS as f64),
        success_rate: if total_requests <= 0 {
            100.0
        } else {
            round2(success_requests as f64 / total_requests as f64 * 100.0)
        },
        p50_latency_ms: percentile(&durations, 0.50),
        p95_latency_ms: percentile(&durations, 0.95),
        p99_latency_ms: percentile(&durations, 0.99),
    }
}

fn is_success(item: &RequestLogSummary) -> bool {
    item.status_code
        .is_some_and(|status| (200..300).contains(&status))
        && item.error.as_deref().unwrap_or_default().trim().is_empty()
}

fn is_error(item: &RequestLogSummary) -> bool {
    !item.error.as_deref().unwrap_or_default().trim().is_empty()
        || item
            .status_code
            .is_some_and(|status| !(200..300).contains(&status))
}

fn percentage(count: i64, total: i64) -> i64 {
    if total <= 0 {
        return 0;
    }
    ((count as f64 / total as f64) * 100.0).round() as i64
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn percentile(sorted_values: &[i64], percentile: f64) -> Option<i64> {
    if sorted_values.is_empty() {
        return None;
    }
    let normalized = percentile.clamp(0.0, 1.0);
    let index = ((sorted_values.len() - 1) as f64 * normalized).ceil() as usize;
    sorted_values.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::{build_gateway_metrics, classify_account_status};
    use codexmanager_core::rpc::types::{AccountSummary, RequestLogSummary, UsageSnapshotResult};

    fn account(status: &str) -> AccountSummary {
        AccountSummary {
            id: "acc_1".to_string(),
            label: "账号".to_string(),
            group_name: None,
            tags: Vec::new(),
            sort: 0,
            status: status.to_string(),
            health_score: 100,
            last_status_reason: None,
            last_status_changed_at: None,
            last_governance_reason: None,
            last_governance_at: None,
            last_isolation_reason_code: None,
            last_isolation_reason: None,
            last_isolation_at: None,
            cooldown_until: None,
            cooldown_reason_code: None,
            cooldown_reason: None,
            subscription_plan_type: None,
            subscription_updated_at: None,
            team_manager_uploaded_at: None,
            official_promo_link: None,
            official_promo_link_updated_at: None,
        }
    }

    #[test]
    fn classify_account_status_prioritizes_disabled_and_cooldown() {
        let disabled = account("disabled");
        assert_eq!(classify_account_status(&disabled, None, 100), "disabled");

        let mut cooldown = account("active");
        cooldown.cooldown_until = Some(120);
        assert_eq!(classify_account_status(&cooldown, None, 100), "cooldown");
    }

    #[test]
    fn gateway_metrics_calculates_percentiles_and_success_rate() {
        let logs = vec![
            RequestLogSummary {
                trace_id: None,
                key_id: None,
                account_id: None,
                initial_account_id: None,
                attempted_account_ids: Vec::new(),
                route_strategy: Some("weighted".to_string()),
                requested_model: None,
                model_fallback_path: Vec::new(),
                request_path: "/v1/responses".to_string(),
                original_path: None,
                adapted_path: None,
                method: "POST".to_string(),
                model: None,
                reasoning_effort: None,
                response_adapter: None,
                upstream_url: None,
                status_code: Some(200),
                duration_ms: Some(100),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: None,
                created_at: 299,
            },
            RequestLogSummary {
                trace_id: None,
                key_id: None,
                account_id: None,
                initial_account_id: None,
                attempted_account_ids: Vec::new(),
                route_strategy: Some("weighted".to_string()),
                requested_model: None,
                model_fallback_path: Vec::new(),
                request_path: "/v1/responses".to_string(),
                original_path: None,
                adapted_path: None,
                method: "POST".to_string(),
                model: None,
                reasoning_effort: None,
                response_adapter: None,
                upstream_url: None,
                status_code: Some(500),
                duration_ms: Some(400),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: Some("upstream".to_string()),
                created_at: 300,
            },
        ];

        let metrics = build_gateway_metrics(&logs, &[], 300);
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.success_requests, 1);
        assert_eq!(metrics.error_requests, 1);
        assert_eq!(metrics.success_rate, 50.0);
        assert_eq!(metrics.p50_latency_ms, Some(400));
        assert_eq!(metrics.p95_latency_ms, Some(400));
    }

    #[test]
    fn gateway_metrics_prefers_ring_buffer_latency_samples() {
        let metrics = build_gateway_metrics(&[], &[90, 120, 300], 300);

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.p50_latency_ms, Some(120));
        assert_eq!(metrics.p95_latency_ms, Some(300));
    }

    #[test]
    fn unavailable_usage_is_reported_as_quota_exhausted() {
        let usage = UsageSnapshotResult {
            account_id: Some("acc_1".to_string()),
            availability_status: Some("unavailable".to_string()),
            used_percent: Some(100.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: Some(1),
        };
        let active = account("active");
        assert_eq!(
            classify_account_status(&active, Some(&usage), 100),
            "quota_exhausted"
        );
    }
}
