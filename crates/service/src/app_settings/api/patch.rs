use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use super::{
    save_persisted_app_setting, save_persisted_bool_setting, set_close_to_tray_on_close_setting,
    set_env_overrides, set_gateway_background_tasks, set_gateway_cpa_no_cookie_header_mode,
    set_gateway_free_account_max_model, set_gateway_model_alias_pools_json,
    set_gateway_new_account_protection_days, set_gateway_originator,
    set_gateway_payload_rewrite_rules_json, set_gateway_quota_protection_enabled,
    set_gateway_quota_protection_threshold_percent, set_gateway_request_compression_enabled,
    set_gateway_residency_requirement, set_gateway_response_cache_enabled,
    set_gateway_response_cache_max_entries, set_gateway_response_cache_ttl_secs,
    set_gateway_retry_policy, set_gateway_route_strategy, set_gateway_sse_keepalive_interval_ms,
    set_gateway_upstream_proxy_url, set_gateway_upstream_stream_timeout_ms,
    set_lightweight_mode_on_close_to_tray_setting, set_mcp_enabled, set_mcp_port,
    set_remote_management_enabled, set_saved_service_addr, set_service_bind_mode,
    set_ui_appearance_preset, set_ui_low_transparency_enabled, set_ui_theme,
    set_update_auto_check_enabled, BackgroundTasksInput,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AppSettingsPatch {
    update_auto_check: Option<bool>,
    close_to_tray_on_close: Option<bool>,
    lightweight_mode_on_close_to_tray: Option<bool>,
    low_transparency: Option<bool>,
    theme: Option<String>,
    appearance_preset: Option<String>,
    service_addr: Option<String>,
    service_listen_mode: Option<String>,
    mcp_enabled: Option<bool>,
    mcp_port: Option<u16>,
    remote_management_enabled: Option<bool>,
    route_strategy: Option<String>,
    free_account_max_model: Option<String>,
    new_account_protection_days: Option<u64>,
    quota_protection_enabled: Option<bool>,
    quota_protection_threshold_percent: Option<u64>,
    request_compression_enabled: Option<bool>,
    payload_rewrite_rules_json: Option<String>,
    model_alias_pools_json: Option<String>,
    retry_policy_max_retries: Option<usize>,
    retry_policy_backoff_strategy: Option<String>,
    retry_policy_retryable_status_codes: Option<Vec<u16>>,
    response_cache_enabled: Option<bool>,
    response_cache_ttl_secs: Option<u64>,
    response_cache_max_entries: Option<usize>,
    gateway_originator: Option<String>,
    gateway_residency_requirement: Option<String>,
    cpa_no_cookie_header_mode_enabled: Option<bool>,
    upstream_proxy_url: Option<String>,
    upstream_stream_timeout_ms: Option<u64>,
    sse_keepalive_interval_ms: Option<u64>,
    background_tasks: Option<BackgroundTasksInput>,
    env_overrides: Option<HashMap<String, String>>,
    team_manager_enabled: Option<bool>,
    team_manager_api_url: Option<String>,
    team_manager_api_key: Option<String>,
    web_access_password: Option<String>,
    remote_management_secret: Option<String>,
}

pub(super) fn parse_app_settings_patch(params: Option<&Value>) -> Result<AppSettingsPatch, String> {
    match params {
        Some(value) => serde_json::from_value::<AppSettingsPatch>(value.clone())
            .map_err(|err| format!("invalid app settings payload: {err}")),
        None => Ok(AppSettingsPatch::default()),
    }
}

pub(super) fn apply_app_settings_patch(patch: AppSettingsPatch) -> Result<(), String> {
    if let Some(enabled) = patch.update_auto_check {
        set_update_auto_check_enabled(enabled)?;
    }
    if let Some(enabled) = patch.close_to_tray_on_close {
        set_close_to_tray_on_close_setting(enabled)?;
    }
    if let Some(enabled) = patch.lightweight_mode_on_close_to_tray {
        set_lightweight_mode_on_close_to_tray_setting(enabled)?;
    }
    if let Some(enabled) = patch.low_transparency {
        set_ui_low_transparency_enabled(enabled)?;
    }
    if let Some(theme) = patch.theme {
        let _ = set_ui_theme(Some(&theme))?;
    }
    if let Some(preset) = patch.appearance_preset {
        let _ = set_ui_appearance_preset(Some(&preset))?;
    }
    if let Some(service_addr) = patch.service_addr {
        let _ = set_saved_service_addr(Some(&service_addr))?;
    }
    if let Some(mode) = patch.service_listen_mode {
        let _ = set_service_bind_mode(&mode)?;
    }
    if let Some(enabled) = patch.mcp_enabled {
        let _ = set_mcp_enabled(enabled)?;
    }
    if let Some(port) = patch.mcp_port {
        let _ = set_mcp_port(port)?;
    }
    if let Some(secret) = patch.remote_management_secret {
        let _ = crate::set_remote_management_secret(Some(&secret))?;
    }
    if let Some(enabled) = patch.remote_management_enabled {
        if enabled && !crate::remote_management_secret_configured() {
            return Err("启用远程管理 API 前请先设置访问密钥".to_string());
        }
        let _ = set_remote_management_enabled(enabled)?;
    } else if crate::current_remote_management_enabled()
        && !crate::remote_management_secret_configured()
    {
        return Err("远程管理 API 已启用，不能在未关闭前清空访问密钥".to_string());
    }
    if let Some(strategy) = patch.route_strategy {
        let _ = set_gateway_route_strategy(&strategy)?;
    }
    if let Some(model) = patch.free_account_max_model {
        let _ = set_gateway_free_account_max_model(&model)?;
    }
    if let Some(value) = patch.new_account_protection_days {
        let _ = set_gateway_new_account_protection_days(value)?;
    }
    if let Some(enabled) = patch.quota_protection_enabled {
        let _ = set_gateway_quota_protection_enabled(enabled)?;
    }
    if let Some(value) = patch.quota_protection_threshold_percent {
        let _ = set_gateway_quota_protection_threshold_percent(value)?;
    }
    if let Some(enabled) = patch.request_compression_enabled {
        let _ = set_gateway_request_compression_enabled(enabled)?;
    }
    if let Some(raw) = patch.payload_rewrite_rules_json {
        let _ = set_gateway_payload_rewrite_rules_json(Some(&raw))?;
    }
    if let Some(raw) = patch.model_alias_pools_json {
        let _ = set_gateway_model_alias_pools_json(Some(&raw))?;
    }
    if patch.retry_policy_max_retries.is_some()
        || patch.retry_policy_backoff_strategy.is_some()
        || patch.retry_policy_retryable_status_codes.is_some()
    {
        let current = crate::current_gateway_retry_policy();
        let _ = set_gateway_retry_policy(
            patch
                .retry_policy_max_retries
                .unwrap_or(current.max_retries),
            patch
                .retry_policy_backoff_strategy
                .as_deref()
                .unwrap_or(current.backoff_strategy.as_str()),
            patch
                .retry_policy_retryable_status_codes
                .unwrap_or(current.retryable_status_codes),
        )?;
    }
    if let Some(enabled) = patch.response_cache_enabled {
        let _ = set_gateway_response_cache_enabled(enabled)?;
    }
    if let Some(ttl_secs) = patch.response_cache_ttl_secs {
        let _ = set_gateway_response_cache_ttl_secs(ttl_secs)?;
    }
    if let Some(max_entries) = patch.response_cache_max_entries {
        let _ = set_gateway_response_cache_max_entries(max_entries)?;
    }
    if let Some(originator) = patch.gateway_originator {
        let _ = set_gateway_originator(&originator)?;
    }
    if let Some(residency_requirement) = patch.gateway_residency_requirement {
        let _ = set_gateway_residency_requirement(Some(&residency_requirement))?;
    }
    if let Some(enabled) = patch.cpa_no_cookie_header_mode_enabled {
        let _ = set_gateway_cpa_no_cookie_header_mode(enabled)?;
    }
    if let Some(proxy_url) = patch.upstream_proxy_url {
        let _ = set_gateway_upstream_proxy_url(Some(&proxy_url))?;
    }
    if let Some(timeout_ms) = patch.upstream_stream_timeout_ms {
        let _ = set_gateway_upstream_stream_timeout_ms(timeout_ms)?;
    }
    if let Some(interval_ms) = patch.sse_keepalive_interval_ms {
        let _ = set_gateway_sse_keepalive_interval_ms(interval_ms)?;
    }
    if let Some(background_tasks) = patch.background_tasks {
        let _ = set_gateway_background_tasks(background_tasks)?;
    }
    if let Some(env_overrides) = patch.env_overrides {
        let _ = set_env_overrides(env_overrides)?;
    }
    if let Some(enabled) = patch.team_manager_enabled {
        save_persisted_bool_setting(crate::APP_SETTING_TEAM_MANAGER_ENABLED_KEY, enabled)?;
    }
    if let Some(api_url) = patch.team_manager_api_url {
        save_persisted_app_setting(crate::APP_SETTING_TEAM_MANAGER_API_URL_KEY, Some(&api_url))?;
    }
    if let Some(api_key) = patch.team_manager_api_key {
        save_persisted_app_setting(crate::APP_SETTING_TEAM_MANAGER_API_KEY_KEY, Some(&api_key))?;
    }
    if let Some(password) = patch.web_access_password {
        let _ = crate::set_web_access_password(Some(&password))?;
    }

    Ok(())
}
