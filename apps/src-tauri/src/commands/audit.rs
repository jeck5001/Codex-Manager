use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_audit_list(
    addr: Option<String>,
    action: Option<String>,
    object_type: Option<String>,
    object_id: Option<String>,
    time_from: Option<i64>,
    time_to: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "action": action,
        "objectType": object_type,
        "objectId": object_id,
        "timeFrom": time_from,
        "timeTo": time_to,
        "page": page,
        "pageSize": page_size
    });
    rpc_call_in_background("audit/list", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_audit_export(
    addr: Option<String>,
    format: Option<String>,
    action: Option<String>,
    object_type: Option<String>,
    object_id: Option<String>,
    time_from: Option<i64>,
    time_to: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "format": format,
        "action": action,
        "objectType": object_type,
        "objectId": object_id,
        "timeFrom": time_from,
        "timeTo": time_to
    });
    rpc_call_in_background("audit/export", addr, Some(params)).await
}
