use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_apikey_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("apikey/list", addr, None).await
}

#[tauri::command]
pub async fn service_apikey_read_secret(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/readSecret", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_rate_limit_get(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/rateLimit/get", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_rate_limit_set(
    addr: Option<String>,
    key_id: String,
    rpm: Option<i64>,
    tpm: Option<i64>,
    daily_limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": key_id,
      "rpm": rpm,
      "tpm": tpm,
      "dailyLimit": daily_limit,
    });
    rpc_call_in_background("apikey/rateLimit/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_model_fallback_get(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/modelFallback/get", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_model_fallback_set(
    addr: Option<String>,
    key_id: String,
    model_chain: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": key_id,
      "modelChain": model_chain.unwrap_or_default(),
    });
    rpc_call_in_background("apikey/modelFallback/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_allowed_models_get(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/allowedModels/get", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_allowed_models_set(
    addr: Option<String>,
    key_id: String,
    allowed_models: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": key_id,
      "allowedModels": allowed_models.unwrap_or_default(),
    });
    rpc_call_in_background("apikey/allowedModels/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_response_cache_get(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/responseCache/get", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_response_cache_set(
    addr: Option<String>,
    key_id: String,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": key_id,
      "enabled": enabled,
    });
    rpc_call_in_background("apikey/responseCache/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_create(
    addr: Option<String>,
    name: Option<String>,
    model_slug: Option<String>,
    reasoning_effort: Option<String>,
    protocol_type: Option<String>,
    upstream_base_url: Option<String>,
    static_headers_json: Option<String>,
    expires_at: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "name": name,
      "modelSlug": model_slug,
      "reasoningEffort": reasoning_effort,
      "protocolType": protocol_type,
      "upstreamBaseUrl": upstream_base_url,
      "staticHeadersJson": static_headers_json,
      "expiresAt": expires_at,
    });
    rpc_call_in_background("apikey/create", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_models(
    addr: Option<String>,
    refresh_remote: Option<bool>,
) -> Result<serde_json::Value, String> {
    let params = refresh_remote.map(|value| serde_json::json!({ "refreshRemote": value }));
    rpc_call_in_background("apikey/models", addr, params).await
}

#[tauri::command]
pub async fn service_apikey_usage_stats(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("apikey/usageStats", addr, None).await
}

#[tauri::command]
pub async fn service_apikey_update_model(
    addr: Option<String>,
    key_id: String,
    model_slug: Option<String>,
    reasoning_effort: Option<String>,
    protocol_type: Option<String>,
    upstream_base_url: Option<String>,
    static_headers_json: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "id": key_id,
      "modelSlug": model_slug,
      "reasoningEffort": reasoning_effort,
      "protocolType": protocol_type,
      "upstreamBaseUrl": upstream_base_url,
      "staticHeadersJson": static_headers_json,
    });
    rpc_call_in_background("apikey/updateModel", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_delete(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/delete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_renew(
    addr: Option<String>,
    key_id: String,
    expires_at: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id, "expiresAt": expires_at });
    rpc_call_in_background("apikey/renew", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_disable(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/disable", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_apikey_enable(
    addr: Option<String>,
    key_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": key_id });
    rpc_call_in_background("apikey/enable", addr, Some(params)).await
}
