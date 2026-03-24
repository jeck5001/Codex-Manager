use codexmanager_core::auth::DEFAULT_ORIGINATOR;
use codexmanager_core::auth::{DEFAULT_CLIENT_ID, DEFAULT_ISSUER};
use reqwest::blocking::Client;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

static UPSTREAM_CLIENT: OnceLock<RwLock<Client>> = OnceLock::new();
static UPSTREAM_CLIENT_POOL: OnceLock<RwLock<UpstreamClientPool>> = OnceLock::new();
static RUNTIME_CONFIG_LOADED: OnceLock<()> = OnceLock::new();
static REQUEST_GATE_WAIT_TIMEOUT_MS: AtomicU64 =
    AtomicU64::new(DEFAULT_REQUEST_GATE_WAIT_TIMEOUT_MS);
static TRACE_BODY_PREVIEW_MAX_BYTES: AtomicUsize =
    AtomicUsize::new(DEFAULT_TRACE_BODY_PREVIEW_MAX_BYTES);
static FRONT_PROXY_MAX_BODY_BYTES: AtomicUsize =
    AtomicUsize::new(DEFAULT_FRONT_PROXY_MAX_BODY_BYTES);
static UPSTREAM_CONNECT_TIMEOUT_SECS: AtomicU64 =
    AtomicU64::new(DEFAULT_UPSTREAM_CONNECT_TIMEOUT_SECS);
static UPSTREAM_TOTAL_TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_UPSTREAM_TOTAL_TIMEOUT_MS);
static UPSTREAM_STREAM_TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_UPSTREAM_STREAM_TIMEOUT_MS);
static ACCOUNT_MAX_INFLIGHT: AtomicUsize = AtomicUsize::new(DEFAULT_ACCOUNT_MAX_INFLIGHT);
static CPA_NO_COOKIE_HEADER_MODE: AtomicBool = AtomicBool::new(DEFAULT_CPA_NO_COOKIE_HEADER_MODE);
static STRICT_REQUEST_PARAM_ALLOWLIST: AtomicBool =
    AtomicBool::new(DEFAULT_STRICT_REQUEST_PARAM_ALLOWLIST);
static ENABLE_REQUEST_COMPRESSION: AtomicBool = AtomicBool::new(DEFAULT_ENABLE_REQUEST_COMPRESSION);
static UPSTREAM_COOKIE: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static UPSTREAM_PROXY_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static FREE_ACCOUNT_MAX_MODEL: OnceLock<RwLock<String>> = OnceLock::new();
static PAYLOAD_REWRITE_RULES: OnceLock<RwLock<Vec<PayloadRewriteRule>>> = OnceLock::new();
static MODEL_ALIAS_POOLS: OnceLock<RwLock<Vec<ModelAliasPool>>> = OnceLock::new();
static ORIGINATOR: OnceLock<RwLock<String>> = OnceLock::new();
static RESIDENCY_REQUIREMENT: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static TOKEN_EXCHANGE_CLIENT_ID: OnceLock<RwLock<String>> = OnceLock::new();
static TOKEN_EXCHANGE_ISSUER: OnceLock<RwLock<String>> = OnceLock::new();

pub(crate) const DEFAULT_GATEWAY_DEBUG: bool = false;
const DEFAULT_UPSTREAM_CONNECT_TIMEOUT_SECS: u64 = 15;
const DEFAULT_UPSTREAM_TOTAL_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_UPSTREAM_STREAM_TIMEOUT_MS: u64 = 1_800_000;
// 中文注释：默认把单账号并发收紧到 1，避免多个长连接 Codex 会话同时压到同一账号上。
const DEFAULT_ACCOUNT_MAX_INFLIGHT: usize = 1;
const DEFAULT_CPA_NO_COOKIE_HEADER_MODE: bool = false;
const DEFAULT_STRICT_REQUEST_PARAM_ALLOWLIST: bool = true;
const DEFAULT_ENABLE_REQUEST_COMPRESSION: bool = true;
const DEFAULT_REQUEST_GATE_WAIT_TIMEOUT_MS: u64 = 300;
const DEFAULT_TRACE_BODY_PREVIEW_MAX_BYTES: usize = 0;
const DEFAULT_FRONT_PROXY_MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_FREE_ACCOUNT_MAX_MODEL: &str = "auto";
const MAX_UPSTREAM_PROXY_POOL_SIZE: usize = 5;

const ENV_REQUEST_GATE_WAIT_TIMEOUT_MS: &str = "CODEXMANAGER_REQUEST_GATE_WAIT_TIMEOUT_MS";
const ENV_TRACE_BODY_PREVIEW_MAX_BYTES: &str = "CODEXMANAGER_TRACE_BODY_PREVIEW_MAX_BYTES";
const ENV_FRONT_PROXY_MAX_BODY_BYTES: &str = "CODEXMANAGER_FRONT_PROXY_MAX_BODY_BYTES";
const ENV_UPSTREAM_CONNECT_TIMEOUT_SECS: &str = "CODEXMANAGER_UPSTREAM_CONNECT_TIMEOUT_SECS";
const ENV_UPSTREAM_TOTAL_TIMEOUT_MS: &str = "CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS";
const ENV_UPSTREAM_STREAM_TIMEOUT_MS: &str = "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS";
const ENV_ACCOUNT_MAX_INFLIGHT: &str = "CODEXMANAGER_ACCOUNT_MAX_INFLIGHT";
const ENV_CPA_NO_COOKIE_HEADER_MODE: &str = "CODEXMANAGER_CPA_NO_COOKIE_HEADER_MODE";
const ENV_STRICT_REQUEST_PARAM_ALLOWLIST: &str = "CODEXMANAGER_STRICT_REQUEST_PARAM_ALLOWLIST";
const ENV_ENABLE_REQUEST_COMPRESSION: &str = "CODEXMANAGER_ENABLE_REQUEST_COMPRESSION";
const ENV_TOKEN_EXCHANGE_CLIENT_ID: &str = "CODEXMANAGER_CLIENT_ID";
const ENV_TOKEN_EXCHANGE_ISSUER: &str = "CODEXMANAGER_ISSUER";
const ENV_PROXY_LIST: &str = "CODEXMANAGER_PROXY_LIST";
const ENV_UPSTREAM_PROXY_URL: &str = "CODEXMANAGER_UPSTREAM_PROXY_URL";
const ENV_FREE_ACCOUNT_MAX_MODEL: &str = "CODEXMANAGER_FREE_ACCOUNT_MAX_MODEL";
const ENV_ORIGINATOR: &str = "CODEXMANAGER_ORIGINATOR";
const ENV_PAYLOAD_REWRITE_RULES: &str = "CODEXMANAGER_PAYLOAD_REWRITE_RULES";
const ENV_MODEL_ALIAS_POOLS: &str = "CODEXMANAGER_MODEL_ALIAS_POOLS";
const ENV_RESIDENCY_REQUIREMENT: &str = "CODEXMANAGER_RESIDENCY_REQUIREMENT";
pub(crate) const RESIDENCY_HEADER_NAME: &str = "x-openai-internal-codex-residency";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PayloadRewriteMode {
    Set,
    SetIfMissing,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PayloadRewriteRule {
    #[serde(default = "payload_rewrite_rule_enabled_default")]
    pub enabled: bool,
    pub path: String,
    pub field: String,
    pub mode: PayloadRewriteMode,
    pub value: Value,
}

fn payload_rewrite_rule_enabled_default() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModelAliasPoolStrategy {
    Ordered,
    Weighted,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct ModelAliasPoolTarget {
    #[serde(default = "model_alias_pool_target_enabled_default")]
    pub enabled: bool,
    pub model: String,
    #[serde(default = "model_alias_pool_target_weight_default")]
    pub weight: u32,
    #[serde(default)]
    pub channel: Option<String>,
}

fn model_alias_pool_target_enabled_default() -> bool {
    true
}

fn model_alias_pool_target_weight_default() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct ModelAliasPool {
    #[serde(default = "model_alias_pool_enabled_default")]
    pub enabled: bool,
    pub alias: String,
    pub strategy: ModelAliasPoolStrategy,
    #[serde(default)]
    pub targets: Vec<ModelAliasPoolTarget>,
}

fn model_alias_pool_enabled_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelAliasResolution {
    pub alias: String,
    pub actual_model: String,
    pub channel: Option<String>,
}

#[derive(Default, Clone)]
struct UpstreamClientPool {
    proxies: Vec<String>,
    clients: Vec<Client>,
}

impl UpstreamClientPool {
    fn client_for_account(&self, account_id: &str) -> Option<&Client> {
        let idx = stable_proxy_index(account_id, self.clients.len())?;
        self.clients.get(idx)
    }

    fn proxy_for_account(&self, account_id: &str) -> Option<&str> {
        let idx = stable_proxy_index(account_id, self.proxies.len())?;
        self.proxies.get(idx).map(String::as_str)
    }
}

pub(crate) fn upstream_client() -> Client {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(upstream_client_lock(), "upstream_client").clone()
}

pub(crate) fn fresh_upstream_client() -> Client {
    ensure_runtime_config_loaded();
    build_upstream_client()
}

pub(crate) fn upstream_client_for_account(account_id: &str) -> Client {
    ensure_runtime_config_loaded();
    let cached =
        crate::lock_utils::read_recover(upstream_client_pool_lock(), "upstream_client_pool")
            .client_for_account(account_id)
            .cloned();
    cached.unwrap_or_else(upstream_client)
}

pub(crate) fn fresh_upstream_client_for_account(account_id: &str) -> Client {
    ensure_runtime_config_loaded();
    let pool = crate::lock_utils::read_recover(upstream_client_pool_lock(), "upstream_client_pool");
    if let Some(proxy_url) = pool.proxy_for_account(account_id) {
        return build_upstream_client_with_proxy(Some(proxy_url));
    }
    build_upstream_client()
}

fn upstream_connect_timeout_cached() -> Duration {
    Duration::from_secs(UPSTREAM_CONNECT_TIMEOUT_SECS.load(Ordering::Relaxed))
}

fn build_upstream_client() -> Client {
    let proxy_url = current_upstream_proxy_url();
    build_upstream_client_with_proxy(proxy_url.as_deref())
}

fn build_upstream_client_with_proxy(proxy_url: Option<&str>) -> Client {
    let mut builder = Client::builder()
        // 中文注释：显式关闭总超时，避免长时流式响应在客户端层被误判超时中断。
        .timeout(None::<Duration>)
        // 中文注释：连接阶段设置超时，避免网络异常时线程长期卡死占满并发槽位。
        .connect_timeout(upstream_connect_timeout_cached())
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .tcp_keepalive(Some(Duration::from_secs(30)));
    if proxy_url.is_none() && upstream_base_uses_loopback_host() {
        // 中文注释：本地 mock upstream / 本地旁路部署不应吃系统代理，否则 127.0.0.1
        // 也可能被送进企业代理/VPN，导致测试与本地联调出现假性 5xx/超时。
        builder = builder.no_proxy();
    }
    if let Some(proxy_url) = proxy_url {
        let proxy = match Proxy::all(proxy_url) {
            Ok(proxy) => proxy,
            Err(err) => {
                log::warn!(
                    "event=gateway_proxy_pool_invalid_proxy proxy={} err={}",
                    proxy_url,
                    err
                );
                return build_upstream_client();
            }
        };
        builder = builder.proxy(proxy);
    }
    builder.build().unwrap_or_else(|err| {
        log::warn!("event=gateway_upstream_client_build_failed err={}", err);
        Client::new()
    })
}

fn upstream_base_uses_loopback_host() -> bool {
    let base = crate::gateway::resolve_upstream_base_url();
    reqwest::Url::parse(base.as_str())
        .ok()
        .and_then(|url: reqwest::Url| url.host_str().map(|host: &str| host.to_ascii_lowercase()))
        .is_some_and(|host: String| matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1"))
}

pub(crate) fn upstream_total_timeout() -> Option<Duration> {
    ensure_runtime_config_loaded();
    let timeout_ms = UPSTREAM_TOTAL_TIMEOUT_MS.load(Ordering::Relaxed);
    if timeout_ms == 0 {
        None
    } else {
        Some(Duration::from_millis(timeout_ms))
    }
}

pub(crate) fn upstream_stream_timeout() -> Option<Duration> {
    ensure_runtime_config_loaded();
    let timeout_ms = UPSTREAM_STREAM_TIMEOUT_MS.load(Ordering::Relaxed);
    if timeout_ms == 0 {
        None
    } else {
        Some(Duration::from_millis(timeout_ms))
    }
}

pub(crate) fn current_upstream_stream_timeout_ms() -> u64 {
    ensure_runtime_config_loaded();
    UPSTREAM_STREAM_TIMEOUT_MS.load(Ordering::Relaxed)
}

pub(crate) fn request_compression_enabled() -> bool {
    ensure_runtime_config_loaded();
    ENABLE_REQUEST_COMPRESSION.load(Ordering::Relaxed)
}

pub(crate) fn current_payload_rewrite_rules() -> Vec<PayloadRewriteRule> {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(payload_rewrite_rules_cell(), "payload_rewrite_rules").clone()
}

pub(crate) fn current_payload_rewrite_rules_json() -> String {
    let rules = current_payload_rewrite_rules();
    serde_json::to_string(&rules).unwrap_or_else(|_| "[]".to_string())
}

pub(crate) fn current_model_alias_pools() -> Vec<ModelAliasPool> {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(model_alias_pools_cell(), "model_alias_pools").clone()
}

pub(crate) fn current_model_alias_pools_json() -> String {
    let pools = current_model_alias_pools();
    serde_json::to_string(&pools).unwrap_or_else(|_| "[]".to_string())
}

pub(crate) fn resolve_model_alias(
    requested_model: &str,
    trace_id: &str,
) -> Option<ModelAliasResolution> {
    ensure_runtime_config_loaded();
    let requested_model = normalize_alias_or_model_token(requested_model)?;
    let pools = crate::lock_utils::read_recover(model_alias_pools_cell(), "model_alias_pools");
    let pool = pools.iter().find(|pool| {
        pool.enabled
            && normalize_alias_or_model_token(pool.alias.as_str()).as_deref()
                == Some(requested_model.as_str())
    })?;
    select_model_alias_target(pool, trace_id).map(|target| ModelAliasResolution {
        alias: requested_model,
        actual_model: target.model.clone(),
        channel: target.channel.clone(),
    })
}

pub(crate) fn account_max_inflight_limit() -> usize {
    ensure_runtime_config_loaded();
    ACCOUNT_MAX_INFLIGHT.load(Ordering::Relaxed)
}

pub(super) fn cpa_no_cookie_header_mode_enabled() -> bool {
    ensure_runtime_config_loaded();
    CPA_NO_COOKIE_HEADER_MODE.load(Ordering::Relaxed)
}

pub(crate) fn strict_request_param_allowlist_enabled() -> bool {
    ensure_runtime_config_loaded();
    STRICT_REQUEST_PARAM_ALLOWLIST.load(Ordering::Relaxed)
}

pub(super) fn set_cpa_no_cookie_header_mode_enabled(enabled: bool) {
    ensure_runtime_config_loaded();
    CPA_NO_COOKIE_HEADER_MODE.store(enabled, Ordering::Relaxed);
}

pub(crate) fn request_gate_wait_timeout() -> Duration {
    ensure_runtime_config_loaded();
    Duration::from_millis(REQUEST_GATE_WAIT_TIMEOUT_MS.load(Ordering::Relaxed))
}

pub(crate) fn trace_body_preview_max_bytes() -> usize {
    ensure_runtime_config_loaded();
    TRACE_BODY_PREVIEW_MAX_BYTES.load(Ordering::Relaxed)
}

pub(crate) fn front_proxy_max_body_bytes() -> usize {
    ensure_runtime_config_loaded();
    FRONT_PROXY_MAX_BODY_BYTES.load(Ordering::Relaxed)
}

pub(super) fn upstream_cookie() -> Option<String> {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(upstream_cookie_cell(), "upstream_cookie").clone()
}

pub(super) fn upstream_proxy_url() -> Option<String> {
    ensure_runtime_config_loaded();
    current_upstream_proxy_url()
}

pub(crate) fn current_free_account_max_model() -> String {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(free_account_max_model_cell(), "free_account_max_model").clone()
}

pub(crate) fn current_originator() -> String {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(originator_cell(), "originator").clone()
}

pub(crate) fn set_originator(originator: &str) -> Result<String, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_originator(originator)?;
    std::env::set_var(ENV_ORIGINATOR, normalized.as_str());
    let mut cached = crate::lock_utils::write_recover(originator_cell(), "originator");
    *cached = normalized.clone();
    Ok(normalized)
}

pub(crate) fn current_codex_user_agent() -> String {
    ensure_runtime_config_loaded();
    let originator = current_originator();
    format!(
        "{}/{} ({}; {}) CodexManagerGateway",
        originator,
        "0.101.0",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

pub(crate) fn current_residency_requirement() -> Option<String> {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(residency_requirement_cell(), "residency_requirement").clone()
}

pub(crate) fn set_residency_requirement(value: Option<&str>) -> Result<Option<String>, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_residency_requirement(value)?;
    if let Some(value) = normalized.as_deref() {
        std::env::set_var(ENV_RESIDENCY_REQUIREMENT, value);
    } else {
        std::env::remove_var(ENV_RESIDENCY_REQUIREMENT);
    }
    let mut cached =
        crate::lock_utils::write_recover(residency_requirement_cell(), "residency_requirement");
    *cached = normalized.clone();
    Ok(normalized)
}

pub(crate) fn set_free_account_max_model(model: &str) -> Result<String, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_model_slug(model)?;
    std::env::set_var(ENV_FREE_ACCOUNT_MAX_MODEL, normalized.as_str());
    let mut cached =
        crate::lock_utils::write_recover(free_account_max_model_cell(), "free_account_max_model");
    *cached = normalized.clone();
    Ok(normalized)
}

pub(crate) fn set_request_compression_enabled(enabled: bool) -> bool {
    ensure_runtime_config_loaded();
    ENABLE_REQUEST_COMPRESSION.store(enabled, Ordering::Relaxed);
    std::env::set_var(
        ENV_ENABLE_REQUEST_COMPRESSION,
        if enabled { "1" } else { "0" },
    );
    enabled
}

pub(crate) fn set_payload_rewrite_rules_json(raw: Option<&str>) -> Result<String, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_payload_rewrite_rules_json(raw)?;
    let rules = parse_payload_rewrite_rules_json(Some(normalized.as_str()))?;
    if rules.is_empty() {
        std::env::remove_var(ENV_PAYLOAD_REWRITE_RULES);
    } else {
        std::env::set_var(ENV_PAYLOAD_REWRITE_RULES, normalized.as_str());
    }
    let mut cached =
        crate::lock_utils::write_recover(payload_rewrite_rules_cell(), "payload_rewrite_rules");
    *cached = rules;
    Ok(normalized)
}

pub(crate) fn set_model_alias_pools_json(raw: Option<&str>) -> Result<String, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_model_alias_pools_json(raw)?;
    let pools = parse_model_alias_pools_json(Some(normalized.as_str()))?;
    if pools.is_empty() {
        std::env::remove_var(ENV_MODEL_ALIAS_POOLS);
    } else {
        std::env::set_var(ENV_MODEL_ALIAS_POOLS, normalized.as_str());
    }
    let mut cached =
        crate::lock_utils::write_recover(model_alias_pools_cell(), "model_alias_pools");
    *cached = pools;
    Ok(normalized)
}

pub(super) fn set_upstream_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    ensure_runtime_config_loaded();
    let normalized = normalize_upstream_proxy_url(proxy_url)?;

    if let Some(value) = normalized.as_deref() {
        std::env::set_var(ENV_UPSTREAM_PROXY_URL, value);
    } else {
        std::env::remove_var(ENV_UPSTREAM_PROXY_URL);
    }

    let mut cached_proxy_url =
        crate::lock_utils::write_recover(upstream_proxy_url_cell(), "upstream_proxy_url");
    *cached_proxy_url = normalized.clone();
    drop(cached_proxy_url);
    refresh_upstream_clients_from_runtime_config();
    Ok(normalized)
}

pub(crate) fn set_upstream_stream_timeout_ms(timeout_ms: u64) -> u64 {
    ensure_runtime_config_loaded();
    UPSTREAM_STREAM_TIMEOUT_MS.store(timeout_ms, Ordering::Relaxed);
    std::env::set_var(ENV_UPSTREAM_STREAM_TIMEOUT_MS, timeout_ms.to_string());
    timeout_ms
}

pub(super) fn token_exchange_client_id() -> String {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(token_exchange_client_id_cell(), "token_exchange_client_id")
        .clone()
}

pub(super) fn token_exchange_default_issuer() -> String {
    ensure_runtime_config_loaded();
    crate::lock_utils::read_recover(token_exchange_issuer_cell(), "token_exchange_issuer").clone()
}

pub(super) fn reload_from_env() {
    REQUEST_GATE_WAIT_TIMEOUT_MS.store(
        env_u64_or(
            ENV_REQUEST_GATE_WAIT_TIMEOUT_MS,
            DEFAULT_REQUEST_GATE_WAIT_TIMEOUT_MS,
        ),
        Ordering::Relaxed,
    );
    TRACE_BODY_PREVIEW_MAX_BYTES.store(
        env_usize_or(
            ENV_TRACE_BODY_PREVIEW_MAX_BYTES,
            DEFAULT_TRACE_BODY_PREVIEW_MAX_BYTES,
        ),
        Ordering::Relaxed,
    );
    FRONT_PROXY_MAX_BODY_BYTES.store(
        env_usize_or(
            ENV_FRONT_PROXY_MAX_BODY_BYTES,
            DEFAULT_FRONT_PROXY_MAX_BODY_BYTES,
        ),
        Ordering::Relaxed,
    );
    UPSTREAM_CONNECT_TIMEOUT_SECS.store(
        env_u64_or(
            ENV_UPSTREAM_CONNECT_TIMEOUT_SECS,
            DEFAULT_UPSTREAM_CONNECT_TIMEOUT_SECS,
        ),
        Ordering::Relaxed,
    );
    UPSTREAM_TOTAL_TIMEOUT_MS.store(
        env_u64_or(
            ENV_UPSTREAM_TOTAL_TIMEOUT_MS,
            DEFAULT_UPSTREAM_TOTAL_TIMEOUT_MS,
        ),
        Ordering::Relaxed,
    );
    UPSTREAM_STREAM_TIMEOUT_MS.store(
        env_u64_or(
            ENV_UPSTREAM_STREAM_TIMEOUT_MS,
            DEFAULT_UPSTREAM_STREAM_TIMEOUT_MS,
        ),
        Ordering::Relaxed,
    );
    ACCOUNT_MAX_INFLIGHT.store(
        env_usize_or(ENV_ACCOUNT_MAX_INFLIGHT, DEFAULT_ACCOUNT_MAX_INFLIGHT),
        Ordering::Relaxed,
    );
    CPA_NO_COOKIE_HEADER_MODE.store(
        env_bool_or(
            ENV_CPA_NO_COOKIE_HEADER_MODE,
            DEFAULT_CPA_NO_COOKIE_HEADER_MODE,
        ),
        Ordering::Relaxed,
    );
    STRICT_REQUEST_PARAM_ALLOWLIST.store(
        env_bool_or(
            ENV_STRICT_REQUEST_PARAM_ALLOWLIST,
            DEFAULT_STRICT_REQUEST_PARAM_ALLOWLIST,
        ),
        Ordering::Relaxed,
    );
    ENABLE_REQUEST_COMPRESSION.store(
        env_bool_or(
            ENV_ENABLE_REQUEST_COMPRESSION,
            DEFAULT_ENABLE_REQUEST_COMPRESSION,
        ),
        Ordering::Relaxed,
    );

    let cookie = env_non_empty(ENV_UPSTREAM_COOKIE);
    let mut cached_cookie =
        crate::lock_utils::write_recover(upstream_cookie_cell(), "upstream_cookie");
    *cached_cookie = cookie;

    let client_id = env_non_empty(ENV_TOKEN_EXCHANGE_CLIENT_ID)
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());
    let mut cached_client_id = crate::lock_utils::write_recover(
        token_exchange_client_id_cell(),
        "token_exchange_client_id",
    );
    *cached_client_id = client_id;

    let issuer =
        env_non_empty(ENV_TOKEN_EXCHANGE_ISSUER).unwrap_or_else(|| DEFAULT_ISSUER.to_string());
    let mut cached_issuer =
        crate::lock_utils::write_recover(token_exchange_issuer_cell(), "token_exchange_issuer");
    *cached_issuer = issuer;

    let proxy_url = env_non_empty(ENV_UPSTREAM_PROXY_URL);
    let converted_proxy = match normalize_upstream_proxy_url(proxy_url.as_deref()) {
        Ok(normalized) => normalized,
        Err(err) => {
            log::warn!(
                "event=gateway_invalid_upstream_proxy_url source=env var={} err={}",
                ENV_UPSTREAM_PROXY_URL,
                err
            );
            None
        }
    };
    let mut cached_proxy_url =
        crate::lock_utils::write_recover(upstream_proxy_url_cell(), "upstream_proxy_url");
    *cached_proxy_url = converted_proxy;
    drop(cached_proxy_url);

    let free_account_max_model = env_non_empty(ENV_FREE_ACCOUNT_MAX_MODEL)
        .and_then(|value| normalize_model_slug(value.as_str()).ok())
        .unwrap_or_else(|| DEFAULT_FREE_ACCOUNT_MAX_MODEL.to_string());
    let mut cached_free_account_max_model =
        crate::lock_utils::write_recover(free_account_max_model_cell(), "free_account_max_model");
    *cached_free_account_max_model = free_account_max_model;
    drop(cached_free_account_max_model);

    let payload_rewrite_rules = env_non_empty(ENV_PAYLOAD_REWRITE_RULES)
        .as_deref()
        .map_or_else(Vec::new, |raw| {
            match parse_payload_rewrite_rules_json(Some(raw)) {
                Ok(rules) => rules,
                Err(err) => {
                    log::warn!(
                        "event=gateway_invalid_payload_rewrite_rules source=env var={} err={}",
                        ENV_PAYLOAD_REWRITE_RULES,
                        err
                    );
                    Vec::new()
                }
            }
        });
    let mut cached_payload_rewrite_rules =
        crate::lock_utils::write_recover(payload_rewrite_rules_cell(), "payload_rewrite_rules");
    *cached_payload_rewrite_rules = payload_rewrite_rules;
    drop(cached_payload_rewrite_rules);

    let model_alias_pools =
        env_non_empty(ENV_MODEL_ALIAS_POOLS)
            .as_deref()
            .map_or_else(Vec::new, |raw| {
                match parse_model_alias_pools_json(Some(raw)) {
                    Ok(pools) => pools,
                    Err(err) => {
                        log::warn!(
                            "event=gateway_invalid_model_alias_pools source=env var={} err={}",
                            ENV_MODEL_ALIAS_POOLS,
                            err
                        );
                        Vec::new()
                    }
                }
            });
    let mut cached_model_alias_pools =
        crate::lock_utils::write_recover(model_alias_pools_cell(), "model_alias_pools");
    *cached_model_alias_pools = model_alias_pools;
    drop(cached_model_alias_pools);

    let originator = env_non_empty(ENV_ORIGINATOR)
        .and_then(|value| normalize_originator(value.as_str()).ok())
        .unwrap_or_else(|| DEFAULT_ORIGINATOR.to_string());
    let mut cached_originator = crate::lock_utils::write_recover(originator_cell(), "originator");
    *cached_originator = originator;
    drop(cached_originator);

    let residency_requirement = env_non_empty(ENV_RESIDENCY_REQUIREMENT)
        .and_then(|value| normalize_residency_requirement(Some(value.as_str())).ok())
        .flatten();
    let mut cached_residency =
        crate::lock_utils::write_recover(residency_requirement_cell(), "residency_requirement");
    *cached_residency = residency_requirement;
    drop(cached_residency);
}

pub(super) fn reload_from_env_and_refresh_clients() {
    reload_from_env();
    refresh_upstream_clients_from_runtime_config();
}

const ENV_UPSTREAM_COOKIE: &str = "CODEXMANAGER_UPSTREAM_COOKIE";

fn ensure_runtime_config_loaded() {
    // 中文注释：懒加载默认只刷新标量/字符串配置，避免在异步上下文里因为首次读取
    // front_proxy_max_body_bytes 等轻量配置而触发 reqwest::blocking::Client 初始化。
    let _ = RUNTIME_CONFIG_LOADED.get_or_init(reload_from_env);
}

fn upstream_client_lock() -> &'static RwLock<Client> {
    UPSTREAM_CLIENT.get_or_init(|| RwLock::new(build_upstream_client()))
}

fn upstream_client_pool_lock() -> &'static RwLock<UpstreamClientPool> {
    UPSTREAM_CLIENT_POOL.get_or_init(|| RwLock::new(build_upstream_client_pool()))
}

fn refresh_upstream_clients_from_runtime_config() {
    // 中文注释：纯配置读取路径（例如 app_settings/get）可能只需要刷新标量 runtime 状态，
    // 不应为了热重载而强制初始化 reqwest client；已初始化的 client 再按最新 env 重建即可。
    if let Some(lock) = UPSTREAM_CLIENT.get() {
        let client = build_upstream_client();
        let mut client_lock = crate::lock_utils::write_recover(lock, "upstream_client");
        *client_lock = client;
    }

    if let Some(lock) = UPSTREAM_CLIENT_POOL.get() {
        let pool = build_upstream_client_pool();
        let mut pool_lock = crate::lock_utils::write_recover(lock, "upstream_client_pool");
        *pool_lock = pool;
    }
}

fn build_upstream_client_pool() -> UpstreamClientPool {
    if current_upstream_proxy_url().is_some() {
        return UpstreamClientPool::default();
    }
    let raw_proxies = parse_proxy_list_env();
    if raw_proxies.is_empty() {
        return UpstreamClientPool::default();
    }
    let mut proxies = Vec::with_capacity(raw_proxies.len());
    let mut clients = Vec::with_capacity(raw_proxies.len());
    for proxy in raw_proxies.into_iter() {
        if let Err(err) = Proxy::all(proxy.as_str()) {
            log::warn!(
                "event=gateway_proxy_pool_invalid_proxy proxy={} err={}",
                proxy,
                err
            );
            continue;
        }
        let client = build_upstream_client_with_proxy(Some(proxy.as_str()));
        proxies.push(proxy);
        clients.push(client);
    }
    if clients.is_empty() {
        UpstreamClientPool::default()
    } else {
        log::info!(
            "event=gateway_proxy_pool_initialized size={}",
            clients.len()
        );
        UpstreamClientPool { proxies, clients }
    }
}

fn upstream_cookie_cell() -> &'static RwLock<Option<String>> {
    UPSTREAM_COOKIE.get_or_init(|| RwLock::new(None))
}

fn upstream_proxy_url_cell() -> &'static RwLock<Option<String>> {
    UPSTREAM_PROXY_URL.get_or_init(|| RwLock::new(None))
}

fn free_account_max_model_cell() -> &'static RwLock<String> {
    FREE_ACCOUNT_MAX_MODEL.get_or_init(|| RwLock::new(DEFAULT_FREE_ACCOUNT_MAX_MODEL.to_string()))
}

fn payload_rewrite_rules_cell() -> &'static RwLock<Vec<PayloadRewriteRule>> {
    PAYLOAD_REWRITE_RULES.get_or_init(|| RwLock::new(Vec::new()))
}

fn model_alias_pools_cell() -> &'static RwLock<Vec<ModelAliasPool>> {
    MODEL_ALIAS_POOLS.get_or_init(|| RwLock::new(Vec::new()))
}

fn originator_cell() -> &'static RwLock<String> {
    ORIGINATOR.get_or_init(|| RwLock::new(DEFAULT_ORIGINATOR.to_string()))
}

fn residency_requirement_cell() -> &'static RwLock<Option<String>> {
    RESIDENCY_REQUIREMENT.get_or_init(|| RwLock::new(None))
}

#[cfg(test)]
pub(crate) fn gateway_runtime_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GATEWAY_RUNTIME_TEST_MUTEX: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    GATEWAY_RUNTIME_TEST_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn current_upstream_proxy_url() -> Option<String> {
    crate::lock_utils::read_recover(upstream_proxy_url_cell(), "upstream_proxy_url").clone()
}

fn token_exchange_client_id_cell() -> &'static RwLock<String> {
    TOKEN_EXCHANGE_CLIENT_ID.get_or_init(|| RwLock::new(DEFAULT_CLIENT_ID.to_string()))
}

fn token_exchange_issuer_cell() -> &'static RwLock<String> {
    TOKEN_EXCHANGE_ISSUER.get_or_init(|| RwLock::new(DEFAULT_ISSUER.to_string()))
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u64_or(name: &str, default: u64) -> u64 {
    env_non_empty(name)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize_or(name: &str, default: usize) -> usize {
    env_non_empty(name)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_bool_or(name: &str, default: bool) -> bool {
    let Some(value) = env_non_empty(name) else {
        return default;
    };
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn normalize_payload_rewrite_rules_json(raw: Option<&str>) -> Result<String, String> {
    let rules = parse_payload_rewrite_rules_json(raw)?;
    serde_json::to_string(&rules)
        .map_err(|err| format!("serialize payload rewrite rules failed: {err}"))
}

fn normalize_model_alias_pools_json(raw: Option<&str>) -> Result<String, String> {
    let pools = parse_model_alias_pools_json(raw)?;
    serde_json::to_string(&pools)
        .map_err(|err| format!("serialize model alias pools failed: {err}"))
}

fn parse_payload_rewrite_rules_json(raw: Option<&str>) -> Result<Vec<PayloadRewriteRule>, String> {
    let Some(raw) = raw.map(str::trim) else {
        return Ok(Vec::new());
    };
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let rules = serde_json::from_str::<Vec<PayloadRewriteRule>>(raw)
        .map_err(|err| format!("invalid payload rewrite rules json: {err}"))?;
    validate_payload_rewrite_rules(&rules)?;
    Ok(rules)
}

fn parse_model_alias_pools_json(raw: Option<&str>) -> Result<Vec<ModelAliasPool>, String> {
    let Some(raw) = raw.map(str::trim) else {
        return Ok(Vec::new());
    };
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let pools = serde_json::from_str::<Vec<ModelAliasPool>>(raw)
        .map_err(|err| format!("invalid model alias pools json: {err}"))?;
    validate_model_alias_pools(&pools)?;
    Ok(pools)
}

fn validate_payload_rewrite_rules(rules: &[PayloadRewriteRule]) -> Result<(), String> {
    for (idx, rule) in rules.iter().enumerate() {
        let path = rule.path.trim();
        if path.is_empty() {
            return Err(format!("rule[{idx}].path is required"));
        }
        if path != "*" && !path.starts_with('/') {
            return Err(format!("rule[{idx}].path must start with '/' or be '*'"));
        }

        let field = rule.field.trim();
        if field.is_empty() {
            return Err(format!("rule[{idx}].field is required"));
        }
        if field.contains('.') || field.contains('[') || field.contains(']') {
            return Err(format!(
                "rule[{idx}].field only supports top-level json keys"
            ));
        }
        if field.eq_ignore_ascii_case("model") {
            return Err(format!(
                "rule[{idx}].field does not support 'model' in the first payload rewrite slice"
            ));
        }
    }
    Ok(())
}

fn validate_model_alias_pools(pools: &[ModelAliasPool]) -> Result<(), String> {
    let mut seen_aliases = std::collections::BTreeSet::new();
    for (pool_idx, pool) in pools.iter().enumerate() {
        let Some(alias) = normalize_alias_or_model_token(pool.alias.as_str()) else {
            return Err(format!("pool[{pool_idx}].alias is required"));
        };
        if !seen_aliases.insert(alias.clone()) {
            return Err(format!("pool[{pool_idx}].alias duplicates '{alias}'"));
        }
        if pool.targets.is_empty() {
            return Err(format!("pool[{pool_idx}].targets must not be empty"));
        }
        let enabled_targets = pool.targets.iter().filter(|target| target.enabled).count();
        if pool.enabled && enabled_targets == 0 {
            return Err(format!(
                "pool[{pool_idx}] must include at least one enabled target"
            ));
        }
        let mut total_weight = 0_u64;
        for (target_idx, target) in pool.targets.iter().enumerate() {
            if normalize_alias_or_model_token(target.model.as_str()).is_none() {
                return Err(format!(
                    "pool[{pool_idx}].targets[{target_idx}].model is required"
                ));
            }
            if target.enabled {
                total_weight += u64::from(target.weight);
            }
        }
        if pool.enabled
            && matches!(pool.strategy, ModelAliasPoolStrategy::Weighted)
            && total_weight == 0
        {
            return Err(format!(
                "pool[{pool_idx}] weighted strategy requires enabled targets with weight > 0"
            ));
        }
    }
    Ok(())
}

fn normalize_alias_or_model_token(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':'))
        .then_some(normalized)
}

fn select_model_alias_target<'a>(
    pool: &'a ModelAliasPool,
    trace_id: &str,
) -> Option<&'a ModelAliasPoolTarget> {
    let enabled_targets = pool
        .targets
        .iter()
        .filter(|target| target.enabled)
        .collect::<Vec<_>>();
    if enabled_targets.is_empty() {
        return None;
    }
    match pool.strategy {
        ModelAliasPoolStrategy::Ordered => enabled_targets.first().copied(),
        ModelAliasPoolStrategy::Weighted => {
            let total_weight = enabled_targets
                .iter()
                .map(|target| u64::from(target.weight))
                .sum::<u64>();
            if total_weight == 0 {
                return enabled_targets.first().copied();
            }
            let ticket = stable_account_hash(trace_id) % total_weight;
            let mut cursor = 0_u64;
            for target in enabled_targets {
                cursor += u64::from(target.weight);
                if ticket < cursor {
                    return Some(target);
                }
            }
            None
        }
    }
}

fn normalize_model_slug(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("freeAccountMaxModel is required".to_string());
    }
    if normalized == "auto" {
        return Ok(normalized);
    }
    if normalized == "gpt-5.4-pro" {
        return Ok("gpt-5.4".to_string());
    }
    if normalized
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':')))
    {
        return Err("freeAccountMaxModel contains unsupported characters".to_string());
    }
    Ok(normalized)
}

fn normalize_originator(raw: &str) -> Result<String, String> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err("originator is required".to_string());
    }
    if normalized.chars().any(|ch| ch.is_ascii_control()) {
        return Err("originator contains control characters".to_string());
    }
    Ok(normalized.to_string())
}

fn normalize_residency_requirement(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "us" => Ok(Some("us".to_string())),
        _ => Err("residencyRequirement only supports 'us' or empty".to_string()),
    }
}

fn rewrite_socks_proxy_url(proxy_url: &str) -> String {
    let mut normalized = proxy_url.trim().to_string();
    if let Some(rest) = normalized.strip_prefix("http://socks") {
        normalized = format!("socks{rest}");
    } else if let Some(rest) = normalized.strip_prefix("https://socks") {
        normalized = format!("socks{rest}");
    }
    if normalized.starts_with("socks5://") {
        normalized = normalized.replacen("socks5://", "socks5h://", 1);
    } else if normalized.starts_with("socks://") {
        normalized = normalized.replacen("socks://", "socks5h://", 1);
    }
    normalized
}

fn normalize_upstream_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    let mut normalized = proxy_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(value) = normalized.as_mut() {
        *value = rewrite_socks_proxy_url(value);
        Proxy::all(value.as_str()).map_err(|err| format!("invalid proxy url: {err}"))?;
    }
    Ok(normalized)
}

fn parse_proxy_list_env() -> Vec<String> {
    let Some(raw) = env_non_empty(ENV_PROXY_LIST) else {
        return Vec::new();
    };
    raw.split([',', ';', '\n', '\r'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .take(MAX_UPSTREAM_PROXY_POOL_SIZE)
        .map(rewrite_socks_proxy_url)
        .collect()
}

fn stable_proxy_index(account_id: &str, size: usize) -> Option<usize> {
    if size == 0 {
        return None;
    }
    if size == 1 {
        return Some(0);
    }
    let hash = stable_account_hash(account_id);
    Some((hash as usize) % size)
}

fn stable_account_hash(account_id: &str) -> u64 {
    // 中文注释：FNV-1a 保证跨进程稳定，不受 std 默认随机种子影响。
    let mut hash = 14695981039346656037_u64;
    for byte in account_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211_u64);
    }
    hash
}

#[cfg(test)]
#[path = "tests/runtime_config_tests.rs"]
mod tests;
