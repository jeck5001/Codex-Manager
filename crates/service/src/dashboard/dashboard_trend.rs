use codexmanager_core::{
    rpc::types::{DashboardTrendPoint, DashboardTrendResult, RequestLogSummary},
    storage::now_ts,
};

use crate::requestlog_list;

const TREND_BUCKET_MINUTES: i64 = 1;
const TREND_WINDOW_MINUTES: i64 = 60;
const TREND_WINDOW_SECS: i64 = TREND_WINDOW_MINUTES * 60;
const TREND_BUCKET_SECS: i64 = TREND_BUCKET_MINUTES * 60;
const DASHBOARD_REQUEST_LOG_LIMIT: i64 = 20_000;

pub(crate) fn read_dashboard_trend() -> Result<DashboardTrendResult, String> {
    let generated_at = now_ts();
    let request_logs = requestlog_list::read_request_logs(None, Some(DASHBOARD_REQUEST_LOG_LIMIT))?;
    Ok(build_dashboard_trend(&request_logs, generated_at))
}

fn build_dashboard_trend(request_logs: &[RequestLogSummary], now_ts: i64) -> DashboardTrendResult {
    let end_bucket = now_ts - now_ts.rem_euclid(TREND_BUCKET_SECS);
    let start_bucket = end_bucket - (TREND_WINDOW_MINUTES - 1) * TREND_BUCKET_SECS;
    let mut points = (0..TREND_WINDOW_MINUTES)
        .map(|index| DashboardTrendPoint {
            bucket_ts: start_bucket + index * TREND_BUCKET_SECS,
            request_count: 0,
            error_count: 0,
            error_rate: 0.0,
        })
        .collect::<Vec<_>>();

    let earliest_ts = end_bucket - TREND_WINDOW_SECS;
    for item in request_logs
        .iter()
        .filter(|item| item.created_at >= earliest_ts)
    {
        let bucket_ts = item.created_at - item.created_at.rem_euclid(TREND_BUCKET_SECS);
        if bucket_ts < start_bucket || bucket_ts > end_bucket {
            continue;
        }
        let index = ((bucket_ts - start_bucket) / TREND_BUCKET_SECS) as usize;
        if let Some(point) = points.get_mut(index) {
            point.request_count += 1;
            if is_error(item) {
                point.error_count += 1;
            }
        }
    }

    for point in &mut points {
        point.error_rate = if point.request_count <= 0 {
            0.0
        } else {
            round2(point.error_count as f64 / point.request_count as f64 * 100.0)
        };
    }

    DashboardTrendResult {
        generated_at: now_ts,
        bucket_minutes: TREND_BUCKET_MINUTES,
        points,
    }
}

fn is_error(item: &RequestLogSummary) -> bool {
    !item.error.as_deref().unwrap_or_default().trim().is_empty()
        || item
            .status_code
            .is_some_and(|status| !(200..300).contains(&status))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::build_dashboard_trend;
    use codexmanager_core::rpc::types::RequestLogSummary;

    fn request_log(created_at: i64, status_code: i64) -> RequestLogSummary {
        RequestLogSummary {
            trace_id: None,
            key_id: None,
            account_id: None,
            initial_account_id: None,
            attempted_account_ids: Vec::new(),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("balanced".to_string()),
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
            status_code: Some(status_code),
            duration_ms: Some(100),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            reasoning_output_tokens: None,
            estimated_cost_usd: None,
            error: None,
            created_at,
        }
    }

    #[test]
    fn trend_builds_full_hour_and_groups_by_minute() {
        let trend = build_dashboard_trend(
            &[request_log(3_600 - 30, 200), request_log(3_600 - 25, 500)],
            3_600,
        );
        assert_eq!(trend.points.len(), 60);
        let current_bucket = trend.points.last().expect("current minute bucket");
        assert_eq!(current_bucket.request_count, 0);
        let previous_bucket = trend
            .points
            .iter()
            .rev()
            .nth(1)
            .expect("previous minute bucket");
        assert_eq!(previous_bucket.request_count, 2);
        assert_eq!(previous_bucket.error_count, 1);
        assert_eq!(previous_bucket.error_rate, 50.0);
    }
}
