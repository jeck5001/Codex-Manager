use codexmanager_core::rpc::types::{
    ConsumerDetailParams, ConsumerRankingParams, CostSummaryParams, JsonRpcRequest,
    JsonRpcResponse, ModelPricingItem, TrendQueryParams,
};

use crate::{
    stats_cache, stats_consumer, stats_cost_export, stats_cost_summary, stats_model_pricing,
    stats_trends,
};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "stats/cost/summary" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cost_summary::read_cost_summary(params))
        }
        "stats/cost/export" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cost_export::export_cost_summary(params))
        }
        "stats/cost/modelPricing/get" => {
            super::value_or_error(stats_model_pricing::read_model_pricing())
        }
        "stats/cost/modelPricing/set" => {
            let items = req
                .params
                .as_ref()
                .and_then(|params| params.get("items"))
                .and_then(|value| {
                    serde_json::from_value::<Vec<ModelPricingItem>>(value.clone()).ok()
                })
                .unwrap_or_default();
            super::ok_or_error(stats_model_pricing::update_model_pricing(items))
        }
        "stats/trends/requests" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<TrendQueryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_trends::read_request_trends(params))
        }
        "stats/trends/models" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<TrendQueryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_trends::read_model_trends(params))
        }
        "stats/trends/heatmap" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<TrendQueryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_trends::read_heatmap_trends(params))
        }
        // -- Consumer Analytics --
        "stats/consumer/overview" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| {
                    serde_json::from_value::<ConsumerDetailParams>(value.clone()).ok()
                })
                .unwrap_or_default();
            super::value_or_error(stats_consumer::read_consumer_overview(params))
        }
        "stats/consumer/trend" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| {
                    serde_json::from_value::<ConsumerDetailParams>(value.clone()).ok()
                })
                .unwrap_or_default();
            super::value_or_error(stats_consumer::read_consumer_trend(params))
        }
        "stats/consumer/modelBreakdown" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| {
                    serde_json::from_value::<ConsumerDetailParams>(value.clone()).ok()
                })
                .unwrap_or_default();
            super::value_or_error(stats_consumer::read_consumer_model_breakdown(params))
        }
        "stats/consumer/ranking" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| {
                    serde_json::from_value::<ConsumerRankingParams>(value.clone()).ok()
                })
                .unwrap_or_default();
            super::value_or_error(stats_consumer::read_consumer_ranking(params))
        }
        // -- Cache Analytics --
        "stats/cache/summary" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cache::read_cache_summary(params))
        }
        "stats/cache/trend" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cache::read_cache_trend(params))
        }
        "stats/cache/byModel" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cache::read_cache_by_model(params))
        }
        "stats/cache/byKey" => {
            let params = req
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<CostSummaryParams>(value.clone()).ok())
                .unwrap_or_default();
            super::value_or_error(stats_cache::read_cache_by_key(params))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
