use codexmanager_core::auth::{DEFAULT_CLIENT_ID, DEFAULT_ISSUER, DEFAULT_ORIGINATOR};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

const ENV_OVERRIDE_SCOPE_SERVICE: &str = "service";
const ENV_OVERRIDE_SCOPE_DESKTOP: &str = "desktop";
const ENV_OVERRIDE_SCOPE_WEB: &str = "web";
const ENV_OVERRIDE_APPLY_MODE_RUNTIME: &str = "runtime";
const ENV_OVERRIDE_APPLY_MODE_RESTART: &str = "restart";

const APP_SETTINGS_ENV_UNSUPPORTED_KEYS: &[&str] = &[
    "CODEXMANAGER_DB_PATH",
    "CODEXMANAGER_RPC_TOKEN",
    "CODEXMANAGER_RPC_TOKEN_FILE",
];

const APP_SETTINGS_ENV_RESERVED_KEYS: &[&str] = &[
    "CODEXMANAGER_SERVICE_ADDR",
    "CODEXMANAGER_ROUTE_STRATEGY",
    "CODEXMANAGER_CPA_NO_COOKIE_HEADER_MODE",
    "CODEXMANAGER_UPSTREAM_PROXY_URL",
    "CODEXMANAGER_DISABLE_POLLING",
    "CODEXMANAGER_USAGE_POLLING_ENABLED",
    "CODEXMANAGER_USAGE_POLL_INTERVAL_SECS",
    "CODEXMANAGER_GATEWAY_KEEPALIVE_ENABLED",
    "CODEXMANAGER_GATEWAY_KEEPALIVE_INTERVAL_SECS",
    "CODEXMANAGER_TOKEN_REFRESH_POLLING_ENABLED",
    "CODEXMANAGER_TOKEN_REFRESH_POLL_INTERVAL_SECS",
    "CODEXMANAGER_USAGE_REFRESH_WORKERS",
    "CODEXMANAGER_HTTP_WORKER_FACTOR",
    "CODEXMANAGER_HTTP_WORKER_MIN",
    "CODEXMANAGER_HTTP_STREAM_WORKER_FACTOR",
    "CODEXMANAGER_HTTP_STREAM_WORKER_MIN",
];

pub(crate) fn env_override_reserved_keys() -> &'static [&'static str] {
    APP_SETTINGS_ENV_RESERVED_KEYS
}

pub(crate) fn env_override_unsupported_keys() -> &'static [&'static str] {
    APP_SETTINGS_ENV_UNSUPPORTED_KEYS
}

static ENV_OVERRIDE_BASELINE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct EnvOverrideCatalogItem {
    key: &'static str,
    label: &'static str,
    scope: &'static str,
    apply_mode: &'static str,
    default_value: &'static str,
}

impl EnvOverrideCatalogItem {
    const fn new(
        key: &'static str,
        label: &'static str,
        scope: &'static str,
        apply_mode: &'static str,
        default_value: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            scope,
            apply_mode,
            default_value,
        }
    }
}

const ENV_OVERRIDE_CATALOG: &[EnvOverrideCatalogItem] = &[
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ACCOUNT_IMPORT_BATCH_SIZE",
        "账号导入批大小",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "200",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ACCOUNT_MAX_INFLIGHT",
        "单账号最大并发",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "0",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ALLOW_NON_LOOPBACK_LOGIN_ADDR",
        "允许非回环登录回调",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "0",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_CANDIDATE_CACHE_TTL_MS",
        "候选缓存 TTL（毫秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "500",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_CLIENT_ID",
        "OpenAI Client ID",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        DEFAULT_CLIENT_ID,
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_FRONT_PROXY_MAX_BODY_BYTES",
        "前置代理最大请求体（字节）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "16777216",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_GATEWAY_KEEPALIVE_FAILURE_BACKOFF_MAX_SECS",
        "保活失败退避上限（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "900",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_GATEWAY_KEEPALIVE_JITTER_SECS",
        "保活抖动（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "5",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_GITHUB_TOKEN",
        "GitHub 访问令牌",
        ENV_OVERRIDE_SCOPE_DESKTOP,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_HTTP_BRIDGE_OUTPUT_TEXT_LIMIT_BYTES",
        "HTTP 桥输出截断上限（字节）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "131072",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_HTTP_QUEUE_FACTOR",
        "普通请求队列因子",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "4",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_HTTP_QUEUE_MIN",
        "普通请求最小队列",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "32",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_HTTP_STREAM_QUEUE_FACTOR",
        "流式请求队列因子",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "2",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_HTTP_STREAM_QUEUE_MIN",
        "流式请求最小队列",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "16",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ISSUER",
        "OpenAI Issuer",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        DEFAULT_ISSUER,
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_LOGIN_ADDR",
        "登录回调监听地址",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "localhost:1455",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_NO_SERVICE",
        "桌面端不启动 Service",
        ENV_OVERRIDE_SCOPE_DESKTOP,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ORIGINATOR",
        "登录 Originator",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        DEFAULT_ORIGINATOR,
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_POLL_FAILURE_BACKOFF_MAX_SECS",
        "通用轮询失败退避上限（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "1800",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_POLL_JITTER_SECS",
        "通用轮询抖动（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "5",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_PROMPT_CACHE_CAPACITY",
        "Prompt 缓存容量",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "4096",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_PROMPT_CACHE_CLEANUP_INTERVAL_SECS",
        "Prompt 缓存清理间隔（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "60",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_PROMPT_CACHE_TTL_SECS",
        "Prompt 缓存 TTL（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "3600",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_PROXY_LIST",
        "上游代理池列表",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_REDIRECT_URI",
        "登录回调 URI",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "http://localhost:1455/auth/callback",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_REQUEST_GATE_WAIT_TIMEOUT_MS",
        "请求闸门等待超时（毫秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "300",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ROUTE_HEALTH_P2C_BALANCED_WINDOW",
        "均衡模式 P2C 窗口",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "6",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ROUTE_HEALTH_P2C_ENABLED",
        "启用路由 P2C 健康选择",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "1",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ROUTE_HEALTH_P2C_ORDERED_WINDOW",
        "有序模式 P2C 窗口",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "3",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ROUTE_STATE_CAPACITY",
        "路由状态容量",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "4096",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_ROUTE_STATE_TTL_SECS",
        "路由状态 TTL（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "21600",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_STRICT_REQUEST_PARAM_ALLOWLIST",
        "严格请求参数白名单",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "1",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_TRACE_BODY_PREVIEW_MAX_BYTES",
        "Trace Body 预览上限（字节）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "0",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_TRACE_QUEUE_CAPACITY",
        "Trace 队列容量",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "2048",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPDATE_PRERELEASE",
        "更新包含预发布",
        ENV_OVERRIDE_SCOPE_DESKTOP,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPDATE_REPO",
        "更新仓库",
        ENV_OVERRIDE_SCOPE_DESKTOP,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "qxcnm/Codex-Manager",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_BASE_URL",
        "上游基础地址",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "https://chatgpt.com/backend-api/codex",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_CONNECT_TIMEOUT_SECS",
        "上游连接超时（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "15",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_COOKIE",
        "上游 Cookie",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_FALLBACK_BASE_URL",
        "上游回退地址",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS",
        "上游流式超时（毫秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "300000",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS",
        "上游总超时（毫秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "120000",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_USAGE_BASE_URL",
        "用量接口基础地址",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "https://chatgpt.com",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_USAGE_POLL_FAILURE_BACKOFF_MAX_SECS",
        "用量轮询失败退避上限（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "1800",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_USAGE_POLL_JITTER_SECS",
        "用量轮询抖动（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "5",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_USAGE_REFRESH_FAILURE_EVENT_WINDOW_SECS",
        "用量失败事件去重窗口（秒）",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "60",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_USAGE_SNAPSHOTS_RETAIN_PER_ACCOUNT",
        "每账号保留用量快照数",
        ENV_OVERRIDE_SCOPE_SERVICE,
        ENV_OVERRIDE_APPLY_MODE_RUNTIME,
        "200",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_WEB_ADDR",
        "Web 监听地址",
        ENV_OVERRIDE_SCOPE_WEB,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "localhost:48761",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_WEB_NO_OPEN",
        "Web 启动后不自动打开",
        ENV_OVERRIDE_SCOPE_WEB,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_WEB_NO_SPAWN_SERVICE",
        "Web 不自动拉起 Service",
        ENV_OVERRIDE_SCOPE_WEB,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
    EnvOverrideCatalogItem::new(
        "CODEXMANAGER_WEB_ROOT",
        "Web 静态资源目录",
        ENV_OVERRIDE_SCOPE_WEB,
        ENV_OVERRIDE_APPLY_MODE_RESTART,
        "",
    ),
];

pub(crate) fn env_override_catalog_value() -> Vec<Value> {
    ENV_OVERRIDE_CATALOG
        .iter()
        .map(|item| {
            serde_json::json!({
                "key": item.key,
                "label": item.label,
                "scope": item.scope,
                "applyMode": item.apply_mode,
                "defaultValue": env_override_default_value(item.key),
            })
        })
        .collect()
}

fn env_override_baseline() -> &'static Mutex<HashMap<String, Option<String>>> {
    ENV_OVERRIDE_BASELINE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn env_override_catalog_item(key: &str) -> Option<&'static EnvOverrideCatalogItem> {
    ENV_OVERRIDE_CATALOG
        .iter()
        .find(|item| item.key.eq_ignore_ascii_case(key))
}

fn is_env_override_catalog_key(key: &str) -> bool {
    env_override_catalog_item(key).is_some()
}

fn is_env_override_unsupported_key(key: &str) -> bool {
    APP_SETTINGS_ENV_UNSUPPORTED_KEYS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(key))
}

fn is_env_override_reserved_key(key: &str) -> bool {
    APP_SETTINGS_ENV_RESERVED_KEYS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(key))
}

fn normalize_env_override_key(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err("environment variable key is empty".to_string());
    }
    if !normalized.starts_with("CODEXMANAGER_") {
        return Err(format!("{normalized} must start with CODEXMANAGER_"));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(format!("{normalized} contains unsupported characters"));
    }
    if is_env_override_unsupported_key(&normalized) {
        return Err(format!(
            "{normalized} must stay in process/.env because it is required before app_settings can be loaded"
        ));
    }
    if is_env_override_reserved_key(&normalized) {
        return Err(format!(
            "{normalized} is already managed by an existing settings card; update it there instead"
        ));
    }
    Ok(normalized)
}

fn normalize_env_override_patch_value(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_saved_env_override_text(raw: &str) -> String {
    raw.trim().to_string()
}

fn normalize_env_overrides_patch(
    overrides: HashMap<String, String>,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut normalized = BTreeMap::new();
    for (raw_key, raw_value) in overrides {
        let key = normalize_env_override_key(&raw_key)?;
        normalized.insert(key, normalize_env_override_patch_value(Some(&raw_value)));
    }
    Ok(normalized)
}

fn parse_saved_env_override_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(normalize_saved_env_override_text(text)),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_string()),
        Value::Null => None,
        _ => None,
    }
}

fn env_override_original_process_value(key: &str) -> Option<String> {
    let baseline =
        crate::lock_utils::lock_recover(env_override_baseline(), "env_override_baseline");
    if let Some(value) = baseline.get(key) {
        return value.clone();
    }
    drop(baseline);
    super::normalize_optional_text(std::env::var(key).ok().as_deref())
}

fn env_override_default_value(key: &str) -> String {
    env_override_original_process_value(key).unwrap_or_else(|| {
        env_override_catalog_item(key)
            .map(|item| item.default_value.to_string())
            .unwrap_or_default()
    })
}

fn env_override_default_snapshot() -> BTreeMap<String, String> {
    let mut snapshot = BTreeMap::new();
    for item in ENV_OVERRIDE_CATALOG {
        snapshot.insert(item.key.to_string(), env_override_default_value(item.key));
    }
    snapshot
}

fn persisted_env_overrides(mut normalized: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let Some(raw) = super::get_persisted_app_setting(super::APP_SETTING_ENV_OVERRIDES_KEY) else {
        return normalized;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Map<String, Value>>(&raw) else {
        log::warn!("parse persisted env overrides failed: invalid json");
        return normalized;
    };

    for (raw_key, raw_value) in parsed {
        let Ok(key) = normalize_env_override_key(&raw_key) else {
            log::warn!(
                "skip persisted env override: key={} invalid",
                raw_key.trim()
            );
            continue;
        };
        if let Some(value) = parse_saved_env_override_value(&raw_value) {
            if is_env_override_catalog_key(&key) || !value.is_empty() {
                normalized.insert(key, value);
            } else {
                normalized.remove(&key);
            }
        }
    }
    normalized
}

pub(crate) fn persisted_env_overrides_only() -> BTreeMap<String, String> {
    persisted_env_overrides(BTreeMap::new())
}

pub(crate) fn current_env_overrides() -> BTreeMap<String, String> {
    persisted_env_overrides(env_override_default_snapshot())
}

pub(crate) fn save_env_overrides_value(overrides: &BTreeMap<String, String>) -> Result<(), String> {
    let raw = serde_json::to_string(overrides)
        .map_err(|err| format!("serialize env overrides failed: {err}"))?;
    super::save_persisted_app_setting(super::APP_SETTING_ENV_OVERRIDES_KEY, Some(&raw))
}

pub(crate) fn apply_env_overrides_to_process(
    previous: &BTreeMap<String, String>,
    next: &BTreeMap<String, String>,
) {
    let mut all_keys = BTreeSet::new();
    all_keys.extend(previous.keys().cloned());
    all_keys.extend(next.keys().cloned());
    if all_keys.is_empty() {
        return;
    }

    let mut baseline =
        crate::lock_utils::lock_recover(env_override_baseline(), "env_override_baseline");
    for key in &all_keys {
        baseline
            .entry(key.clone())
            .or_insert_with(|| super::normalize_optional_text(std::env::var(key).ok().as_deref()));
    }

    for key in all_keys {
        if let Some(value) = next.get(&key) {
            if value.trim().is_empty() {
                if let Some(original) = baseline.get(&key).and_then(|value| value.clone()) {
                    std::env::set_var(&key, original);
                } else {
                    std::env::remove_var(&key);
                }
            } else {
                std::env::set_var(&key, value);
            }
            continue;
        }
        if let Some(original) = baseline.get(&key).and_then(|value| value.clone()) {
            std::env::set_var(&key, original);
        } else {
            std::env::remove_var(&key);
        }
    }
}

pub(crate) fn reload_runtime_after_env_override_apply() {
    crate::gateway::reload_runtime_config_from_env();
    crate::usage_refresh::reload_background_tasks_runtime_from_env();
    crate::usage_http::reload_usage_http_client_from_env();
}

pub(crate) fn set_env_overrides(
    overrides: HashMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let previous = current_env_overrides();
    let patch = normalize_env_overrides_patch(overrides)?;
    let mut next = if patch.is_empty() {
        env_override_default_snapshot()
    } else {
        previous.clone()
    };

    for (key, value) in patch {
        if let Some(value) = value {
            next.insert(key, value);
        } else if is_env_override_catalog_key(&key) {
            next.insert(key.clone(), env_override_default_value(&key));
        } else {
            next.remove(&key);
        }
    }

    for item in ENV_OVERRIDE_CATALOG {
        next.entry(item.key.to_string())
            .or_insert_with(|| env_override_default_value(item.key));
    }

    save_env_overrides_value(&next)?;
    apply_env_overrides_to_process(&previous, &next);
    reload_runtime_after_env_override_apply();
    Ok(next)
}
