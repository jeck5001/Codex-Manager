use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: u64,
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    pub server_name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub label: String,
    pub status: String,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub note: Option<String>,
    pub tags: Option<String>,
    pub group_name: Option<String>,
    pub sort: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountListResult {
    pub items: Vec<AccountSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthInfo {
    pub user_code_url: String,
    pub token_url: String,
    pub verification_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStartResult {
    pub auth_url: String,
    pub login_id: String,
    pub login_type: String,
    pub issuer: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub warning: Option<String>,
    pub device: Option<DeviceAuthInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshotResult {
    pub account_id: Option<String>,
    pub used_percent: Option<f64>,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<i64>,
    pub secondary_used_percent: Option<f64>,
    pub secondary_window_minutes: Option<i64>,
    pub secondary_resets_at: Option<i64>,
    pub credits_json: Option<String>,
    pub captured_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageReadResult {
    pub snapshot: Option<UsageSnapshotResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageListResult {
    pub items: Vec<UsageSnapshotResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySummary {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyListResult {
    pub items: Vec<ApiKeySummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreateResult {
    pub id: String,
    pub key: String,
}
