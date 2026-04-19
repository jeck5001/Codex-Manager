use codexmanager_core::storage::now_ts;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Mutex, Once, OnceLock};
use std::thread;
use std::time::Duration;

use crate::{
    app_settings::{
        get_persisted_app_setting, parse_bool_with_default, APP_SETTING_CPA_SYNC_API_URL_KEY,
        APP_SETTING_CPA_SYNC_ENABLED_KEY, APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY,
        APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY,
        APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY,
    },
    storage_helpers::open_storage,
};

const CPA_AUTH_FILES_PATH: &str = "/v0/management/auth-files";
const CPA_AUTH_FILES_DOWNLOAD_PATH: &str = "/v0/management/auth-files/download";
const CPA_HTTP_TIMEOUT_SECS: u64 = 30;
const CPA_SAVED_KEY_SENTINEL: &str = "use_saved_key";
const DEFAULT_CPA_SYNC_INTERVAL_MINUTES: u64 = 30;

#[derive(Debug, Clone, Default)]
struct CpaSyncSettings {
    api_url: String,
    management_key: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CpaConnectionPayload {
    api_url: Option<String>,
    management_key: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CpaAuthFile {
    name: String,
    source: Option<String>,
    runtime_only: bool,
    item: Value,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CpaSyncResult {
    total_files: usize,
    eligible_files: usize,
    downloaded_files: usize,
    created: usize,
    updated: usize,
    failed: usize,
    imported_account_ids: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CpaSyncStatus {
    status: String,
    schedule_enabled: bool,
    interval_minutes: u64,
    is_running: bool,
    last_trigger: String,
    last_started_at: Option<i64>,
    last_finished_at: Option<i64>,
    last_success_at: Option<i64>,
    last_summary: String,
    last_error: String,
    next_run_at: Option<i64>,
}

impl Default for CpaSyncStatus {
    fn default() -> Self {
        Self {
            status: "disabled".to_string(),
            schedule_enabled: false,
            interval_minutes: DEFAULT_CPA_SYNC_INTERVAL_MINUTES,
            is_running: false,
            last_trigger: String::new(),
            last_started_at: None,
            last_finished_at: None,
            last_success_at: None,
            last_summary: String::new(),
            last_error: String::new(),
            next_run_at: None,
        }
    }
}

#[derive(Debug, Clone)]
struct CpaSyncRuntimeState {
    status: String,
    schedule_enabled: bool,
    interval_minutes: u64,
    is_running: bool,
    last_trigger: String,
    last_started_at: Option<i64>,
    last_finished_at: Option<i64>,
    last_success_at: Option<i64>,
    last_summary: String,
    last_error: String,
    next_run_at: Option<i64>,
}

impl CpaSyncRuntimeState {
    fn into_status(self) -> CpaSyncStatus {
        CpaSyncStatus {
            status: self.status,
            schedule_enabled: self.schedule_enabled,
            interval_minutes: self.interval_minutes,
            is_running: self.is_running,
            last_trigger: self.last_trigger,
            last_started_at: self.last_started_at,
            last_finished_at: self.last_finished_at,
            last_success_at: self.last_success_at,
            last_summary: self.last_summary,
            last_error: self.last_error,
            next_run_at: self.next_run_at,
        }
    }
}

impl Default for CpaSyncRuntimeState {
    fn default() -> Self {
        Self {
            status: "disabled".to_string(),
            schedule_enabled: false,
            interval_minutes: DEFAULT_CPA_SYNC_INTERVAL_MINUTES,
            is_running: false,
            last_trigger: String::new(),
            last_started_at: None,
            last_finished_at: None,
            last_success_at: None,
            last_summary: String::new(),
            last_error: String::new(),
            next_run_at: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CpaSyncScheduleConfig {
    source_enabled: bool,
    schedule_enabled: bool,
    interval_minutes: u64,
    api_url: String,
    has_management_key: bool,
}

static CPA_SYNC_RUNTIME: OnceLock<Mutex<CpaSyncRuntimeState>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
struct ImportSummary {
    created: usize,
    updated: usize,
    failed: usize,
    errors: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct CpaSyncRunGuard;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CpaConnectionResult {
    success: bool,
    message: String,
    total_files: usize,
}

fn cpa_sync_runtime() -> &'static Mutex<CpaSyncRuntimeState> {
    CPA_SYNC_RUNTIME.get_or_init(|| Mutex::new(CpaSyncRuntimeState::default()))
}

fn with_cpa_sync_runtime<T>(f: impl FnOnce(&mut CpaSyncRuntimeState) -> T) -> T {
    let mut state = cpa_sync_runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut state)
}

fn read_cpa_sync_status() -> CpaSyncStatus {
    with_cpa_sync_runtime(|state| state.clone().into_status())
}

fn interval_seconds(interval_minutes: u64) -> i64 {
    interval_minutes.max(1) as i64 * 60
}

fn schedule_next_run_at(state: &mut CpaSyncRuntimeState, from_ts: i64) {
    if state.schedule_enabled {
        state.next_run_at = Some(from_ts + interval_seconds(state.interval_minutes));
    } else {
        state.next_run_at = None;
    }
}

fn begin_cpa_sync_run(trigger: &str) -> Result<CpaSyncRunGuard, String> {
    with_cpa_sync_runtime(|state| {
        if state.is_running {
            return Err("CPA 同步正在执行中，请稍后再试".to_string());
        }

        state.is_running = true;
        state.last_trigger = trigger.to_string();
        state.last_started_at = Some(now_ts());
        state.last_error.clear();
        state.status = "running".to_string();
        if trigger == "scheduled" {
            state.next_run_at = None;
        }
        Ok(CpaSyncRunGuard)
    })
}

impl Drop for CpaSyncRunGuard {
    fn drop(&mut self) {
        with_cpa_sync_runtime(|state| {
            state.is_running = false;
            if state.status == "running" {
                state.status = if state.schedule_enabled {
                    "idle".to_string()
                } else {
                    "disabled".to_string()
                };
            }
        });
    }
}

fn cpa_sync_summary_text(result: &CpaSyncResult) -> String {
    format!(
        "总文件 {}，可导入 {}，新增 {}，更新 {}，失败 {}",
        result.total_files, result.eligible_files, result.created, result.updated, result.failed
    )
}

fn mark_cpa_sync_success(result: &CpaSyncResult) {
    let finished_at = now_ts();
    with_cpa_sync_runtime(|state| {
        state.last_finished_at = Some(finished_at);
        state.last_success_at = Some(finished_at);
        state.last_summary = cpa_sync_summary_text(result);
        state.last_error.clear();
        state.status = "idle".to_string();
        schedule_next_run_at(state, finished_at);
    });
}

fn mark_cpa_sync_error(err: &str) {
    let finished_at = now_ts();
    with_cpa_sync_runtime(|state| {
        state.last_finished_at = Some(finished_at);
        state.last_error = err.to_string();
        state.status = "error".to_string();
        schedule_next_run_at(state, finished_at);
    });
}

fn load_cpa_sync_schedule_config() -> CpaSyncScheduleConfig {
    let source_enabled = get_persisted_app_setting(APP_SETTING_CPA_SYNC_ENABLED_KEY)
        .map(|raw| parse_bool_with_default(&raw, false))
        .unwrap_or(false);
    let schedule_enabled = get_persisted_app_setting(APP_SETTING_CPA_SYNC_SCHEDULE_ENABLED_KEY)
        .map(|raw| parse_bool_with_default(&raw, false))
        .unwrap_or(false);
    let interval_minutes =
        get_persisted_app_setting(APP_SETTING_CPA_SYNC_SCHEDULE_INTERVAL_MINUTES_KEY)
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .map(|value| value.max(1))
            .unwrap_or(DEFAULT_CPA_SYNC_INTERVAL_MINUTES);
    let api_url = get_persisted_app_setting(APP_SETTING_CPA_SYNC_API_URL_KEY).unwrap_or_default();
    let has_management_key = get_persisted_app_setting(APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY)
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false);

    CpaSyncScheduleConfig {
        source_enabled,
        schedule_enabled,
        interval_minutes,
        api_url,
        has_management_key,
    }
}

fn apply_cpa_sync_schedule_config(config: &CpaSyncScheduleConfig) {
    with_cpa_sync_runtime(|state| {
        let was_active = state.schedule_enabled;
        let interval_changed = state.interval_minutes != config.interval_minutes.max(1);
        let active_enabled = config.source_enabled && config.schedule_enabled;

        state.schedule_enabled = active_enabled;
        state.interval_minutes = config.interval_minutes.max(1);

        if !active_enabled {
            state.status = "disabled".to_string();
            state.next_run_at = None;
            state.last_error.clear();
            return;
        }

        if config.api_url.trim().is_empty() || !config.has_management_key {
            state.status = "misconfigured".to_string();
            state.last_error = "CPA API URL 或 Management Key 未配置".to_string();
            state.next_run_at = None;
            return;
        }

        if state.is_running {
            state.status = "running".to_string();
            return;
        }

        state.status = "idle".to_string();
        if !was_active || interval_changed || state.next_run_at.is_none() {
            schedule_next_run_at(state, now_ts());
        }
    });
}

fn should_trigger_scheduled_sync() -> bool {
    with_cpa_sync_runtime(|state| {
        state.schedule_enabled
            && !state.is_running
            && matches!(state.status.as_str(), "idle" | "error")
            && state
                .next_run_at
                .map(|next_run_at| next_run_at <= now_ts())
                .unwrap_or(false)
    })
}

fn cpa_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(CPA_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("build CPA client failed: {err}"))
}

fn normalize_api_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("CPA API URL 未配置".to_string());
    }
    let parsed = Url::parse(trimmed).map_err(|_| "CPA API URL 格式非法".to_string())?;
    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

fn normalize_management_key(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != CPA_SAVED_KEY_SENTINEL)
}

fn resolve_cpa_settings(payload: Option<&Value>) -> Result<CpaSyncSettings, String> {
    let parsed: CpaConnectionPayload = payload
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|err| format!("invalid CPA payload: {err}"))?
        .unwrap_or_default();

    let api_url = parsed
        .api_url
        .and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .or_else(|| get_persisted_app_setting(APP_SETTING_CPA_SYNC_API_URL_KEY))
        .unwrap_or_default();
    let api_url = normalize_api_url(&api_url)?;

    let management_key = normalize_management_key(parsed.management_key)
        .or_else(|| {
            normalize_management_key(get_persisted_app_setting(
                APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY,
            ))
        })
        .unwrap_or_default();
    if management_key.is_empty() {
        return Err("CPA Management Key 未配置".to_string());
    }

    Ok(CpaSyncSettings {
        api_url,
        management_key,
    })
}

fn build_cpa_endpoint(base: &str, path: &str) -> Result<Url, String> {
    let base = Url::parse(base).map_err(|_| "CPA API URL 格式非法".to_string())?;
    base.join(path.trim_start_matches('/'))
        .map_err(|err| format!("build CPA endpoint failed: {err}"))
}

fn with_cpa_auth(request: RequestBuilder, management_key: &str) -> RequestBuilder {
    request
        .header("Authorization", format!("Bearer {management_key}"))
        .header("X-Management-Key", management_key)
}

fn read_response_text(response: reqwest::blocking::Response) -> String {
    response.text().unwrap_or_default()
}

fn http_error_message(status: StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| value.get("detail").and_then(Value::as_str))
                .or_else(|| value.get("error").and_then(Value::as_str))
                .map(ToString::to_string)
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| body.trim().chars().take(200).collect::<String>());
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            if detail.is_empty() {
                "CPA Management Key 无效或没有权限".to_string()
            } else {
                format!("CPA Management Key 无效或没有权限: {detail}")
            }
        }
        _ => {
            if detail.is_empty() {
                format!("CPA 请求失败: HTTP {}", status.as_u16())
            } else {
                format!("CPA 请求失败: HTTP {} - {detail}", status.as_u16())
            }
        }
    }
}

fn array_from_container(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => Some(items.clone()),
        Value::Object(map) => {
            for key in ["files", "items", "data", "authFiles", "auth_files", "list"] {
                if let Some(found) = map.get(key).and_then(array_from_container) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn first_string_field(item: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| item.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn parse_auth_files(payload: Value) -> Result<Vec<CpaAuthFile>, String> {
    let items = array_from_container(&payload)
        .ok_or_else(|| "CPA auth-files 响应结构不兼容".to_string())?;
    Ok(items
        .into_iter()
        .enumerate()
        .map(|(index, item)| CpaAuthFile {
            name: first_string_field(&item, &["name", "filename", "fileName", "id", "fileId"])
                .unwrap_or_else(|| format!("auth-file-{}", index + 1)),
            source: first_string_field(&item, &["source", "sourceType", "type"]),
            runtime_only: item
                .get("runtime_only")
                .or_else(|| item.get("runtimeOnly"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            item,
        })
        .collect())
}

fn fetch_cpa_auth_files(settings: &CpaSyncSettings) -> Result<Vec<CpaAuthFile>, String> {
    let url = build_cpa_endpoint(&settings.api_url, CPA_AUTH_FILES_PATH)?;
    let response = with_cpa_auth(cpa_http_client()?.get(url), &settings.management_key)
        .send()
        .map_err(|err| format!("CPA 连接失败: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(http_error_message(status, &read_response_text(response)));
    }
    let payload = response
        .json::<Value>()
        .map_err(|err| format!("invalid CPA auth-files response: {err}"))?;
    parse_auth_files(payload)
}

fn metadata_blob(item: &Value) -> String {
    let mut parts = Vec::new();
    for key in [
        "name", "filename", "fileName", "provider", "service", "type", "source", "label", "email",
        "issuer",
    ] {
        if let Some(value) = item.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_ascii_lowercase());
            }
        }
    }
    parts.join(" ")
}

fn is_runtime_only_source(file: &CpaAuthFile) -> bool {
    if file.runtime_only {
        return true;
    }
    file.source
        .as_deref()
        .map(|value| value.trim().to_ascii_lowercase().contains("runtime"))
        .unwrap_or(false)
}

fn is_downloadable_file_source(file: &CpaAuthFile) -> bool {
    file.source
        .as_deref()
        .map(|value| value.trim().eq_ignore_ascii_case("file"))
        .unwrap_or(false)
}

fn looks_like_target_file(file: &CpaAuthFile) -> bool {
    let metadata = format!(
        "{} {}",
        file.name.to_ascii_lowercase(),
        metadata_blob(&file.item)
    );
    ["openai", "chatgpt", "codex"]
        .iter()
        .any(|needle| metadata.contains(needle))
}

fn has_token_field(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn item_looks_importable(item: &Value) -> bool {
    let tokens = item.get("tokens").unwrap_or(item);
    let has_access_token =
        has_token_field(tokens, "access_token") || has_token_field(tokens, "accessToken");
    if !has_access_token {
        return false;
    }
    has_token_field(tokens, "id_token")
        || has_token_field(tokens, "idToken")
        || has_token_field(tokens, "refresh_token")
        || has_token_field(tokens, "refreshToken")
        || has_token_field(tokens, "session_token")
        || has_token_field(tokens, "sessionToken")
        || has_token_field(tokens, "cookie")
        || has_token_field(tokens, "cookies")
        || metadata_blob(item).contains("openai")
        || metadata_blob(item).contains("chatgpt")
        || metadata_blob(item).contains("codex")
}

fn resolve_download_url(settings: &CpaSyncSettings, file: &CpaAuthFile) -> Result<Url, String> {
    let mut canonical_url = build_cpa_endpoint(&settings.api_url, CPA_AUTH_FILES_DOWNLOAD_PATH)?;
    canonical_url
        .query_pairs_mut()
        .append_pair("name", &file.name);
    Ok(canonical_url)
}

fn resolve_legacy_download_url(
    settings: &CpaSyncSettings,
    file: &CpaAuthFile,
) -> Result<Option<Url>, String> {
    if let Some(raw) = first_string_field(
        &file.item,
        &[
            "downloadUrl",
            "download_url",
            "downloadURL",
            "url",
            "download",
            "downloadPath",
            "download_path",
            "path",
        ],
    ) {
        if let Ok(url) = Url::parse(&raw) {
            return Ok(Some(url));
        }
        let base = Url::parse(&settings.api_url).map_err(|_| "CPA API URL 格式非法".to_string())?;
        return base
            .join(raw.trim_start_matches('/'))
            .map(Some)
            .map_err(|err| format!("build CPA download url failed: {err}"));
    }
    Ok(None)
}

fn resolve_download_fallback_url(
    settings: &CpaSyncSettings,
    file: &CpaAuthFile,
) -> Result<Url, String> {
    let mut url = build_cpa_endpoint(&settings.api_url, CPA_AUTH_FILES_DOWNLOAD_PATH)?;
    url.query_pairs_mut().append_pair("filename", &file.name);
    Ok(url)
}

fn download_auth_file(settings: &CpaSyncSettings, file: &CpaAuthFile) -> Result<String, String> {
    let client = cpa_http_client()?;
    let primary_url = resolve_download_url(settings, file)?;
    let response = with_cpa_auth(client.get(primary_url.clone()), &settings.management_key)
        .send()
        .map_err(|err| format!("下载失败: {err}"))?;
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        if let Some(legacy_url) = resolve_legacy_download_url(settings, file)? {
            let legacy_response = with_cpa_auth(client.get(legacy_url), &settings.management_key)
                .send()
                .map_err(|err| format!("下载失败: {err}"))?;
            let legacy_status = legacy_response.status();
            if legacy_status.is_success() {
                return legacy_response
                    .text()
                    .map_err(|err| format!("读取下载内容失败: {err}"));
            }
        }
        let fallback_url = resolve_download_fallback_url(settings, file)?;
        let fallback_response = with_cpa_auth(client.get(fallback_url), &settings.management_key)
            .send()
            .map_err(|err| format!("下载失败: {err}"))?;
        let fallback_status = fallback_response.status();
        if fallback_status.is_success() {
            return fallback_response
                .text()
                .map_err(|err| format!("读取下载内容失败: {err}"));
        }
        return Err(http_error_message(
            fallback_status,
            &read_response_text(fallback_response),
        ));
    }
    if !status.is_success() {
        return Err(http_error_message(status, &read_response_text(response)));
    }
    response
        .text()
        .map_err(|err| format!("读取下载内容失败: {err}"))
}

fn parse_auth_file_content(content: &str) -> Result<Vec<Value>, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).map_err(|err| format!("invalid JSON array: {err}"));
    }

    if let Ok(single) = serde_json::from_str::<Value>(trimmed) {
        return match single {
            Value::Array(items) => Ok(items),
            Value::Object(map) => {
                if let Some(items) = array_from_container(&Value::Object(map.clone())) {
                    Ok(items)
                } else {
                    Ok(vec![Value::Object(map)])
                }
            }
            other => Ok(vec![other]),
        };
    }

    let mut out = Vec::new();
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    for value in stream {
        out.push(value.map_err(|err| format!("invalid JSON object stream: {err}"))?);
    }
    Ok(out)
}

fn filtered_import_items(items: Vec<Value>, metadata_match: bool) -> Vec<Value> {
    items
        .into_iter()
        .filter(|item| metadata_match || item_looks_importable(item))
        .collect()
}

fn serialize_import_payload(items: &[Value]) -> Result<String, String> {
    serde_json::to_string(items).map_err(|err| format!("serialize auth file payload failed: {err}"))
}

fn import_payloads(payloads: Vec<String>) -> Result<ImportSummary, String> {
    if payloads.is_empty() {
        return Ok(ImportSummary::default());
    }
    let result = crate::account_import::import_account_auth_json(payloads)?;
    let value = serde_json::to_value(result)
        .map_err(|err| format!("serialize import result failed: {err}"))?;

    Ok(ImportSummary {
        created: value.get("created").and_then(Value::as_u64).unwrap_or(0) as usize,
        updated: value.get("updated").and_then(Value::as_u64).unwrap_or(0) as usize,
        failed: value.get("failed").and_then(Value::as_u64).unwrap_or(0) as usize,
        errors: value
            .get("errors")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.get("message")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                            .or_else(|| item.as_str().map(ToString::to_string))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

fn collect_recent_imported_account_ids(started_at: i64) -> Vec<String> {
    let Some(storage) = open_storage() else {
        return Vec::new();
    };
    let mut ids = storage
        .list_accounts()
        .ok()
        .unwrap_or_default()
        .into_iter()
        .filter(|account| account.updated_at >= started_at)
        .map(|account| account.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

pub(crate) fn test_cpa_connection(params: Option<&Value>) -> Result<CpaConnectionResult, String> {
    let files = fetch_cpa_auth_files(&resolve_cpa_settings(params)?)?;
    Ok(CpaConnectionResult {
        success: true,
        message: format!("CPA 连接测试成功，可见 {} 个 auth 文件", files.len()),
        total_files: files.len(),
    })
}

fn sync_cpa_accounts_once(params: Option<&Value>, trigger: &str) -> Result<CpaSyncResult, String> {
    let _guard = begin_cpa_sync_run(trigger)?;
    let run_result: Result<CpaSyncResult, String> = (|| -> Result<CpaSyncResult, String> {
        let settings = resolve_cpa_settings(params)?;
        let files = fetch_cpa_auth_files(&settings)?;
        let total_files = files.len();
        let mut eligible_files = 0;
        let mut downloaded_files = 0;
        let mut import_payloads_raw = Vec::new();
        let mut errors = Vec::new();

        for file in files {
            if is_runtime_only_source(&file) {
                errors.push(format!("{} 已跳过: runtime-only auth source", file.name));
                continue;
            }
            if !is_downloadable_file_source(&file) {
                let source = file.source.as_deref().unwrap_or("unknown");
                errors.push(format!(
                    "{} 已跳过: source={} 不是可下载的 file 类型",
                    file.name, source
                ));
                continue;
            }

            let metadata_match = looks_like_target_file(&file);
            let content = match download_auth_file(&settings, &file) {
                Ok(content) => {
                    downloaded_files += 1;
                    content
                }
                Err(err) => {
                    errors.push(format!("{} 下载失败: {err}", file.name));
                    continue;
                }
            };

            let parsed_items = match parse_auth_file_content(&content) {
                Ok(items) => items,
                Err(err) => {
                    errors.push(format!("{} 内容解析失败: {err}", file.name));
                    continue;
                }
            };

            let filtered = filtered_import_items(parsed_items, metadata_match);
            if filtered.is_empty() {
                errors.push(format!(
                    "{} 已跳过: 不是 Codex/OpenAI/ChatGPT auth 文件",
                    file.name
                ));
                continue;
            }

            eligible_files += 1;
            import_payloads_raw.push(serialize_import_payload(&filtered)?);
        }

        let started_at = now_ts();
        let import_summary = import_payloads(import_payloads_raw)?;
        errors.extend(import_summary.errors.iter().cloned());

        Ok(CpaSyncResult {
            total_files,
            eligible_files,
            downloaded_files,
            created: import_summary.created,
            updated: import_summary.updated,
            failed: errors.len() + import_summary.failed,
            imported_account_ids: collect_recent_imported_account_ids(started_at),
            errors,
        })
    })();

    match run_result {
        Ok(result) => {
            mark_cpa_sync_success(&result);
            Ok(result)
        }
        Err(err) => {
            mark_cpa_sync_error(&err);
            Err(err)
        }
    }
}

pub(crate) fn sync_cpa_accounts(params: Option<&Value>) -> Result<CpaSyncResult, String> {
    sync_cpa_accounts_once(params, "manual")
}

pub(crate) fn cpa_sync_status(_params: Option<&Value>) -> Result<CpaSyncStatus, String> {
    Ok(read_cpa_sync_status())
}

pub(crate) fn refresh_cpa_sync_schedule() -> Result<(), String> {
    let config = load_cpa_sync_schedule_config();
    apply_cpa_sync_schedule_config(&config);
    Ok(())
}

pub(crate) fn ensure_cpa_sync_scheduler_started() {
    static START: Once = Once::new();
    START.call_once(|| {
        thread::Builder::new()
            .name("cpa-sync-scheduler".to_string())
            .spawn(|| loop {
                let _ = refresh_cpa_sync_schedule();
                if should_trigger_scheduled_sync() {
                    let _ = sync_cpa_accounts_once(None, "scheduled");
                }
                thread::sleep(Duration::from_secs(1));
            })
            .expect("spawn cpa sync scheduler");
    });
}

#[cfg(test)]
pub(crate) struct CpaImportSummary {
    pub(crate) created: usize,
    pub(crate) failed: usize,
}

#[cfg(test)]
pub(crate) fn import_cpa_payloads_for_test(
    payloads: Vec<String>,
) -> Result<CpaImportSummary, String> {
    let summary = import_payloads(payloads)?;
    Ok(CpaImportSummary {
        created: summary.created,
        failed: summary.failed,
    })
}

#[cfg(test)]
pub(crate) fn auth_files_from_test_payload(payload: Value) -> Result<Vec<String>, String> {
    parse_auth_files(payload).map(|files| files.into_iter().map(|file| file.name).collect())
}

#[cfg(test)]
pub(crate) fn auth_file_flags_for_test(
    payload: Value,
) -> Result<Vec<(String, Option<String>, bool)>, String> {
    parse_auth_files(payload).map(|files| {
        files
            .into_iter()
            .map(|file| (file.name, file.source, file.runtime_only))
            .collect()
    })
}

#[cfg(test)]
pub(crate) fn filter_import_items_for_test(
    payload: &str,
    metadata_match: bool,
) -> Result<usize, String> {
    Ok(filtered_import_items(parse_auth_file_content(payload)?, metadata_match).len())
}

#[cfg(test)]
pub(crate) fn download_auth_file_for_test(
    api_url: &str,
    management_key: &str,
    payload: Value,
) -> Result<String, String> {
    let mut files = parse_auth_files(payload)?;
    let file = files.pop().ok_or_else(|| "missing auth file".to_string())?;
    download_auth_file(
        &CpaSyncSettings {
            api_url: api_url.to_string(),
            management_key: management_key.to_string(),
        },
        &file,
    )
}

#[cfg(test)]
pub(crate) fn resolve_cpa_settings_for_test(
    payload: Option<&Value>,
) -> Result<(String, String), String> {
    let settings = resolve_cpa_settings(payload)?;
    Ok((settings.api_url, settings.management_key))
}

#[cfg(test)]
pub(crate) fn cpa_sync_status_for_test() -> CpaSyncStatus {
    read_cpa_sync_status()
}

#[cfg(test)]
pub(crate) fn begin_cpa_sync_run_for_test(trigger: &str) -> Result<CpaSyncRunGuard, String> {
    begin_cpa_sync_run(trigger)
}

#[cfg(test)]
pub(crate) fn reset_cpa_sync_runtime_for_test() {
    with_cpa_sync_runtime(|state| {
        *state = CpaSyncRuntimeState::default();
    });
}

#[cfg(test)]
pub(crate) fn refresh_cpa_sync_schedule_for_test(
    _source_enabled: Option<bool>,
    schedule_enabled: bool,
    interval_minutes: u64,
    api_url: &str,
    has_management_key: bool,
) {
    apply_cpa_sync_schedule_config(&CpaSyncScheduleConfig {
        source_enabled: true,
        schedule_enabled,
        interval_minutes,
        api_url: api_url.to_string(),
        has_management_key,
    });
}

#[cfg(test)]
#[path = "tests/account_cpa_sync_tests.rs"]
mod tests;
