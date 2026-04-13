pub const APP_SETTING_UPDATE_AUTO_CHECK_KEY: &str = "app.update.auto_check";
pub const APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY: &str = "app.close_to_tray_on_close";
pub const APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY: &str =
    "app.lightweight_mode_on_close_to_tray";
pub const APP_SETTING_UI_LOW_TRANSPARENCY_KEY: &str = "ui.low_transparency";
pub const APP_SETTING_UI_THEME_KEY: &str = "ui.theme";
pub const APP_SETTING_UI_APPEARANCE_PRESET_KEY: &str = "ui.appearance_preset";
pub const APP_SETTING_UI_VISIBLE_MENU_ITEMS_KEY: &str = "ui.visible_menu_items";
pub const APP_SETTING_SERVICE_ADDR_KEY: &str = "app.service_addr";
pub const APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY: &str = "gateway.route_strategy";
pub const APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY: &str = "gateway.free_account_max_model";
pub const APP_SETTING_GATEWAY_QUOTA_PROTECTION_ENABLED_KEY: &str =
    "gateway.quota_protection_enabled";
pub const APP_SETTING_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT_KEY: &str =
    "gateway.quota_protection_threshold_percent";
pub const APP_SETTING_GATEWAY_NEW_ACCOUNT_PROTECTION_DAYS_KEY: &str =
    "gateway.new_account_protection_days";
pub const APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY: &str =
    "gateway.request_compression_enabled";
pub const APP_SETTING_GATEWAY_RETRY_POLICY_MAX_RETRIES_KEY: &str =
    "gateway.retry_policy.max_retries";
pub const APP_SETTING_GATEWAY_RETRY_POLICY_BACKOFF_STRATEGY_KEY: &str =
    "gateway.retry_policy.backoff_strategy";
pub const APP_SETTING_GATEWAY_RETRY_POLICY_RETRYABLE_STATUS_CODES_KEY: &str =
    "gateway.retry_policy.retryable_status_codes";
pub const APP_SETTING_GATEWAY_RESPONSE_CACHE_ENABLED_KEY: &str = "gateway.response_cache_enabled";
pub const APP_SETTING_GATEWAY_RESPONSE_CACHE_TTL_SECS_KEY: &str = "gateway.response_cache_ttl_secs";
pub const APP_SETTING_GATEWAY_RESPONSE_CACHE_MAX_ENTRIES_KEY: &str =
    "gateway.response_cache_max_entries";
pub const APP_SETTING_GATEWAY_ORIGINATOR_KEY: &str = "gateway.originator";
pub const APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY: &str = "gateway.residency_requirement";
pub const APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY: &str =
    "gateway.cpa_no_cookie_header_mode";
pub const APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY: &str = "gateway.upstream_proxy_url";
pub const APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY: &str =
    "gateway.upstream_stream_timeout_ms";
pub const APP_SETTING_GATEWAY_PAYLOAD_REWRITE_RULES_JSON_KEY: &str =
    "gateway.payload_rewrite_rules_json";
pub const APP_SETTING_GATEWAY_MODEL_ALIAS_POOLS_JSON_KEY: &str = "gateway.model_alias_pools_json";
pub const APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY: &str =
    "gateway.sse_keepalive_interval_ms";
pub const APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY: &str = "gateway.background_tasks";
pub const APP_SETTING_ENV_OVERRIDES_KEY: &str = "app.env_overrides";
pub const APP_SETTING_MCP_ENABLED_KEY: &str = "mcp.enabled";
pub const APP_SETTING_MCP_PORT_KEY: &str = "mcp.port";
pub const APP_SETTING_REMOTE_MANAGEMENT_ENABLED_KEY: &str = "remote_management.enabled";
pub const APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY: &str = "remote_management.secret_hash";
pub const APP_SETTING_TEAM_MANAGER_ENABLED_KEY: &str = "team_manager.enabled";
pub const APP_SETTING_TEAM_MANAGER_API_URL_KEY: &str = "team_manager.api_url";
pub const APP_SETTING_TEAM_MANAGER_API_KEY_KEY: &str = "team_manager.api_key";
pub const APP_SETTING_CPA_SYNC_ENABLED_KEY: &str = "cpa_sync.enabled";
pub const APP_SETTING_CPA_SYNC_API_URL_KEY: &str = "cpa_sync.api_url";
pub const APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY: &str = "cpa_sync.management_key";
pub const APP_SETTING_ACCOUNT_PAYMENT_STATE_KEY: &str = "account.payment_state";
pub const APP_SETTING_ACCOUNT_SESSION_STATE_KEY: &str = "account.session_state";
pub const APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY: &str = "web.auth.password_hash";
pub const APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY: &str = "web.auth.2fa.secret_encrypted";
pub const APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY: &str = "web.auth.2fa.recovery_codes";
pub const WEB_ACCESS_SESSION_COOKIE_NAME: &str = "codexmanager_web_auth";

pub(crate) fn parse_bool_with_default(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

pub(crate) fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
