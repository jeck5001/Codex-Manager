use chrono::{Duration, Local};
use codexmanager_core::rpc::types::{
    HeatmapCellItem, HeatmapTrendResult, ModelTrendItem, ModelTrendResult, RequestTrendItem,
    RequestTrendResult, TrendQueryParams,
};

use crate::storage_helpers::open_storage;

fn resolve_range(params: &TrendQueryParams) -> Result<(String, i64, i64), String> {
    let preset = params
        .preset
        .clone()
        .unwrap_or_else(|| "30d".to_string())
        .trim()
        .to_ascii_lowercase();
    let now = Local::now().timestamp();

    match preset.as_str() {
        "30d" => Ok((preset, now - Duration::days(30).num_seconds(), now + 1)),
        "90d" => Ok((preset, now - Duration::days(90).num_seconds(), now + 1)),
        "custom" => {
            let start = params
                .start_ts
                .ok_or_else(|| "startTs required".to_string())?;
            let end = params.end_ts.ok_or_else(|| "endTs required".to_string())?;
            if end <= start {
                return Err("endTs must be greater than startTs".to_string());
            }
            Ok((preset, start, end))
        }
        _ => Err("preset must be one of 30d/90d/custom".to_string()),
    }
}

fn resolve_granularity(params: &TrendQueryParams) -> Result<String, String> {
    let granularity = params
        .granularity
        .clone()
        .unwrap_or_else(|| "day".to_string())
        .trim()
        .to_ascii_lowercase();
    match granularity.as_str() {
        "day" | "week" | "month" => Ok(granularity),
        _ => Err("granularity must be one of day/week/month".to_string()),
    }
}

fn success_rate(success_count: i64, request_count: i64) -> f64 {
    if request_count <= 0 {
        return 0.0;
    }
    ((success_count as f64 / request_count as f64) * 1000.0).round() / 10.0
}

pub(crate) fn read_request_trends(params: TrendQueryParams) -> Result<RequestTrendResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_range(&params)?;
    let granularity = resolve_granularity(&params)?;
    let items = storage
        .summarize_request_trends_between(range_start, range_end, &granularity)
        .map_err(|err| err.to_string())?;

    Ok(RequestTrendResult {
        preset,
        granularity,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| RequestTrendItem {
                bucket: item.bucket,
                request_count: item.request_count,
                success_count: item.success_count,
                success_rate: success_rate(item.success_count, item.request_count),
            })
            .collect(),
    })
}

pub(crate) fn read_model_trends(params: TrendQueryParams) -> Result<ModelTrendResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_range(&params)?;
    let items = storage
        .summarize_request_model_trends_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(ModelTrendResult {
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| ModelTrendItem {
                model: item.model,
                request_count: item.request_count,
                success_count: item.success_count,
                success_rate: success_rate(item.success_count, item.request_count),
            })
            .collect(),
    })
}

pub(crate) fn read_heatmap_trends(params: TrendQueryParams) -> Result<HeatmapTrendResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let (preset, range_start, range_end) = resolve_range(&params)?;
    let items = storage
        .summarize_request_heatmap_between(range_start, range_end)
        .map_err(|err| err.to_string())?;

    Ok(HeatmapTrendResult {
        preset,
        range_start,
        range_end,
        items: items
            .into_iter()
            .map(|item| HeatmapCellItem {
                weekday: item.weekday,
                hour: item.hour,
                request_count: item.request_count,
                success_count: item.success_count,
                success_rate: success_rate(item.success_count, item.request_count),
            })
            .collect(),
    })
}
