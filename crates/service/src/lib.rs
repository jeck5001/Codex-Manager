use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use codexmanager_core::storage::now_ts;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

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
mod app_settings;
#[path = "auth/auth_callback.rs"]
mod auth_callback;
#[path = "auth/auth_login.rs"]
mod auth_login;
#[path = "auth/auth_tokens.rs"]
mod auth_tokens;
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
mod rpc_dispatch;
#[path = "storage/storage_helpers.rs"]
mod storage_helpers;
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
use app_settings::{
    apply_env_overrides_to_process, current_env_overrides, env_override_catalog_value,
    env_override_reserved_keys, env_override_unsupported_keys, get_persisted_app_setting,
    list_app_settings_map, open_app_settings_storage, persisted_env_overrides_only,
    reload_runtime_after_env_override_apply, save_env_overrides_value, save_persisted_app_setting,
    save_persisted_bool_setting, set_env_overrides,
};
pub use web_access::{
    build_web_access_session_token, current_web_access_password_hash, set_web_access_password,
    verify_web_access_password, web_access_password_configured, web_auth_status_value,
};

pub const DEFAULT_ADDR: &str = "localhost:48760";
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0:48760";

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static RPC_AUTH_TOKEN: OnceLock<String> = OnceLock::new();

pub mod portable {
    // 中文注释：service/web 发行物使用“同目录可选 env 文件 + 默认 DB + token 文件”机制，做到解压即用。
    pub fn bootstrap_current_process() {
        crate::process_env::load_env_from_exe_dir();
        crate::process_env::ensure_default_db_path();
        // 提前生成并落库 token，便于 web 进程/外部工具复用同一 token。
        let _ = crate::rpc_auth_token();
    }
}

pub const SERVICE_BIND_MODE_SETTING_KEY: &str = "service.bind_mode";
pub const SERVICE_BIND_MODE_LOOPBACK: &str = "loopback";
pub const SERVICE_BIND_MODE_ALL_INTERFACES: &str = "all_interfaces";

fn normalize_service_bind_mode(raw: Option<&str>) -> &'static str {
    let Some(value) = raw else {
        return SERVICE_BIND_MODE_LOOPBACK;
    };
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "all_interfaces" | "all-interfaces" | "all" | "0.0.0.0" => SERVICE_BIND_MODE_ALL_INTERFACES,
        _ => SERVICE_BIND_MODE_LOOPBACK,
    }
}

pub fn current_service_bind_mode() -> String {
    get_persisted_app_setting(SERVICE_BIND_MODE_SETTING_KEY)
        .map(|value| normalize_service_bind_mode(Some(&value)).to_string())
        .or_else(current_env_service_bind_mode)
        .unwrap_or_else(|| SERVICE_BIND_MODE_LOOPBACK.to_string())
}

pub fn set_service_bind_mode(mode: &str) -> Result<String, String> {
    let normalized = normalize_service_bind_mode(Some(mode)).to_string();
    let storage = open_app_settings_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .set_app_setting(SERVICE_BIND_MODE_SETTING_KEY, &normalized, now_ts())
        .map_err(|err| format!("save service bind mode failed: {err}"))?;
    Ok(normalized)
}

pub fn bind_all_interfaces_enabled() -> bool {
    current_service_bind_mode() == SERVICE_BIND_MODE_ALL_INTERFACES
}

pub fn default_listener_bind_addr() -> String {
    if bind_all_interfaces_enabled() {
        DEFAULT_BIND_ADDR.to_string()
    } else {
        DEFAULT_ADDR.to_string()
    }
}

// 中文注释：客户端本地探活/调用继续走 localhost；真正监听地址是否放开到 0.0.0.0 由配置控制。
pub fn listener_bind_addr(addr: &str) -> String {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return default_listener_bind_addr();
    }

    let addr = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    let addr = addr.strip_prefix("https://").unwrap_or(addr);
    let addr = addr.split('/').next().unwrap_or(addr);
    let bind_all = bind_all_interfaces_enabled();

    if !addr.contains(':') {
        return if bind_all {
            format!("0.0.0.0:{addr}")
        } else {
            format!("localhost:{addr}")
        };
    }

    let Some((host, port)) = addr.rsplit_once(':') else {
        return addr.to_string();
    };
    if host == "0.0.0.0" {
        return format!("0.0.0.0:{port}");
    }
    if host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
    {
        return if bind_all {
            format!("0.0.0.0:{port}")
        } else {
            format!("localhost:{port}")
        };
    }

    addr.to_string()
}

pub const APP_SETTING_UPDATE_AUTO_CHECK_KEY: &str = "app.update.auto_check";
pub const APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY: &str = "app.close_to_tray_on_close";
pub const APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY: &str =
    "app.lightweight_mode_on_close_to_tray";
pub const APP_SETTING_UI_LOW_TRANSPARENCY_KEY: &str = "ui.low_transparency";
pub const APP_SETTING_UI_THEME_KEY: &str = "ui.theme";
pub const APP_SETTING_SERVICE_ADDR_KEY: &str = "app.service_addr";
pub const APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY: &str = "gateway.route_strategy";
pub const APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY: &str =
    "gateway.cpa_no_cookie_header_mode";
pub const APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY: &str = "gateway.upstream_proxy_url";
pub const APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY: &str = "gateway.background_tasks";
pub const APP_SETTING_ENV_OVERRIDES_KEY: &str = "app.env_overrides";
pub const APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY: &str = "web.auth.password_hash";
pub const WEB_ACCESS_SESSION_COOKIE_NAME: &str = "codexmanager_web_auth";

const DEFAULT_UI_THEME: &str = "tech";
const VALID_UI_THEMES: &[&str] = &[
    "tech", "dark", "business", "mint", "sunset", "grape", "ocean", "forest", "rose", "slate",
    "aurora",
];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTasksInput {
    pub usage_polling_enabled: Option<bool>,
    pub usage_poll_interval_secs: Option<u64>,
    pub gateway_keepalive_enabled: Option<bool>,
    pub gateway_keepalive_interval_secs: Option<u64>,
    pub token_refresh_polling_enabled: Option<bool>,
    pub token_refresh_poll_interval_secs: Option<u64>,
    pub usage_refresh_workers: Option<usize>,
    pub http_worker_factor: Option<usize>,
    pub http_worker_min: Option<usize>,
    pub http_stream_worker_factor: Option<usize>,
    pub http_stream_worker_min: Option<usize>,
}

impl BackgroundTasksInput {
    fn into_patch(self) -> usage_refresh::BackgroundTasksSettingsPatch {
        usage_refresh::BackgroundTasksSettingsPatch {
            usage_polling_enabled: self.usage_polling_enabled,
            usage_poll_interval_secs: self.usage_poll_interval_secs,
            gateway_keepalive_enabled: self.gateway_keepalive_enabled,
            gateway_keepalive_interval_secs: self.gateway_keepalive_interval_secs,
            token_refresh_polling_enabled: self.token_refresh_polling_enabled,
            token_refresh_poll_interval_secs: self.token_refresh_poll_interval_secs,
            usage_refresh_workers: self.usage_refresh_workers,
            http_worker_factor: self.http_worker_factor,
            http_worker_min: self.http_worker_min,
            http_stream_worker_factor: self.http_stream_worker_factor,
            http_stream_worker_min: self.http_stream_worker_min,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsPatch {
    update_auto_check: Option<bool>,
    close_to_tray_on_close: Option<bool>,
    lightweight_mode_on_close_to_tray: Option<bool>,
    low_transparency: Option<bool>,
    theme: Option<String>,
    service_addr: Option<String>,
    service_listen_mode: Option<String>,
    route_strategy: Option<String>,
    cpa_no_cookie_header_mode_enabled: Option<bool>,
    upstream_proxy_url: Option<String>,
    background_tasks: Option<BackgroundTasksInput>,
    env_overrides: Option<HashMap<String, String>>,
    web_access_password: Option<String>,
}

fn parse_bool_with_default(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_ui_theme(raw: Option<&str>) -> String {
    let candidate = raw.unwrap_or(DEFAULT_UI_THEME).trim().to_ascii_lowercase();
    if VALID_UI_THEMES.iter().any(|theme| *theme == candidate) {
        candidate
    } else {
        DEFAULT_UI_THEME.to_string()
    }
}

fn normalize_saved_service_addr(raw: Option<&str>) -> Result<String, String> {
    let Some(value) = normalize_optional_text(raw) else {
        return Ok(DEFAULT_ADDR.to_string());
    };
    let value = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(&value);
    let value = value.split('/').next().unwrap_or(value).trim();
    if value.is_empty() {
        return Err("service address is empty".to_string());
    }
    if value.contains(':') {
        return Ok(value.to_string());
    }
    Ok(format!("localhost:{value}"))
}

fn current_env_service_addr() -> Option<String> {
    let raw = std::env::var("CODEXMANAGER_SERVICE_ADDR").ok()?;
    let normalized = normalize_saved_service_addr(Some(&raw)).ok()?;
    let Some((host, port)) = normalized.rsplit_once(':') else {
        return Some(normalized);
    };
    match host {
        "0.0.0.0" | "::" | "[::]" => Some(format!("localhost:{port}")),
        _ => Some(normalized),
    }
}

fn current_env_service_bind_mode() -> Option<String> {
    let raw = std::env::var("CODEXMANAGER_SERVICE_ADDR").ok()?;
    let normalized = normalize_saved_service_addr(Some(&raw)).ok()?;
    let host = normalized
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(normalized.as_str());
    let mode = match host {
        "0.0.0.0" | "::" | "[::]" => SERVICE_BIND_MODE_ALL_INTERFACES,
        _ => SERVICE_BIND_MODE_LOOPBACK,
    };
    Some(mode.to_string())
}

pub fn current_saved_service_addr() -> String {
    get_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY)
        .and_then(|value| normalize_saved_service_addr(Some(&value)).ok())
        .or_else(current_env_service_addr)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string())
}

pub fn set_saved_service_addr(addr: Option<&str>) -> Result<String, String> {
    let normalized = normalize_saved_service_addr(addr)?;
    save_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY, Some(&normalized))?;
    Ok(normalized)
}

pub fn current_update_auto_check_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_UPDATE_AUTO_CHECK_KEY)
        .map(|value| parse_bool_with_default(&value, true))
        .unwrap_or(true)
}

pub fn set_update_auto_check_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_UPDATE_AUTO_CHECK_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_close_to_tray_on_close_setting() -> bool {
    get_persisted_app_setting(APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

pub fn set_close_to_tray_on_close_setting(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_lightweight_mode_on_close_to_tray_setting() -> bool {
    get_persisted_app_setting(APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

pub fn set_lightweight_mode_on_close_to_tray_setting(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_ui_low_transparency_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_UI_LOW_TRANSPARENCY_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

pub fn set_ui_low_transparency_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_UI_LOW_TRANSPARENCY_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_ui_theme() -> String {
    normalize_ui_theme(get_persisted_app_setting(APP_SETTING_UI_THEME_KEY).as_deref())
}

pub fn set_ui_theme(theme: Option<&str>) -> Result<String, String> {
    let normalized = normalize_ui_theme(theme);
    save_persisted_app_setting(APP_SETTING_UI_THEME_KEY, Some(&normalized))?;
    Ok(normalized)
}

pub fn set_gateway_route_strategy(strategy: &str) -> Result<String, String> {
    let applied = gateway::set_route_strategy(strategy)?.to_string();
    save_persisted_app_setting(APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, Some(&applied))?;
    Ok(applied)
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

pub fn set_gateway_background_tasks(
    input: BackgroundTasksInput,
) -> Result<serde_json::Value, String> {
    let applied = usage_refresh::set_background_tasks_settings(input.into_patch());
    let raw = serde_json::to_string(&applied)
        .map_err(|err| format!("serialize background tasks failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY, Some(&raw))?;
    serde_json::to_value(applied).map_err(|err| err.to_string())
}

fn current_background_tasks_snapshot_value() -> Result<serde_json::Value, String> {
    serde_json::to_value(usage_refresh::background_tasks_settings()).map_err(|err| err.to_string())
}

pub fn sync_runtime_settings_from_storage() {
    let settings = list_app_settings_map();
    let env_overrides = persisted_env_overrides_only();
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
    if let Some(raw) = settings.get(APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY) {
        gateway::set_cpa_no_cookie_header_mode(parse_bool_with_default(raw, false));
    }
    if let Some(proxy_url) = settings.get(APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY) {
        let normalized = normalize_optional_text(Some(proxy_url));
        if let Err(err) = gateway::set_upstream_proxy_url(normalized.as_deref()) {
            log::warn!("sync persisted upstream proxy failed: {err}");
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

pub fn app_settings_get() -> Result<Value, String> {
    app_settings_get_with_overrides(None, None)
}

pub fn app_settings_get_with_overrides(
    close_to_tray_on_close: Option<bool>,
    close_to_tray_supported: Option<bool>,
) -> Result<Value, String> {
    initialize_storage_if_needed()?;
    sync_runtime_settings_from_storage();
    let background_tasks = current_background_tasks_snapshot_value()?;
    let update_auto_check = current_update_auto_check_enabled();
    let persisted_close_to_tray = current_close_to_tray_on_close_setting();
    let close_to_tray = close_to_tray_on_close.unwrap_or(persisted_close_to_tray);
    let lightweight_mode_on_close_to_tray = current_lightweight_mode_on_close_to_tray_setting();
    let low_transparency = current_ui_low_transparency_enabled();
    let theme = current_ui_theme();
    let service_addr = current_saved_service_addr();
    let service_listen_mode = current_service_bind_mode();
    let route_strategy = gateway::current_route_strategy().to_string();
    let cpa_no_cookie_header_mode_enabled = gateway::cpa_no_cookie_header_mode_enabled();
    let upstream_proxy_url = gateway::current_upstream_proxy_url();
    let background_tasks_raw = serde_json::to_string(&background_tasks)
        .map_err(|err| format!("serialize background tasks failed: {err}"))?;
    let env_overrides = current_env_overrides();

    let _ = save_persisted_bool_setting(APP_SETTING_UPDATE_AUTO_CHECK_KEY, update_auto_check);
    let _ = save_persisted_bool_setting(
        APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY,
        persisted_close_to_tray,
    );
    let _ = save_persisted_bool_setting(
        APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY,
        lightweight_mode_on_close_to_tray,
    );
    let _ = save_persisted_bool_setting(APP_SETTING_UI_LOW_TRANSPARENCY_KEY, low_transparency);
    let _ = save_persisted_app_setting(APP_SETTING_UI_THEME_KEY, Some(&theme));
    let _ = save_persisted_app_setting(APP_SETTING_SERVICE_ADDR_KEY, Some(&service_addr));
    let _ = save_persisted_app_setting(SERVICE_BIND_MODE_SETTING_KEY, Some(&service_listen_mode));
    let _ = save_persisted_app_setting(
        APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY,
        Some(&route_strategy),
    );
    let _ = save_persisted_bool_setting(
        APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
        cpa_no_cookie_header_mode_enabled,
    );
    let _ = save_persisted_app_setting(
        APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
        upstream_proxy_url.as_deref(),
    );
    let _ = save_persisted_app_setting(
        APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
        Some(&background_tasks_raw),
    );
    let _ = save_env_overrides_value(&env_overrides);

    Ok(serde_json::json!({
        "updateAutoCheck": update_auto_check,
        "closeToTrayOnClose": close_to_tray,
        "closeToTraySupported": close_to_tray_supported,
        "lightweightModeOnCloseToTray": lightweight_mode_on_close_to_tray,
        "lowTransparency": low_transparency,
        "theme": theme,
        "serviceAddr": service_addr,
        "serviceListenMode": service_listen_mode,
        "serviceListenModeOptions": [
            SERVICE_BIND_MODE_LOOPBACK,
            SERVICE_BIND_MODE_ALL_INTERFACES
        ],
        "routeStrategy": route_strategy,
        "routeStrategyOptions": ["ordered", "balanced"],
        "cpaNoCookieHeaderModeEnabled": cpa_no_cookie_header_mode_enabled,
        "upstreamProxyUrl": upstream_proxy_url.unwrap_or_default(),
        "backgroundTasks": background_tasks,
        "envOverrides": env_overrides,
        "envOverrideCatalog": env_override_catalog_value(),
        "envOverrideReservedKeys": env_override_reserved_keys(),
        "envOverrideUnsupportedKeys": env_override_unsupported_keys(),
        "webAccessPasswordConfigured": web_access_password_configured(),
    }))
}

pub fn app_settings_set(params: Option<&Value>) -> Result<Value, String> {
    initialize_storage_if_needed()?;
    let patch = match params {
        Some(value) => serde_json::from_value::<AppSettingsPatch>(value.clone())
            .map_err(|err| format!("invalid app settings payload: {err}"))?,
        None => AppSettingsPatch::default(),
    };

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
    if let Some(service_addr) = patch.service_addr {
        let _ = set_saved_service_addr(Some(&service_addr))?;
    }
    if let Some(mode) = patch.service_listen_mode {
        let _ = set_service_bind_mode(&mode)?;
    }
    if let Some(strategy) = patch.route_strategy {
        let _ = set_gateway_route_strategy(&strategy)?;
    }
    if let Some(enabled) = patch.cpa_no_cookie_header_mode_enabled {
        let _ = set_gateway_cpa_no_cookie_header_mode(enabled)?;
    }
    if let Some(proxy_url) = patch.upstream_proxy_url {
        let _ = set_gateway_upstream_proxy_url(Some(&proxy_url))?;
    }
    if let Some(background_tasks) = patch.background_tasks {
        let _ = set_gateway_background_tasks(background_tasks)?;
    }
    if let Some(env_overrides) = patch.env_overrides {
        let _ = set_env_overrides(env_overrides)?;
    }
    if let Some(password) = patch.web_access_password {
        let _ = set_web_access_password(Some(&password))?;
    }

    app_settings_get()
}

pub struct ServerHandle {
    pub addr: String,
    join: thread::JoinHandle<()>,
}

impl ServerHandle {
    pub fn join(self) {
        let _ = self.join.join();
    }
}

pub fn start_one_shot_server() -> std::io::Result<ServerHandle> {
    portable::bootstrap_current_process();
    gateway::reload_runtime_config_from_env();
    // 中文注释：one-shot 入口也先尝试建表，避免未初始化数据库在首个 RPC 就触发读写失败。
    if let Err(err) = storage_helpers::initialize_storage() {
        log::warn!("storage startup init skipped: {}", err);
    }
    sync_runtime_settings_from_storage();
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    let addr = server
        .server_addr()
        .to_ip()
        .map(|a| a.to_string())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "server addr missing"))?;
    let join = thread::spawn(move || {
        if let Some(request) = server.incoming_requests().next() {
            crate::http::backend_router::handle_backend_request(request);
        }
    });
    Ok(ServerHandle { addr, join })
}

pub fn start_server(addr: &str) -> std::io::Result<()> {
    portable::bootstrap_current_process();
    gateway::reload_runtime_config_from_env();
    // 中文注释：启动阶段先做一次显式初始化；不放在每次 open_storage 里是为避免高频 RPC 重复执行迁移检查。
    if let Err(err) = storage_helpers::initialize_storage() {
        log::warn!("storage startup init skipped: {}", err);
    }
    sync_runtime_settings_from_storage();
    usage_refresh::ensure_usage_polling();
    usage_refresh::ensure_gateway_keepalive();
    usage_refresh::ensure_token_refresh_polling();
    http::server::start_http(addr)
}

pub fn initialize_storage_if_needed() -> Result<(), String> {
    storage_helpers::initialize_storage()
}

pub fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

pub fn clear_shutdown_flag() {
    SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
}

fn build_rpc_auth_token() -> String {
    if let Some(token) = process_env::read_rpc_token_from_env_or_file() {
        std::env::set_var(process_env::ENV_RPC_TOKEN, &token);
        return token;
    }

    let generated = process_env::generate_rpc_token_hex_32bytes();
    std::env::set_var(process_env::ENV_RPC_TOKEN, &generated);

    // 中文注释：多进程启动（例如 docker compose）时，避免两个进程同时生成不同 token 并互相覆盖。
    if let Some(existing) = process_env::persist_rpc_token_if_missing(&generated) {
        std::env::set_var(process_env::ENV_RPC_TOKEN, &existing);
        return existing;
    }

    generated
}

pub fn rpc_auth_token() -> &'static str {
    RPC_AUTH_TOKEN.get_or_init(build_rpc_auth_token).as_str()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

pub fn rpc_auth_token_matches(candidate: &str) -> bool {
    let expected = rpc_auth_token();
    constant_time_eq(expected.as_bytes(), candidate.trim().as_bytes())
}

pub fn request_shutdown(addr: &str) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
    // Best-effort wakeups for both IPv4 and IPv6 loopback so whichever listener is active exits.
    let _ = send_shutdown_request(addr);
    let addr_trimmed = addr.trim();
    if addr_trimmed.len() > "localhost:".len()
        && addr_trimmed[..("localhost:".len())].eq_ignore_ascii_case("localhost:")
    {
        let port = &addr_trimmed["localhost:".len()..];
        let _ = send_shutdown_request(&format!("127.0.0.1:{port}"));
        let _ = send_shutdown_request(&format!("[::1]:{port}"));
    }
}

fn send_shutdown_request(addr: &str) -> std::io::Result<()> {
    let addr = addr.trim();
    if addr.is_empty() {
        return Ok(());
    }
    let addr = addr.strip_prefix("http://").unwrap_or(addr);
    let addr = addr.strip_prefix("https://").unwrap_or(addr);
    let addr = addr.split('/').next().unwrap_or(addr);
    let mut stream = TcpStream::connect(addr)?;
    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let request = format!("GET /__shutdown HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    Ok(())
}

pub(crate) fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    rpc_dispatch::handle_request(req)
}

#[cfg(test)]
#[path = "tests/lib_tests.rs"]
mod tests;
