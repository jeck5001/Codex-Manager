use std::sync::atomic::Ordering;

use crate::app_shell::{
    CLOSE_TO_TRAY_ON_CLOSE, KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE,
    LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY, TRAY_AVAILABLE,
};
use crate::app_storage::apply_runtime_storage_env;
use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_listen_config_get(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    apply_runtime_storage_env(&app);
    tauri::async_runtime::spawn_blocking(move || {
        Ok(serde_json::json!({
            "mode": codexmanager_service::current_service_bind_mode(),
            "options": [
                codexmanager_service::SERVICE_BIND_MODE_LOOPBACK,
                codexmanager_service::SERVICE_BIND_MODE_ALL_INTERFACES
            ],
            "requiresRestart": true,
        }))
    })
    .await
    .map_err(|err| format!("service_listen_config_get task failed: {err}"))?
}

#[tauri::command]
pub async fn service_listen_config_set(
    app: tauri::AppHandle,
    mode: String,
) -> Result<serde_json::Value, String> {
    apply_runtime_storage_env(&app);
    tauri::async_runtime::spawn_blocking(move || {
        codexmanager_service::set_service_bind_mode(&mode).map(|applied| {
            serde_json::json!({
                "mode": applied,
                "options": [
                    codexmanager_service::SERVICE_BIND_MODE_LOOPBACK,
                    codexmanager_service::SERVICE_BIND_MODE_ALL_INTERFACES
                ],
                "requiresRestart": true,
            })
        })
    })
    .await
    .map_err(|err| format!("service_listen_config_set task failed: {err}"))?
}

#[tauri::command]
pub async fn service_gateway_route_strategy_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/routeStrategy/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_route_strategy_set(
    addr: Option<String>,
    strategy: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "strategy": strategy });
    rpc_call_in_background("gateway/routeStrategy/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_gateway_manual_account_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/manualAccount/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_manual_account_set(
    addr: Option<String>,
    account_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountId": account_id });
    rpc_call_in_background("gateway/manualAccount/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_gateway_manual_account_clear(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/manualAccount/clear", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_header_policy_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/headerPolicy/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_header_policy_set(
    addr: Option<String>,
    cpa_no_cookie_header_mode_enabled: bool,
) -> Result<serde_json::Value, String> {
    let params =
        serde_json::json!({ "cpaNoCookieHeaderModeEnabled": cpa_no_cookie_header_mode_enabled });
    rpc_call_in_background("gateway/headerPolicy/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_gateway_background_tasks_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/backgroundTasks/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_background_tasks_set(
    addr: Option<String>,
    usage_polling_enabled: Option<bool>,
    usage_poll_interval_secs: Option<u64>,
    gateway_keepalive_enabled: Option<bool>,
    gateway_keepalive_interval_secs: Option<u64>,
    token_refresh_polling_enabled: Option<bool>,
    token_refresh_poll_interval_secs: Option<u64>,
    usage_refresh_workers: Option<u64>,
    http_worker_factor: Option<u64>,
    http_worker_min: Option<u64>,
    http_stream_worker_factor: Option<u64>,
    http_stream_worker_min: Option<u64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
      "usagePollingEnabled": usage_polling_enabled,
      "usagePollIntervalSecs": usage_poll_interval_secs,
      "gatewayKeepaliveEnabled": gateway_keepalive_enabled,
      "gatewayKeepaliveIntervalSecs": gateway_keepalive_interval_secs,
      "tokenRefreshPollingEnabled": token_refresh_polling_enabled,
      "tokenRefreshPollIntervalSecs": token_refresh_poll_interval_secs,
      "usageRefreshWorkers": usage_refresh_workers,
      "httpWorkerFactor": http_worker_factor,
      "httpWorkerMin": http_worker_min,
      "httpStreamWorkerFactor": http_stream_worker_factor,
      "httpStreamWorkerMin": http_stream_worker_min
    });
    rpc_call_in_background("gateway/backgroundTasks/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_gateway_upstream_proxy_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/upstreamProxy/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_upstream_proxy_set(
    addr: Option<String>,
    proxy_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "proxyUrl": proxy_url });
    rpc_call_in_background("gateway/upstreamProxy/set", addr, Some(params)).await
}

#[tauri::command]
pub fn app_close_to_tray_on_close_get(app: tauri::AppHandle) -> bool {
    apply_runtime_storage_env(&app);
    if let Ok(mut settings) = codexmanager_service::app_settings_get() {
        sync_window_runtime_state_from_settings(&mut settings);
    }
    CLOSE_TO_TRAY_ON_CLOSE.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn app_close_to_tray_on_close_set(app: tauri::AppHandle, enabled: bool) -> bool {
    apply_runtime_storage_env(&app);
    let payload = serde_json::json!({
        "closeToTrayOnClose": enabled
    });
    if let Ok(mut settings) = codexmanager_service::app_settings_set(Some(&payload)) {
        sync_window_runtime_state_from_settings(&mut settings);
    }
    CLOSE_TO_TRAY_ON_CLOSE.load(Ordering::Relaxed)
}

#[tauri::command]
pub async fn app_settings_get(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    apply_runtime_storage_env(&app);
    let mut settings = tauri::async_runtime::spawn_blocking(move || {
        codexmanager_service::app_settings_get_with_overrides(
            Some(
                codexmanager_service::current_close_to_tray_on_close_setting()
                    && TRAY_AVAILABLE.load(Ordering::Relaxed),
            ),
            Some(TRAY_AVAILABLE.load(Ordering::Relaxed)),
        )
    })
    .await
    .map_err(|err| format!("app_settings_get task failed: {err}"))??;
    sync_window_runtime_state_from_settings(&mut settings);
    Ok(settings)
}

#[tauri::command]
pub async fn app_settings_set(
    app: tauri::AppHandle,
    patch: serde_json::Value,
) -> Result<serde_json::Value, String> {
    apply_runtime_storage_env(&app);
    let mut settings = tauri::async_runtime::spawn_blocking(move || {
        codexmanager_service::app_settings_set(Some(&patch))
    })
    .await
    .map_err(|err| format!("app_settings_set task failed: {err}"))??;
    sync_window_runtime_state_from_settings(&mut settings);
    Ok(settings)
}

pub fn effective_lightweight_mode_on_close_to_tray(
    requested: bool,
    close_to_tray_effective: bool,
) -> bool {
    requested && close_to_tray_effective
}

pub fn sync_window_runtime_state_from_settings(settings: &mut serde_json::Value) {
    let requested_close_to_tray = settings
        .get("closeToTrayOnClose")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let supported = settings
        .get("closeToTraySupported")
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| TRAY_AVAILABLE.load(Ordering::Relaxed));
    let requested_lightweight_mode = settings
        .get("lightweightModeOnCloseToTray")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let effective_close_to_tray = requested_close_to_tray && supported;
    let effective_lightweight_mode = effective_lightweight_mode_on_close_to_tray(
        requested_lightweight_mode,
        effective_close_to_tray,
    );
    if let Some(object) = settings.as_object_mut() {
        object.insert(
            "closeToTrayOnClose".to_string(),
            serde_json::json!(effective_close_to_tray),
        );
        object.insert(
            "closeToTraySupported".to_string(),
            serde_json::json!(supported),
        );
        object.insert(
            "lightweightModeOnCloseToTray".to_string(),
            serde_json::json!(requested_lightweight_mode),
        );
    }
    CLOSE_TO_TRAY_ON_CLOSE.store(effective_close_to_tray, Ordering::Relaxed);
    LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY.store(effective_lightweight_mode, Ordering::Relaxed);
    if !effective_lightweight_mode {
        KEEP_ALIVE_FOR_LIGHTWEIGHT_CLOSE.store(false, Ordering::Relaxed);
    }
}
