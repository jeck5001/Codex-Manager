use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_web_auth_two_factor_setup(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("webAuth/2fa/setup", addr, None).await
}

#[tauri::command]
pub async fn service_web_auth_two_factor_verify(
    addr: Option<String>,
    setup_token: Option<String>,
    code: Option<String>,
    recovery_code: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "setupToken": setup_token,
      "code": code,
      "recoveryCode": recovery_code,
    });
    rpc_call_in_background("webAuth/2fa/verify", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_web_auth_two_factor_disable(
    addr: Option<String>,
    code: Option<String>,
    recovery_code: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "code": code,
      "recoveryCode": recovery_code,
    });
    rpc_call_in_background("webAuth/2fa/disable", addr, Some(params)).await
}
