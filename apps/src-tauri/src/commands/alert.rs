use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_alert_rules_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("alert/rules/list", addr, None).await
}

#[tauri::command]
pub async fn service_alert_rules_upsert(
    addr: Option<String>,
    id: Option<String>,
    name: String,
    r#type: String,
    config: serde_json::Value,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": id,
      "name": name,
      "type": r#type,
      "config": config,
      "enabled": enabled,
    });
    rpc_call_in_background("alert/rules/upsert", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_alert_rules_delete(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("alert/rules/delete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_alert_channels_list(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("alert/channels/list", addr, None).await
}

#[tauri::command]
pub async fn service_alert_channels_upsert(
    addr: Option<String>,
    id: Option<String>,
    name: String,
    r#type: String,
    config: serde_json::Value,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": id,
      "name": name,
      "type": r#type,
      "config": config,
      "enabled": enabled,
    });
    rpc_call_in_background("alert/channels/upsert", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_alert_channels_delete(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("alert/channels/delete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_alert_channels_test(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("alert/channels/test", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_alert_history_list(
    addr: Option<String>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "limit": limit });
    rpc_call_in_background("alert/history/list", addr, Some(params)).await
}
