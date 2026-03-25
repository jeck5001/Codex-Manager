use chrono::{Datelike, Duration, Local, LocalResult, TimeZone};
use codexmanager_core::rpc::types::{
    CostSummaryDayItem, CostSummaryKeyItem, CostSummaryModelItem, CostSummaryParams,
    CostSummaryResult, CostUsageSummaryResult,
};

use crate::storage_helpers::open_storage;

fn start_of_local_day_ts(day: chrono::NaiveDate) -> Result<i64, String> {
    let naive = day
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "build local day start failed".to_string())?;
    Ok(match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => value.timestamp(),
        LocalResult::Ambiguous(a, b) => a.timestamp().min(b.timestamp()),
        LocalResult::None => Local::now().timestamp(),
    })
}

pub(crate) fn resolve_cost_range(
    preset: Option<String>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<(String, i64, i64), String> {
    let preset = preset
        .unwrap_or_else(|| "today".to_string())
        .trim()
        .to_ascii_lowercase();
    let now = Local::now();
    let today = now.date_naive();

    match preset.as_str() {
        "today" => {
            let start = start_of_local_day_ts(today)?;
            Ok((preset, start, start + 24 * 60 * 60))
        }
        "week" => {
            let weekday = today.weekday().num_days_from_monday() as i64;
            let start_day = today - Duration::days(weekday);
            let start = start_of_local_day_ts(start_day)?;
            Ok((preset, start, start + 7 * 24 * 60 * 60))
        }
        "month" => {
            let start_day = today
                .with_day(1)
                .ok_or_else(|| "build month start failed".to_string())?;
            let next_month_day = if start_day.month() == 12 {
                chrono::NaiveDate::from_ymd_opt(start_day.year() + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(start_day.year(), start_day.month() + 1, 1)
            }
            .ok_or_else(|| "build next month start failed".to_string())?;
            Ok((
                preset,
                start_of_local_day_ts(start_day)?,
                start_of_local_day_ts(next_month_day)?,
            ))
        }
        "custom" => {
            let start = start_ts
                .ok_or_else(|| "startTs required".to_string())?;
            let end = end_ts.ok_or_else(|| "endTs required".to_string())?;
            if end <= start {
                return Err("endTs must be greater than startTs".to_string());
            }
            Ok((preset, start, end))
        }
        _ => Err("preset must be one of today/week/month/custom".to_string()),
    }
}

fn resolve_range(params: CostSummaryParams) -> Result<(String, i64, i64), String> {
    resolve_cost_range(params.preset, params.start_ts, params.end_ts)
}

pub(crate) fn read_cost_summary(params: CostSummaryParams) -> Result<CostSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_range(params)?;
    let total = storage
        .summarize_cost_usage_between(range_start, range_end)
        .map_err(|err| err.to_string())?;
    let mut by_key = storage
        .summarize_cost_usage_by_key_between(range_start, range_end)
        .map_err(|err| err.to_string())?;
    let mut by_model = storage
        .summarize_cost_usage_by_model_between(range_start, range_end)
        .map_err(|err| err.to_string())?;
    let by_day = storage
        .summarize_cost_usage_by_day_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    by_key.sort_by(|left, right| {
        right
            .estimated_cost_usd
            .total_cmp(&left.estimated_cost_usd)
            .then_with(|| left.key_id.cmp(&right.key_id))
    });
    by_model.sort_by(|left, right| {
        right
            .estimated_cost_usd
            .total_cmp(&left.estimated_cost_usd)
            .then_with(|| left.model.cmp(&right.model))
    });

    Ok(CostSummaryResult {
        preset,
        range_start,
        range_end,
        total: CostUsageSummaryResult {
            request_count: total.request_count,
            input_tokens: total.input_tokens,
            cached_input_tokens: total.cached_input_tokens,
            output_tokens: total.output_tokens,
            total_tokens: total.total_tokens,
            estimated_cost_usd: total.estimated_cost_usd.max(0.0),
        },
        by_key: by_key
            .into_iter()
            .map(|item| CostSummaryKeyItem {
                key_id: item.key_id,
                request_count: item.request_count,
                input_tokens: item.input_tokens,
                cached_input_tokens: item.cached_input_tokens,
                output_tokens: item.output_tokens,
                total_tokens: item.total_tokens,
                estimated_cost_usd: item.estimated_cost_usd.max(0.0),
            })
            .collect(),
        by_model: by_model
            .into_iter()
            .map(|item| CostSummaryModelItem {
                model: item.model,
                request_count: item.request_count,
                input_tokens: item.input_tokens,
                cached_input_tokens: item.cached_input_tokens,
                output_tokens: item.output_tokens,
                total_tokens: item.total_tokens,
                estimated_cost_usd: item.estimated_cost_usd.max(0.0),
            })
            .collect(),
        by_day: by_day
            .into_iter()
            .map(|item| CostSummaryDayItem {
                day: item.day,
                request_count: item.request_count,
                input_tokens: item.input_tokens,
                cached_input_tokens: item.cached_input_tokens,
                output_tokens: item.output_tokens,
                total_tokens: item.total_tokens,
                estimated_cost_usd: item.estimated_cost_usd.max(0.0),
            })
            .collect(),
    })
}
