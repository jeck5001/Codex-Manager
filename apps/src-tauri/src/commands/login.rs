use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_login_start(
    addr: Option<String>,
    login_type: String,
    open_browser: Option<bool>,
    note: Option<String>,
    tags: Option<String>,
    group_name: Option<String>,
    workspace_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "type": login_type,
      "openBrowser": open_browser.unwrap_or(true),
      "note": note,
      "tags": tags,
      "groupName": group_name,
      "workspaceId": workspace_id
    });
    rpc_call_in_background("account/login/start", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_login_status(
    addr: Option<String>,
    login_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "loginId": login_id
    });
    rpc_call_in_background("account/login/status", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_login_complete(
    addr: Option<String>,
    state: String,
    code: String,
    redirect_uri: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "state": state,
      "code": code,
      "redirectUri": redirect_uri
    });
    rpc_call_in_background("account/login/complete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_login_chatgpt_auth_tokens(
    addr: Option<String>,
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    chatgpt_account_id: Option<String>,
    workspace_id: Option<String>,
    chatgpt_plan_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "type": "chatgptAuthTokens",
      "accessToken": access_token,
      "refreshToken": refresh_token,
      "idToken": id_token,
      "chatgptAccountId": chatgpt_account_id,
      "workspaceId": workspace_id,
      "chatgptPlanType": chatgpt_plan_type
    });
    rpc_call_in_background("account/login/start", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_read(
    addr: Option<String>,
    refresh_token: Option<bool>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "refreshToken": refresh_token.unwrap_or(false)
    });
    rpc_call_in_background("account/read", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_logout(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("account/logout", addr, None).await
}

#[tauri::command]
pub async fn service_chatgpt_auth_tokens_refresh(
    addr: Option<String>,
    reason: Option<String>,
    previous_account_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "reason": reason.unwrap_or_else(|| "unauthorized".to_string()),
      "previousAccountId": previous_account_id
    });
    rpc_call_in_background("account/chatgptAuthTokens/refresh", addr, Some(params)).await
}
