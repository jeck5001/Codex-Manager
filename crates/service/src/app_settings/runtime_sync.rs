use crate::gateway;
use crate::usage_refresh;

use super::{
    apply_env_overrides_to_process, list_app_settings_map, normalize_optional_text,
    parse_bool_with_default, persisted_env_overrides_missing_process_env,
    reload_runtime_after_env_override_apply, set_service_bind_mode, BackgroundTasksInput,
    APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY, APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
    APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY, APP_SETTING_GATEWAY_ORIGINATOR_KEY,
    APP_SETTING_GATEWAY_PAYLOAD_REWRITE_RULES_JSON_KEY,
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
    SERVICE_BIND_MODE_SETTING_KEY,
};

pub fn sync_runtime_settings_from_storage() {
    let settings = list_app_settings_map();
    let env_overrides = persisted_env_overrides_missing_process_env();
    if !env_overrides.is_empty() {
        apply_env_overrides_to_process(&env_overrides, &env_overrides);
    }
    reload_runtime_after_env_override_apply();

    if let Some(mode) = settings.get(SERVICE_BIND_MODE_SETTING_KEY) {
        let _ = set_service_bind_mode(mode);
    }
    if let Some(strategy) = settings.get(APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY) {
        if let Some(strategy) = normalize_optional_text(Some(strategy)) {
            if let Err(err) = gateway::set_route_strategy(&strategy) {
                log::warn!("sync persisted route strategy failed: {err}");
            }
        }
    }
    if let Some(model) = settings.get(APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY) {
        if let Some(model) = normalize_optional_text(Some(model)) {
            if let Err(err) = gateway::set_free_account_max_model(&model) {
                log::warn!("sync persisted free account max model failed: {err}");
            }
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_QUOTA_PROTECTION_ENABLED_KEY) {
        std::env::set_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
            if parse_bool_with_default(raw, false) {
                "1"
            } else {
                "0"
            },
        );
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT_KEY) {
        if let Ok(value) = raw.trim().parse::<u64>() {
            std::env::set_var(
                crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
                value.min(100).to_string(),
            );
        } else {
            log::warn!("parse persisted quota protection threshold failed: {raw}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY) {
        gateway::set_request_compression_enabled(parse_bool_with_default(raw, true));
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_PAYLOAD_REWRITE_RULES_JSON_KEY) {
        if let Err(err) = gateway::set_payload_rewrite_rules_json(Some(raw)) {
            log::warn!("sync persisted payload rewrite rules failed: {err}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_RESPONSE_CACHE_ENABLED_KEY) {
        gateway::set_response_cache_enabled(parse_bool_with_default(raw, false));
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_RESPONSE_CACHE_TTL_SECS_KEY) {
        if let Ok(ttl_secs) = raw.trim().parse::<u64>() {
            if let Err(err) = gateway::set_response_cache_ttl_secs(ttl_secs) {
                log::warn!("sync persisted response cache ttl failed: {err}");
            }
        } else {
            log::warn!("parse persisted response cache ttl failed: {raw}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_RESPONSE_CACHE_MAX_ENTRIES_KEY) {
        if let Ok(max_entries) = raw.trim().parse::<usize>() {
            if let Err(err) = gateway::set_response_cache_max_entries(max_entries) {
                log::warn!("sync persisted response cache max entries failed: {err}");
            }
        } else {
            log::warn!("parse persisted response cache max entries failed: {raw}");
        }
    }
    let retry_policy_max_retries = settings
        .get(APP_SETTING_GATEWAY_RETRY_POLICY_MAX_RETRIES_KEY)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or_else(gateway::retry_policy_max_retries);
    let retry_policy_backoff_strategy = settings
        .get(APP_SETTING_GATEWAY_RETRY_POLICY_BACKOFF_STRATEGY_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| gateway::current_retry_policy().backoff_strategy);
    let retry_policy_retryable_status_codes = settings
        .get(APP_SETTING_GATEWAY_RETRY_POLICY_RETRYABLE_STATUS_CODES_KEY)
        .and_then(|raw| serde_json::from_str::<Vec<u16>>(raw).ok())
        .unwrap_or_else(|| gateway::current_retry_policy().retryable_status_codes);
    if let Err(err) = gateway::set_retry_policy(
        retry_policy_max_retries,
        &retry_policy_backoff_strategy,
        retry_policy_retryable_status_codes,
    ) {
        log::warn!("sync persisted retry policy failed: {err}");
    }
    if let Some(originator) = settings.get(APP_SETTING_GATEWAY_ORIGINATOR_KEY) {
        if let Some(originator) = normalize_optional_text(Some(originator)) {
            if let Err(err) = gateway::set_originator(&originator) {
                log::warn!("sync persisted gateway originator failed: {err}");
            }
        }
    }
    if let Some(residency_requirement) = settings.get(APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY)
    {
        let normalized = normalize_optional_text(Some(residency_requirement));
        if let Err(err) = gateway::set_residency_requirement(normalized.as_deref()) {
            log::warn!("sync persisted gateway residency requirement failed: {err}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY) {
        gateway::set_cpa_no_cookie_header_mode(parse_bool_with_default(raw, false));
    }
    if let Some(proxy_url) = settings.get(APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY) {
        let normalized = normalize_optional_text(Some(proxy_url));
        if let Err(err) = gateway::set_upstream_proxy_url(normalized.as_deref()) {
            log::warn!("sync persisted upstream proxy failed: {err}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY) {
        if let Ok(timeout_ms) = raw.trim().parse::<u64>() {
            gateway::set_upstream_stream_timeout_ms(timeout_ms);
        } else {
            log::warn!("parse persisted upstream stream timeout failed: {raw}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY) {
        if let Ok(interval_ms) = raw.trim().parse::<u64>() {
            if let Err(err) = gateway::set_sse_keepalive_interval_ms(interval_ms) {
                log::warn!("sync persisted sse keepalive interval failed: {err}");
            }
        } else {
            log::warn!("parse persisted sse keepalive interval failed: {raw}");
        }
    }
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY) {
        match serde_json::from_str::<BackgroundTasksInput>(raw) {
            Ok(input) => {
                usage_refresh::set_background_tasks_settings(input.into_patch());
            }
            Err(err) => {
                log::warn!("parse persisted background tasks failed: {err}");
            }
        }
    }
}
