use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_requestlog_list(
    addr: Option<String>,
    query: Option<String>,
    status_filter: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "query": query,
        "statusFilter": status_filter,
        "page": page,
        "pageSize": page_size
    });
    rpc_call_in_background("requestlog/list", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_requestlog_clear(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("requestlog/clear", addr, None).await
}

#[tauri::command]
pub async fn service_requestlog_summary(
    addr: Option<String>,
    query: Option<String>,
    status_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "query": query,
        "statusFilter": status_filter
    });
    rpc_call_in_background("requestlog/summary", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_requestlog_today_summary(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("requestlog/today_summary", addr, None).await
}
