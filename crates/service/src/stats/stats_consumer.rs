use codexmanager_core::rpc::types::{
    ConsumerDetailParams, ConsumerModelBreakdownResult, ConsumerModelItem, ConsumerOverviewResult,
    ConsumerRankingParams, ConsumerRankingResult, ConsumerTrendDayItem, ConsumerTrendResult,
    CostSummaryKeyItem,
};

use crate::storage_helpers::open_storage;

use super::cost_summary::resolve_cost_range;

pub(crate) fn read_consumer_overview(
    params: ConsumerDetailParams,
) -> Result<ConsumerOverviewResult, String> {
    let key_id = params.key_id.trim().to_string();
    if key_id.is_empty() {
        return Err("keyId is required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) =
        resolve_cost_range(params.preset, params.start_ts, params.end_ts)?;
    let overview = storage
        .summarize_consumer_overview_between(&key_id, range_start, range_end)
        .map_err(|err| err.to_string())?;

    let success_rate = if overview.request_count > 0 {
        ((overview.success_count as f64 / overview.request_count as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    Ok(ConsumerOverviewResult {
        key_id,
        preset,
        range_start,
        range_end,
        request_count: overview.request_count,
        input_tokens: overview.input_tokens,
        cached_input_tokens: overview.cached_input_tokens,
        output_tokens: overview.output_tokens,
        total_tokens: overview.total_tokens,
        estimated_cost_usd: overview.estimated_cost_usd.max(0.0),
        success_rate,
        avg_duration_ms: overview.avg_duration_ms.map(|v| (v * 10.0).round() / 10.0),
    })
}

pub(crate) fn read_consumer_trend(
    params: ConsumerDetailParams,
) -> Result<ConsumerTrendResult, String> {
    let key_id = params.key_id.trim().to_string();
    if key_id.is_empty() {
        return Err("keyId is required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) =
        resolve_cost_range(params.preset, params.start_ts, params.end_ts)?;
    let items = storage
        .summarize_consumer_trend_between(&key_id, range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(ConsumerTrendResult {
        key_id,
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| ConsumerTrendDayItem {
                day: item.day,
                request_count: item.request_count,
                input_tokens: item.input_tokens,
                output_tokens: item.output_tokens,
                estimated_cost_usd: item.estimated_cost_usd.max(0.0),
            })
            .collect(),
    })
}

pub(crate) fn read_consumer_model_breakdown(
    params: ConsumerDetailParams,
) -> Result<ConsumerModelBreakdownResult, String> {
    let key_id = params.key_id.trim().to_string();
    if key_id.is_empty() {
        return Err("keyId is required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) =
        resolve_cost_range(params.preset, params.start_ts, params.end_ts)?;
    let items = storage
        .summarize_consumer_model_breakdown_between(&key_id, range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(ConsumerModelBreakdownResult {
        key_id,
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| ConsumerModelItem {
                model: item.model,
                request_count: item.request_count,
                input_tokens: item.input_tokens,
                output_tokens: item.output_tokens,
                total_tokens: item.total_tokens,
                estimated_cost_usd: item.estimated_cost_usd.max(0.0),
            })
            .collect(),
    })
}

pub(crate) fn read_consumer_ranking(
    params: ConsumerRankingParams,
) -> Result<ConsumerRankingResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) =
        resolve_cost_range(params.preset, params.start_ts, params.end_ts)?;
    let limit = params.limit.unwrap_or(20).max(1).min(100);
    let items = storage
        .summarize_consumer_ranking_between(range_start, range_end, limit)
        .map_err(|err| err.to_string())?;

    Ok(ConsumerRankingResult {
        preset,
        range_start,
        range_end,
        items: items
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
    })
}
