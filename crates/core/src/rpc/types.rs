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
    pub user_agent: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub label: String,
    pub group_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub sort: i64,
    pub status: String,
    #[serde(default)]
    pub health_score: i64,
    #[serde(default)]
    pub last_status_reason: Option<String>,
    #[serde(default)]
    pub last_status_changed_at: Option<i64>,
    #[serde(default)]
    pub last_governance_reason: Option<String>,
    #[serde(default)]
    pub last_governance_at: Option<i64>,
    #[serde(default)]
    pub last_isolation_reason_code: Option<String>,
    #[serde(default)]
    pub last_isolation_reason: Option<String>,
    #[serde(default)]
    pub last_isolation_at: Option<i64>,
    #[serde(default)]
    pub cooldown_until: Option<i64>,
    #[serde(default)]
    pub cooldown_reason_code: Option<String>,
    #[serde(default)]
    pub cooldown_reason: Option<String>,
    #[serde(default)]
    pub new_account_protection_until: Option<i64>,
    #[serde(default)]
    pub new_account_protection_reason: Option<String>,
    #[serde(default)]
    pub subscription_plan_type: Option<String>,
    #[serde(default)]
    pub subscription_updated_at: Option<i64>,
    #[serde(default)]
    pub team_manager_uploaded_at: Option<i64>,
    #[serde(default)]
    pub official_promo_link: Option<String>,
    #[serde(default)]
    pub official_promo_link_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountListParams {
    pub page: i64,
    pub page_size: i64,
    pub query: Option<String>,
    pub filter: Option<String>,
    pub group_filter: Option<String>,
}

impl Default for AccountListParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 5,
            query: None,
            filter: None,
            group_filter: None,
        }
    }
}

impl AccountListParams {
    pub fn normalized(self) -> Self {
        // 中文注释：分页参数小于 1 时回退到默认值，避免出现负偏移或零页大小。
        Self {
            page: if self.page < 1 { 1 } else { self.page },
            page_size: if self.page_size < 1 {
                5
            } else {
                self.page_size
            },
            query: self.query,
            filter: self.filter,
            group_filter: self.group_filter,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountListResult {
    pub items: Vec<AccountSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
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
    pub availability_status: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindowResult {
    pub used_percent: i64,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSnapshotResult {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateLimitWindowResult>,
    pub secondary: Option<RateLimitWindowResult>,
    pub credits: Option<serde_json::Value>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRateLimitsReadResult {
    pub rate_limits: RateLimitSnapshotResult,
    pub rate_limits_by_limit_id:
        Option<std::collections::BTreeMap<String, RateLimitSnapshotResult>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageListResult {
    pub items: Vec<UsageSnapshotResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAggregateSummaryResult {
    pub primary_bucket_count: i64,
    pub primary_known_count: i64,
    pub primary_unknown_count: i64,
    pub primary_remain_percent: Option<i64>,
    pub secondary_bucket_count: i64,
    pub secondary_known_count: i64,
    pub secondary_unknown_count: i64,
    pub secondary_remain_percent: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureReasonSummaryItem {
    pub code: String,
    pub label: String,
    pub count: i64,
    pub affected_accounts: i64,
    pub last_seen_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceSummaryItem {
    pub code: String,
    pub label: String,
    pub target_status: String,
    pub count: i64,
    pub affected_accounts: i64,
    pub last_seen_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationAuditItem {
    pub action: String,
    pub label: String,
    pub detail: String,
    pub account_id: Option<String>,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePredictionSummaryResult {
    pub quota_protection_enabled: bool,
    pub quota_protection_threshold_percent: i64,
    pub ready_account_count: i64,
    pub estimated_hours_to_threshold: Option<f64>,
    pub estimated_hours_to_pool_exhaustion: Option<f64>,
    pub threshold_limited_by: Option<String>,
    pub pool_limited_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySummary {
    pub id: String,
    pub name: Option<String>,
    pub model_slug: Option<String>,
    pub reasoning_effort: Option<String>,
    pub client_type: String,
    pub protocol_type: String,
    pub auth_scheme: String,
    pub upstream_base_url: Option<String>,
    pub static_headers_json: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyListResult {
    pub items: Vec<ApiKeySummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyUsageStatSummary {
    pub key_id: String,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsageStatListResult {
    pub items: Vec<ApiKeyUsageStatSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreateResult {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyRateLimitConfig {
    pub key_id: String,
    pub rpm: Option<i64>,
    pub tpm: Option<i64>,
    pub daily_limit: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyModelFallbackConfig {
    pub key_id: String,
    #[serde(default)]
    pub model_chain: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyAllowedModelsConfig {
    pub key_id: String,
    #[serde(default)]
    pub allowed_models: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyResponseCacheConfig {
    pub key_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleItem {
    pub id: String,
    pub name: String,
    pub rule_type: String,
    #[serde(default)]
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertRuleListResult {
    pub items: Vec<AlertRuleItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelItem {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    #[serde(default)]
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertChannelListResult {
    pub items: Vec<AlertChannelItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertHistoryItem {
    pub id: i64,
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub status: String,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertHistoryListResult {
    pub items: Vec<AlertHistoryItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub runtime: String,
    #[serde(default)]
    pub hook_points: Vec<String>,
    pub script_content: String,
    pub enabled: bool,
    pub timeout_ms: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginListResult {
    pub items: Vec<PluginItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelTestResult {
    pub channel_id: String,
    pub status: String,
    pub sent_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPricingItem {
    pub model_slug: String,
    pub input_price_per_1k: f64,
    pub output_price_per_1k: f64,
    #[serde(default)]
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPricingListResult {
    pub items: Vec<ModelPricingItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSummaryParams {
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub start_ts: Option<i64>,
    #[serde(default)]
    pub end_ts: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostUsageSummaryResult {
    pub request_count: i64,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSummaryKeyItem {
    pub key_id: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSummaryModelItem {
    pub model: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSummaryDayItem {
    pub day: String,
    pub request_count: i64,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSummaryResult {
    pub preset: String,
    pub range_start: i64,
    pub range_end: i64,
    pub total: CostUsageSummaryResult,
    #[serde(default)]
    pub by_key: Vec<CostSummaryKeyItem>,
    #[serde(default)]
    pub by_model: Vec<CostSummaryModelItem>,
    #[serde(default)]
    pub by_day: Vec<CostSummaryDayItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostExportResult {
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendQueryParams {
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub start_ts: Option<i64>,
    #[serde(default)]
    pub end_ts: Option<i64>,
    #[serde(default)]
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTrendItem {
    pub bucket: String,
    pub request_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTrendResult {
    pub preset: String,
    pub granularity: String,
    pub range_start: i64,
    pub range_end: i64,
    #[serde(default)]
    pub items: Vec<RequestTrendItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrendItem {
    pub model: String,
    pub request_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrendResult {
    pub preset: String,
    pub range_start: i64,
    pub range_end: i64,
    #[serde(default)]
    pub items: Vec<ModelTrendItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapCellItem {
    pub weekday: i64,
    pub hour: i64,
    pub request_count: i64,
    pub success_count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapTrendResult {
    pub preset: String,
    pub range_start: i64,
    pub range_end: i64,
    #[serde(default)]
    pub items: Vec<HeatmapCellItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecretResult {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyModelListResult {
    pub items: Vec<ModelOption>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogSummary {
    pub trace_id: Option<String>,
    pub key_id: Option<String>,
    pub account_id: Option<String>,
    pub initial_account_id: Option<String>,
    #[serde(default)]
    pub attempted_account_ids: Vec<String>,
    pub route_strategy: Option<String>,
    pub requested_model: Option<String>,
    #[serde(default)]
    pub model_fallback_path: Vec<String>,
    pub request_path: String,
    pub original_path: Option<String>,
    pub adapted_path: Option<String>,
    pub method: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub response_adapter: Option<String>,
    pub upstream_url: Option<String>,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub reasoning_output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub error: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestLogFilterParams {
    pub query: Option<String>,
    pub status_filter: Option<String>,
    pub key_id: Option<String>,
    #[serde(default)]
    pub key_ids: Vec<String>,
    pub model: Option<String>,
    pub time_from: Option<i64>,
    pub time_to: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestLogListParams {
    pub page: i64,
    pub page_size: i64,
    #[serde(flatten)]
    pub filters: RequestLogFilterParams,
}

impl Default for RequestLogListParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            filters: RequestLogFilterParams::default(),
        }
    }
}

impl RequestLogListParams {
    pub fn normalized(self) -> Self {
        Self {
            page: if self.page < 1 { 1 } else { self.page },
            page_size: if self.page_size < 1 {
                20
            } else {
                self.page_size
            },
            filters: self.filters,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogListResult {
    pub items: Vec<RequestLogSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogExportParams {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(flatten)]
    pub filters: RequestLogFilterParams,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogExportResult {
    pub format: String,
    pub file_name: String,
    pub content: String,
    pub record_count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogItem {
    pub id: i64,
    pub action: String,
    pub object_type: String,
    pub object_id: Option<String>,
    pub operator: String,
    pub changes: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditLogFilterParams {
    pub action: Option<String>,
    pub object_type: Option<String>,
    pub object_id: Option<String>,
    pub time_from: Option<i64>,
    pub time_to: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditLogListParams {
    pub page: i64,
    pub page_size: i64,
    #[serde(flatten)]
    pub filters: AuditLogFilterParams,
}

impl Default for AuditLogListParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            filters: AuditLogFilterParams::default(),
        }
    }
}

impl AuditLogListParams {
    pub fn normalized(self) -> Self {
        Self {
            page: if self.page < 1 { 1 } else { self.page },
            page_size: if self.page_size < 1 {
                20
            } else {
                self.page_size
            },
            filters: self.filters,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogListResult {
    pub items: Vec<AuditLogItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogExportParams {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(flatten)]
    pub filters: AuditLogFilterParams,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogExportResult {
    pub format: String,
    pub file_name: String,
    pub content: String,
    pub record_count: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogFilterSummaryResult {
    pub total_count: i64,
    pub filtered_count: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogTodaySummaryResult {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub today_tokens: i64,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardAccountStatusBucket {
    pub key: String,
    pub label: String,
    pub count: i64,
    pub percent: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardGatewayMetricsResult {
    pub window_minutes: i64,
    pub total_requests: i64,
    pub success_requests: i64,
    pub error_requests: i64,
    pub qps: f64,
    pub success_rate: f64,
    pub p50_latency_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub p99_latency_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardHealthResult {
    pub generated_at: i64,
    #[serde(default)]
    pub account_status_buckets: Vec<DashboardAccountStatusBucket>,
    #[serde(default)]
    pub gateway_metrics: DashboardGatewayMetricsResult,
    #[serde(default)]
    pub recent_healthcheck: Option<HealthcheckRunResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardTrendPoint {
    pub bucket_ts: i64,
    pub request_count: i64,
    pub error_count: i64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardTrendResult {
    pub generated_at: i64,
    pub bucket_minutes: i64,
    #[serde(default)]
    pub points: Vec<DashboardTrendPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckFailureAccountResult {
    pub account_id: String,
    pub label: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckRunResult {
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub total_accounts: i64,
    pub sampled_accounts: i64,
    pub success_count: i64,
    pub failure_count: i64,
    #[serde(default)]
    pub failed_accounts: Vec<HealthcheckFailureAccountResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckConfigResult {
    pub enabled: bool,
    pub interval_secs: u64,
    pub sample_size: usize,
    #[serde(default)]
    pub recent_run: Option<HealthcheckRunResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSnapshotResult {
    pub accounts: Vec<AccountSummary>,
    pub usage_snapshots: Vec<UsageSnapshotResult>,
    #[serde(default)]
    pub usage_aggregate_summary: UsageAggregateSummaryResult,
    #[serde(default)]
    pub usage_prediction_summary: UsagePredictionSummaryResult,
    #[serde(default)]
    pub failure_reason_summary: Vec<FailureReasonSummaryItem>,
    #[serde(default)]
    pub governance_summary: Vec<GovernanceSummaryItem>,
    #[serde(default)]
    pub operation_audits: Vec<OperationAuditItem>,
    pub api_keys: Vec<ApiKeySummary>,
    pub api_model_options: Vec<ModelOption>,
    pub manual_preferred_account_id: Option<String>,
    pub request_log_today_summary: RequestLogTodaySummaryResult,
    pub request_logs: Vec<RequestLogSummary>,
}

#[cfg(test)]
#[path = "tests/types_tests.rs"]
mod tests;
