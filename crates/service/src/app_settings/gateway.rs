use crate::gateway;
use crate::usage_refresh;
use serde::Deserialize;

use super::{
    normalize_optional_text, save_persisted_app_setting, save_persisted_bool_setting,
    APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY, APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
    APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY, APP_SETTING_GATEWAY_MODEL_ALIAS_POOLS_JSON_KEY,
    APP_SETTING_GATEWAY_ORIGINATOR_KEY, APP_SETTING_GATEWAY_PAYLOAD_REWRITE_RULES_JSON_KEY,
    APP_SETTING_GATEWAY_QUOTA_PROTECTION_ENABLED_KEY,
    APP_SETTING_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT_KEY,
    APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY,
    APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY, APP_SETTING_GATEWAY_RESPONSE_CACHE_ENABLED_KEY,
    APP_SETTING_GATEWAY_RESPONSE_CACHE_MAX_ENTRIES_KEY,
    APP_SETTING_GATEWAY_RESPONSE_CACHE_TTL_SECS_KEY,
    APP_SETTING_GATEWAY_RETRY_POLICY_BACKOFF_STRATEGY_KEY,
    APP_SETTING_GATEWAY_RETRY_POLICY_MAX_RETRIES_KEY,
    APP_SETTING_GATEWAY_RETRY_POLICY_RETRYABLE_STATUS_CODES_KEY,
    APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY,
    APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY, APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTasksInput {
    pub usage_polling_enabled: Option<bool>,
    pub usage_poll_interval_secs: Option<u64>,
    pub gateway_keepalive_enabled: Option<bool>,
    pub gateway_keepalive_interval_secs: Option<u64>,
    pub token_refresh_polling_enabled: Option<bool>,
    pub token_refresh_poll_interval_secs: Option<u64>,
    pub session_probe_polling_enabled: Option<bool>,
    pub session_probe_interval_secs: Option<u64>,
    pub session_probe_sample_size: Option<usize>,
    pub usage_refresh_workers: Option<usize>,
    pub http_worker_factor: Option<usize>,
    pub http_worker_min: Option<usize>,
    pub http_stream_worker_factor: Option<usize>,
    pub http_stream_worker_min: Option<usize>,
    pub auto_register_pool_enabled: Option<bool>,
    pub auto_register_ready_account_count: Option<usize>,
    pub auto_register_ready_remain_percent: Option<u64>,
    pub auto_disable_risky_accounts_enabled: Option<bool>,
    pub auto_disable_risky_accounts_failure_threshold: Option<usize>,
    pub auto_disable_risky_accounts_health_score_threshold: Option<usize>,
    pub auto_disable_risky_accounts_lookback_mins: Option<u64>,
    pub account_cooldown_auth_secs: Option<u64>,
    pub account_cooldown_rate_limited_secs: Option<u64>,
    pub account_cooldown_server_error_secs: Option<u64>,
    pub account_cooldown_network_secs: Option<u64>,
    pub account_cooldown_low_quota_secs: Option<u64>,
    pub account_cooldown_deactivated_secs: Option<u64>,
}

impl BackgroundTasksInput {
    pub(crate) fn into_patch(self) -> usage_refresh::BackgroundTasksSettingsPatch {
        usage_refresh::BackgroundTasksSettingsPatch {
            usage_polling_enabled: self.usage_polling_enabled,
            usage_poll_interval_secs: self.usage_poll_interval_secs,
            gateway_keepalive_enabled: self.gateway_keepalive_enabled,
            gateway_keepalive_interval_secs: self.gateway_keepalive_interval_secs,
            token_refresh_polling_enabled: self.token_refresh_polling_enabled,
            token_refresh_poll_interval_secs: self.token_refresh_poll_interval_secs,
            session_probe_polling_enabled: self.session_probe_polling_enabled,
            session_probe_interval_secs: self.session_probe_interval_secs,
            session_probe_sample_size: self.session_probe_sample_size,
            usage_refresh_workers: self.usage_refresh_workers,
            http_worker_factor: self.http_worker_factor,
            http_worker_min: self.http_worker_min,
            http_stream_worker_factor: self.http_stream_worker_factor,
            http_stream_worker_min: self.http_stream_worker_min,
            auto_register_pool_enabled: self.auto_register_pool_enabled,
            auto_register_ready_account_count: self.auto_register_ready_account_count,
            auto_register_ready_remain_percent: self.auto_register_ready_remain_percent,
            auto_disable_risky_accounts_enabled: self.auto_disable_risky_accounts_enabled,
            auto_disable_risky_accounts_failure_threshold: self
                .auto_disable_risky_accounts_failure_threshold,
            auto_disable_risky_accounts_health_score_threshold: self
                .auto_disable_risky_accounts_health_score_threshold,
            auto_disable_risky_accounts_lookback_mins: self
                .auto_disable_risky_accounts_lookback_mins,
            account_cooldown_auth_secs: self.account_cooldown_auth_secs,
            account_cooldown_rate_limited_secs: self.account_cooldown_rate_limited_secs,
            account_cooldown_server_error_secs: self.account_cooldown_server_error_secs,
            account_cooldown_network_secs: self.account_cooldown_network_secs,
            account_cooldown_low_quota_secs: self.account_cooldown_low_quota_secs,
            account_cooldown_deactivated_secs: self.account_cooldown_deactivated_secs,
        }
    }
}

pub fn set_gateway_route_strategy(strategy: &str) -> Result<String, String> {
    let applied = gateway::set_route_strategy(strategy)?.to_string();
    save_persisted_app_setting(APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, Some(&applied))?;
    Ok(applied)
}

pub fn set_gateway_free_account_max_model(model: &str) -> Result<String, String> {
    let applied = gateway::set_free_account_max_model(model)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY,
        Some(&applied),
    )?;
    Ok(applied)
}

pub fn current_gateway_free_account_max_model() -> String {
    gateway::current_free_account_max_model()
}

pub fn set_gateway_quota_protection_enabled(enabled: bool) -> Result<bool, String> {
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
        if enabled { "1" } else { "0" },
    );
    crate::gateway::invalidate_candidate_cache();
    save_persisted_bool_setting(APP_SETTING_GATEWAY_QUOTA_PROTECTION_ENABLED_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_gateway_quota_protection_enabled() -> bool {
    crate::account_availability::current_quota_protection_enabled()
}

pub fn set_gateway_quota_protection_threshold_percent(value: u64) -> Result<u64, String> {
    if value > 100 {
        return Err("quotaProtectionThresholdPercent must be between 0 and 100".to_string());
    }
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        value.to_string(),
    );
    crate::gateway::invalidate_candidate_cache();
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT_KEY,
        Some(&value.to_string()),
    )?;
    Ok(value)
}

pub fn current_gateway_quota_protection_threshold_percent() -> u64 {
    crate::account_availability::current_quota_protection_threshold_percent().min(100)
}

pub fn set_gateway_request_compression_enabled(enabled: bool) -> Result<bool, String> {
    let applied = gateway::set_request_compression_enabled(enabled);
    save_persisted_bool_setting(APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY, applied)?;
    Ok(applied)
}

pub fn current_gateway_request_compression_enabled() -> bool {
    gateway::request_compression_enabled()
}

pub fn set_gateway_payload_rewrite_rules_json(raw: Option<&str>) -> Result<String, String> {
    let applied = gateway::set_payload_rewrite_rules_json(raw)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_PAYLOAD_REWRITE_RULES_JSON_KEY,
        Some(&applied),
    )?;
    Ok(applied)
}

pub fn current_gateway_payload_rewrite_rules_json() -> String {
    gateway::current_payload_rewrite_rules_json()
}

pub fn set_gateway_model_alias_pools_json(raw: Option<&str>) -> Result<String, String> {
    let applied = gateway::set_model_alias_pools_json(raw)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_MODEL_ALIAS_POOLS_JSON_KEY,
        Some(&applied),
    )?;
    Ok(applied)
}

pub fn current_gateway_model_alias_pools_json() -> String {
    gateway::current_model_alias_pools_json()
}

pub fn set_gateway_response_cache_enabled(enabled: bool) -> Result<bool, String> {
    let applied = gateway::set_response_cache_enabled(enabled);
    save_persisted_bool_setting(APP_SETTING_GATEWAY_RESPONSE_CACHE_ENABLED_KEY, applied)?;
    Ok(applied)
}

pub fn current_gateway_response_cache_enabled() -> bool {
    gateway::current_response_cache_config().enabled
}

pub fn set_gateway_response_cache_ttl_secs(ttl_secs: u64) -> Result<u64, String> {
    let applied = gateway::set_response_cache_ttl_secs(ttl_secs)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RESPONSE_CACHE_TTL_SECS_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

pub fn current_gateway_response_cache_ttl_secs() -> u64 {
    gateway::current_response_cache_ttl_secs()
}

pub fn set_gateway_response_cache_max_entries(max_entries: usize) -> Result<usize, String> {
    let applied = gateway::set_response_cache_max_entries(max_entries)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RESPONSE_CACHE_MAX_ENTRIES_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

pub fn current_gateway_response_cache_max_entries() -> usize {
    gateway::current_response_cache_max_entries()
}

pub fn set_gateway_retry_policy(
    max_retries: usize,
    backoff_strategy: &str,
    retryable_status_codes: Vec<u16>,
) -> Result<gateway::RetryPolicySnapshot, String> {
    let applied = gateway::set_retry_policy(max_retries, backoff_strategy, retryable_status_codes)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RETRY_POLICY_MAX_RETRIES_KEY,
        Some(&applied.max_retries.to_string()),
    )?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RETRY_POLICY_BACKOFF_STRATEGY_KEY,
        Some(applied.backoff_strategy.as_str()),
    )?;
    let retryable_status_codes_json = serde_json::to_string(&applied.retryable_status_codes)
        .map_err(|err| format!("serialize retry policy status codes failed: {err}"))?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RETRY_POLICY_RETRYABLE_STATUS_CODES_KEY,
        Some(&retryable_status_codes_json),
    )?;
    Ok(applied)
}

pub fn current_gateway_retry_policy() -> gateway::RetryPolicySnapshot {
    gateway::current_retry_policy()
}

pub fn set_gateway_originator(originator: &str) -> Result<String, String> {
    let applied = gateway::set_originator(originator)?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_ORIGINATOR_KEY, Some(&applied))?;
    Ok(applied)
}

pub fn current_gateway_originator() -> String {
    gateway::current_originator()
}

pub fn set_gateway_residency_requirement(value: Option<&str>) -> Result<Option<String>, String> {
    let normalized = normalize_optional_text(value);
    let applied = gateway::set_residency_requirement(normalized.as_deref())?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY,
        applied.as_deref(),
    )?;
    Ok(applied)
}

pub fn current_gateway_residency_requirement() -> Option<String> {
    gateway::current_residency_requirement()
}

pub fn residency_requirement_options() -> &'static [&'static str] {
    &["", "us"]
}

pub fn set_gateway_cpa_no_cookie_header_mode(enabled: bool) -> Result<bool, String> {
    let applied = gateway::set_cpa_no_cookie_header_mode(enabled);
    save_persisted_bool_setting(APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY, applied)?;
    Ok(applied)
}

pub fn set_gateway_upstream_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    let normalized = normalize_optional_text(proxy_url);
    let applied = gateway::set_upstream_proxy_url(normalized.as_deref())?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
        applied.as_deref(),
    )?;
    Ok(applied)
}

pub fn set_gateway_upstream_stream_timeout_ms(timeout_ms: u64) -> Result<u64, String> {
    let applied = gateway::set_upstream_stream_timeout_ms(timeout_ms);
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

pub fn current_gateway_upstream_stream_timeout_ms() -> u64 {
    gateway::current_upstream_stream_timeout_ms()
}

pub fn set_gateway_sse_keepalive_interval_ms(interval_ms: u64) -> Result<u64, String> {
    let applied = gateway::set_sse_keepalive_interval_ms(interval_ms)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

pub fn current_gateway_sse_keepalive_interval_ms() -> u64 {
    gateway::current_sse_keepalive_interval_ms()
}

pub fn set_gateway_background_tasks(
    input: BackgroundTasksInput,
) -> Result<serde_json::Value, String> {
    let applied = usage_refresh::set_background_tasks_settings(input.into_patch());
    let raw = serde_json::to_string(&applied)
        .map_err(|err| format!("serialize background tasks failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY, Some(&raw))?;
    serde_json::to_value(applied).map_err(|err| err.to_string())
}

pub(crate) fn current_background_tasks_snapshot_value() -> Result<serde_json::Value, String> {
    serde_json::to_value(usage_refresh::background_tasks_settings()).map_err(|err| err.to_string())
}
