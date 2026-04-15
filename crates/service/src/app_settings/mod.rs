mod api;
mod env_overrides;
mod gateway;
mod mcp;
mod remote_management;
mod runtime_sync;
mod service;
mod shared;
mod store;
mod ui;

pub use api::{app_settings_get, app_settings_get_with_overrides, app_settings_set};
pub(crate) use env_overrides::{
    apply_env_overrides_to_process, current_env_overrides,
    persisted_env_overrides_missing_process_env, reload_runtime_after_env_override_apply,
    set_env_overrides,
};
pub use gateway::{
    current_gateway_free_account_max_model, current_gateway_model_alias_pools_json,
    current_gateway_new_account_protection_days, current_gateway_originator,
    current_gateway_payload_rewrite_rules_json, current_gateway_quota_protection_enabled,
    current_gateway_quota_protection_threshold_percent,
    current_gateway_request_compression_enabled, current_gateway_residency_requirement,
    current_gateway_response_cache_enabled, current_gateway_response_cache_max_entries,
    current_gateway_response_cache_ttl_secs, current_gateway_retry_policy,
    current_gateway_sse_keepalive_interval_ms, current_gateway_upstream_stream_timeout_ms,
    residency_requirement_options, set_gateway_background_tasks,
    set_gateway_cpa_no_cookie_header_mode, set_gateway_free_account_max_model,
    set_gateway_model_alias_pools_json, set_gateway_new_account_protection_days,
    set_gateway_originator, set_gateway_payload_rewrite_rules_json,
    set_gateway_quota_protection_enabled, set_gateway_quota_protection_threshold_percent,
    set_gateway_request_compression_enabled, set_gateway_residency_requirement,
    set_gateway_response_cache_enabled, set_gateway_response_cache_max_entries,
    set_gateway_response_cache_ttl_secs, set_gateway_retry_policy, set_gateway_route_strategy,
    set_gateway_sse_keepalive_interval_ms, set_gateway_upstream_proxy_url,
    set_gateway_upstream_stream_timeout_ms, BackgroundTasksInput,
};
pub use mcp::{
    current_mcp_enabled, current_mcp_port, set_mcp_enabled, set_mcp_port, DEFAULT_MCP_PORT,
};
pub use remote_management::{current_remote_management_enabled, set_remote_management_enabled};
pub use runtime_sync::sync_runtime_settings_from_storage;
pub use service::{
    bind_all_interfaces_enabled, current_saved_service_addr, current_service_bind_mode,
    default_listener_bind_addr, listener_bind_addr, set_saved_service_addr, set_service_bind_mode,
    DEFAULT_ADDR, DEFAULT_BIND_ADDR, SERVICE_BIND_MODE_ALL_INTERFACES, SERVICE_BIND_MODE_LOOPBACK,
    SERVICE_BIND_MODE_SETTING_KEY,
};
pub(crate) use shared::{normalize_optional_text, parse_bool_with_default};
pub use shared::{
    APP_SETTING_ACCOUNT_PAYMENT_STATE_KEY, APP_SETTING_ACCOUNT_SESSION_STATE_KEY,
    APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY, APP_SETTING_CPA_SYNC_API_URL_KEY,
    APP_SETTING_CPA_SYNC_ENABLED_KEY, APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY,
    APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY,
    APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY, APP_SETTING_ENV_OVERRIDES_KEY,
    APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
    APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
    APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY, APP_SETTING_GATEWAY_MODEL_ALIAS_POOLS_JSON_KEY,
    APP_SETTING_GATEWAY_NEW_ACCOUNT_PROTECTION_DAYS_KEY, APP_SETTING_GATEWAY_ORIGINATOR_KEY,
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
    APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY, APP_SETTING_MCP_ENABLED_KEY,
    APP_SETTING_MCP_PORT_KEY, APP_SETTING_REMOTE_MANAGEMENT_ENABLED_KEY,
    APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY, APP_SETTING_SERVICE_ADDR_KEY,
    APP_SETTING_TEAM_MANAGER_API_KEY_KEY, APP_SETTING_TEAM_MANAGER_API_URL_KEY,
    APP_SETTING_TEAM_MANAGER_ENABLED_KEY, APP_SETTING_UI_APPEARANCE_PRESET_KEY,
    APP_SETTING_UI_LOW_TRANSPARENCY_KEY, APP_SETTING_UI_THEME_KEY,
    APP_SETTING_UI_VISIBLE_MENU_ITEMS_KEY, APP_SETTING_UPDATE_AUTO_CHECK_KEY,
    APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY, APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY,
    APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY, WEB_ACCESS_SESSION_COOKIE_NAME,
};
pub(crate) use store::{
    get_persisted_app_setting, list_app_settings_map, save_persisted_app_setting,
    save_persisted_bool_setting,
};
pub use ui::{
    current_close_to_tray_on_close_setting, current_lightweight_mode_on_close_to_tray_setting,
    current_ui_appearance_preset, current_ui_low_transparency_enabled, current_ui_theme,
    current_ui_visible_menu_items, current_update_auto_check_enabled,
    set_close_to_tray_on_close_setting, set_lightweight_mode_on_close_to_tray_setting,
    set_ui_appearance_preset, set_ui_low_transparency_enabled, set_ui_theme,
    set_ui_visible_menu_items, set_update_auto_check_enabled,
};
