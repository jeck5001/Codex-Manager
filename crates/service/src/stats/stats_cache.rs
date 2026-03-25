use codexmanager_core::rpc::types::{
    CacheAnalyticsByKeyResult, CacheAnalyticsByModelResult, CacheAnalyticsKeyItem,
    CacheAnalyticsModelItem, CacheAnalyticsSummaryResult, CacheAnalyticsTrendDayItem,
    CacheAnalyticsTrendResult, CostSummaryParams,
};

use crate::storage_helpers::open_storage;

use super::cost_summary::resolve_cost_range;

fn hit_rate(cached: i64, total: i64) -> f64 {
    if total <= 0 {
        return 0.0;
    }
    ((cached as f64 / total as f64) * 1000.0).round() / 10.0
}

pub(crate) fn read_cache_summary(
    params: CostSummaryParams,
) -> Result<CacheAnalyticsSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_cost_range(
        params.preset,
        params.start_ts,
        params.end_ts,
    )?;
    let summary = storage
        .summarize_cache_analytics_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(CacheAnalyticsSummaryResult {
        preset,
        range_start,
        range_end,
        total_requests: summary.total_requests,
        cached_requests: summary.cached_requests,
        hit_rate: hit_rate(summary.cached_requests, summary.total_requests),
        total_input_tokens: summary.total_input_tokens,
        cached_input_tokens: summary.cached_input_tokens,
        cache_token_ratio: if summary.total_input_tokens > 0 {
            ((summary.cached_input_tokens as f64 / summary.total_input_tokens as f64) * 1000.0)
                .round()
                / 10.0
        } else {
            0.0
        },
        estimated_savings_usd: summary.estimated_savings_usd.max(0.0),
    })
}

pub(crate) fn read_cache_trend(
    params: CostSummaryParams,
) -> Result<CacheAnalyticsTrendResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_cost_range(
        params.preset,
        params.start_ts,
        params.end_ts,
    )?;
    let items = storage
        .summarize_cache_analytics_trend_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(CacheAnalyticsTrendResult {
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| CacheAnalyticsTrendDayItem {
                day: item.day,
                total_requests: item.total_requests,
                cached_requests: item.cached_requests,
                hit_rate: hit_rate(item.cached_requests, item.total_requests),
                total_input_tokens: item.total_input_tokens,
                cached_input_tokens: item.cached_input_tokens,
            })
            .collect(),
    })
}

pub(crate) fn read_cache_by_model(
    params: CostSummaryParams,
) -> Result<CacheAnalyticsByModelResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_cost_range(
        params.preset,
        params.start_ts,
        params.end_ts,
    )?;
    let items = storage
        .summarize_cache_analytics_by_model_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(CacheAnalyticsByModelResult {
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| CacheAnalyticsModelItem {
                model: item.model,
                total_requests: item.total_requests,
                cached_requests: item.cached_requests,
                hit_rate: hit_rate(item.cached_requests, item.total_requests),
                total_input_tokens: item.total_input_tokens,
                cached_input_tokens: item.cached_input_tokens,
                estimated_savings_usd: item.estimated_savings_usd.max(0.0),
            })
            .collect(),
    })
}

pub(crate) fn read_cache_by_key(
    params: CostSummaryParams,
) -> Result<CacheAnalyticsByKeyResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_cost_range(
        params.preset,
        params.start_ts,
        params.end_ts,
    )?;
    let items = storage
        .summarize_cache_analytics_by_key_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(CacheAnalyticsByKeyResult {
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| CacheAnalyticsKeyItem {
                key_id: item.key_id,
                total_requests: item.total_requests,
                cached_requests: item.cached_requests,
                hit_rate: hit_rate(item.cached_requests, item.total_requests),
                total_input_tokens: item.total_input_tokens,
                cached_input_tokens: item.cached_input_tokens,
                estimated_savings_usd: item.estimated_savings_usd.max(0.0),
            })
            .collect(),
    })
}
