use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

#[path = "account/account_availability.rs"]
mod account_availability;
#[path = "account/account_cleanup.rs"]
mod account_cleanup;
#[path = "account/account_delete.rs"]
mod account_delete;
#[path = "account/account_delete_many.rs"]
mod account_delete_many;
#[path = "account/account_export.rs"]
mod account_export;
#[path = "account/account_import.rs"]
mod account_import;
#[path = "account/account_list.rs"]
mod account_list;
#[path = "account/account_status.rs"]
mod account_status;
#[path = "account/account_update.rs"]
mod account_update;
#[path = "apikey/apikey_create.rs"]
mod apikey_create;
#[path = "apikey/apikey_delete.rs"]
mod apikey_delete;
#[path = "apikey/apikey_disable.rs"]
mod apikey_disable;
#[path = "apikey/apikey_enable.rs"]
mod apikey_enable;
#[path = "apikey/apikey_list.rs"]
mod apikey_list;
#[path = "apikey/apikey_models.rs"]
mod apikey_models;
#[path = "apikey/apikey_profile.rs"]
mod apikey_profile;
#[path = "apikey/apikey_read_secret.rs"]
mod apikey_read_secret;
#[path = "apikey/apikey_update_model.rs"]
mod apikey_update_model;
pub(crate) mod app_settings;
#[path = "auth/auth_callback.rs"]
mod auth_callback;
#[path = "auth/auth_login.rs"]
mod auth_login;
#[path = "auth/auth_tokens.rs"]
mod auth_tokens;
mod bootstrap;
mod error_codes;
mod gateway;
mod http;
mod lock_utils;
pub mod process_env;
mod reasoning_effort;
#[path = "requestlog/requestlog_clear.rs"]
mod requestlog_clear;
#[path = "requestlog/requestlog_list.rs"]
mod requestlog_list;
#[path = "requestlog/requestlog_today_summary.rs"]
mod requestlog_today_summary;
pub(crate) mod rpc_auth;
mod rpc_dispatch;
mod shutdown;
#[path = "storage/storage_helpers.rs"]
mod storage_helpers;
mod startup;
#[path = "usage/usage_account_meta.rs"]
mod usage_account_meta;
#[path = "usage/usage_http.rs"]
mod usage_http;
#[path = "usage/usage_keepalive.rs"]
mod usage_keepalive;
#[path = "usage/usage_list.rs"]
mod usage_list;
#[path = "usage/usage_read.rs"]
mod usage_read;
#[path = "usage/usage_refresh.rs"]
mod usage_refresh;
#[path = "usage/usage_scheduler.rs"]
mod usage_scheduler;
#[path = "usage/usage_snapshot_store.rs"]
mod usage_snapshot_store;
#[path = "usage/usage_token_refresh.rs"]
mod usage_token_refresh;
mod web_access;

pub use app_settings::{
    app_settings_get, app_settings_get_with_overrides, app_settings_set,
    bind_all_interfaces_enabled, current_close_to_tray_on_close_setting,
    current_lightweight_mode_on_close_to_tray_setting, current_saved_service_addr,
    current_service_bind_mode, current_ui_low_transparency_enabled, current_ui_theme,
    current_update_auto_check_enabled, default_listener_bind_addr, listener_bind_addr,
    set_close_to_tray_on_close_setting, set_gateway_background_tasks,
    set_gateway_cpa_no_cookie_header_mode, set_gateway_route_strategy,
    set_gateway_upstream_proxy_url, set_lightweight_mode_on_close_to_tray_setting,
    set_saved_service_addr, set_service_bind_mode, set_ui_low_transparency_enabled,
    set_ui_theme, set_update_auto_check_enabled, sync_runtime_settings_from_storage,
    BackgroundTasksInput, APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY,
    APP_SETTING_ENV_OVERRIDES_KEY, APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
    APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
    APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
    APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY, APP_SETTING_SERVICE_ADDR_KEY,
    APP_SETTING_UI_LOW_TRANSPARENCY_KEY, APP_SETTING_UI_THEME_KEY,
    APP_SETTING_UPDATE_AUTO_CHECK_KEY, APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY,
    DEFAULT_ADDR, DEFAULT_BIND_ADDR, SERVICE_BIND_MODE_ALL_INTERFACES,
    SERVICE_BIND_MODE_LOOPBACK, SERVICE_BIND_MODE_SETTING_KEY,
    WEB_ACCESS_SESSION_COOKIE_NAME,
};
pub use bootstrap::{initialize_storage_if_needed, portable};
pub use rpc_auth::{rpc_auth_token, rpc_auth_token_matches};
pub use shutdown::{clear_shutdown_flag, request_shutdown, shutdown_requested};
pub use startup::{start_one_shot_server, start_server, ServerHandle};
pub use web_access::{
    build_web_access_session_token, current_web_access_password_hash, set_web_access_password,
    verify_web_access_password, web_access_password_configured, web_auth_status_value,
};

pub(crate) fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    rpc_dispatch::handle_request(req)
}

#[cfg(test)]
#[path = "tests/lib_tests.rs"]
mod tests;
