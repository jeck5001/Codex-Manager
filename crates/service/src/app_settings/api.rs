use crate::initialize_storage_if_needed;
use crate::web_access_password_configured;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use super::{
    current_close_to_tray_on_close_setting, current_env_overrides,
    current_lightweight_mode_on_close_to_tray_setting, current_saved_service_addr,
    current_service_bind_mode, current_ui_low_transparency_enabled, current_ui_theme,
    current_update_auto_check_enabled, env_override_catalog_value, env_override_reserved_keys,
    env_override_unsupported_keys, save_env_overrides_value, save_persisted_app_setting,
    save_persisted_bool_setting, set_close_to_tray_on_close_setting, set_env_overrides,
    set_gateway_background_tasks, set_gateway_cpa_no_cookie_header_mode,
    set_gateway_route_strategy, set_gateway_upstream_proxy_url, set_lightweight_mode_on_close_to_tray_setting,
    set_saved_service_addr, set_service_bind_mode, set_ui_low_transparency_enabled, set_ui_theme,
    set_update_auto_check_enabled, sync_runtime_settings_from_storage, BackgroundTasksInput,
    APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY, APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
    APP_SETTING_GATEWAY_CPA_NO_COOKIE_HEADER_MODE_KEY,
    APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
    APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY, APP_SETTING_SERVICE_ADDR_KEY,
    APP_SETTING_UI_LOW_TRANSPARENCY_KEY, APP_SETTING_UI_THEME_KEY,
    APP_SETTING_UPDATE_AUTO_CHECK_KEY, SERVICE_BIND_MODE_ALL_INTERFACES,
    SERVICE_BIND_MODE_LOOPBACK, SERVICE_BIND_MODE_SETTING_KEY,
};
use super::gateway::current_background_tasks_snapshot_value;

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
    let route_strategy = crate::gateway::current_route_strategy().to_string();
    let cpa_no_cookie_header_mode_enabled = crate::gateway::cpa_no_cookie_header_mode_enabled();
    let upstream_proxy_url = crate::gateway::current_upstream_proxy_url();
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
        let _ = crate::set_web_access_password(Some(&password))?;
    }

    app_settings_get()
}
