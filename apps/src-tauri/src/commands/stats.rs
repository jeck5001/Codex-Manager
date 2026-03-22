use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_stats_cost_model_pricing_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("stats/cost/modelPricing/get", addr, None).await
}

#[tauri::command]
pub async fn service_stats_cost_summary(
    addr: Option<String>,
    preset: Option<String>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "preset": preset,
        "startTs": start_ts,
        "endTs": end_ts,
    });
    rpc_call_in_background("stats/cost/summary", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_stats_cost_export(
    addr: Option<String>,
    preset: Option<String>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "preset": preset,
        "startTs": start_ts,
        "endTs": end_ts,
    });
    rpc_call_in_background("stats/cost/export", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_stats_cost_model_pricing_set(
    addr: Option<String>,
    items: Option<Vec<serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "items": items.unwrap_or_default(),
    });
    rpc_call_in_background("stats/cost/modelPricing/set", addr, Some(params)).await
}
