use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tiny_http::{Header, Request, Response, StatusCode};

use super::request_log::RequestLogUsage;

pub(crate) const RESPONSE_CACHE_HEADER_NAME: &str = "X-CodexManager-Cache";
const ACTUAL_MODEL_HEADER_NAME: &str = "X-CodexManager-Actual-Model";

const DEFAULT_RESPONSE_CACHE_ENABLED: bool = false;
const DEFAULT_RESPONSE_CACHE_TTL_SECS: u64 = 3600;
const DEFAULT_RESPONSE_CACHE_MAX_ENTRIES: usize = 256;
const MAX_RESPONSE_CACHE_ENTRIES: usize = 10_000;

const ENV_RESPONSE_CACHE_ENABLED: &str = "CODEXMANAGER_RESPONSE_CACHE_ENABLED";
const ENV_RESPONSE_CACHE_TTL_SECS: &str = "CODEXMANAGER_RESPONSE_CACHE_TTL_SECS";
const ENV_RESPONSE_CACHE_MAX_ENTRIES: &str = "CODEXMANAGER_RESPONSE_CACHE_MAX_ENTRIES";

static RESPONSE_CACHE_CONFIG_LOADED: OnceLock<()> = OnceLock::new();
static RESPONSE_CACHE_ENABLED: AtomicBool = AtomicBool::new(DEFAULT_RESPONSE_CACHE_ENABLED);
static RESPONSE_CACHE_TTL_SECS: AtomicU64 = AtomicU64::new(DEFAULT_RESPONSE_CACHE_TTL_SECS);
static RESPONSE_CACHE_MAX_ENTRIES: AtomicUsize =
    AtomicUsize::new(DEFAULT_RESPONSE_CACHE_MAX_ENTRIES);
static RESPONSE_CACHE_STATE: OnceLock<Mutex<ResponseCacheState>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseCacheConfigSnapshot {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseCacheStatsSnapshot {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: usize,
    pub entry_count: usize,
    pub estimated_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate_percent: f64,
}

#[derive(Debug, Clone)]
pub(in crate::gateway) struct CachedGatewayResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub usage: RequestLogUsage,
    pub actual_model: Option<String>,
}

#[derive(Debug, Clone)]
struct ResponseCacheEntry {
    response: CachedGatewayResponse,
    expires_at: Instant,
    estimated_bytes: usize,
}

#[derive(Debug, Default)]
struct ResponseCacheState {
    entries: HashMap<String, ResponseCacheEntry>,
    lru: VecDeque<String>,
    estimated_bytes: usize,
    hit_count: u64,
    miss_count: u64,
}

impl ResponseCacheState {
    fn remove_key(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.estimated_bytes = self.estimated_bytes.saturating_sub(entry.estimated_bytes);
        }
        self.lru.retain(|item| item != key);
    }

    fn touch_key(&mut self, key: &str) {
        self.lru.retain(|item| item != key);
        self.lru.push_back(key.to_string());
    }

    fn purge_expired(&mut self, now: Instant) {
        let expired_keys = self
            .entries
            .iter()
            .filter_map(|(key, entry)| (entry.expires_at <= now).then_some(key.clone()))
            .collect::<Vec<_>>();
        for key in expired_keys {
            self.remove_key(&key);
        }
    }

    fn trim_to_capacity(&mut self, max_entries: usize) {
        while self.entries.len() > max_entries {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            if self.entries.remove(&oldest).is_some() {
                continue;
            }
        }
        self.estimated_bytes = self
            .entries
            .values()
            .map(|entry| entry.estimated_bytes)
            .sum();
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.lru.clear();
        self.estimated_bytes = 0;
        self.hit_count = 0;
        self.miss_count = 0;
    }
}

fn response_cache_state() -> &'static Mutex<ResponseCacheState> {
    RESPONSE_CACHE_STATE.get_or_init(|| Mutex::new(ResponseCacheState::default()))
}

fn ensure_runtime_loaded() {
    RESPONSE_CACHE_CONFIG_LOADED.get_or_init(|| {
        reload_from_env();
    });
}

fn env_bool_or(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => crate::app_settings::parse_bool_with_default(&value, default),
        Err(_) => default,
    }
}

fn env_u64_or(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize_or(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn normalize_ttl_secs(ttl_secs: u64) -> Result<u64, String> {
    if ttl_secs == 0 {
        return Err("response cache ttlSecs must be greater than 0".to_string());
    }
    Ok(ttl_secs)
}

fn normalize_max_entries(max_entries: usize) -> Result<usize, String> {
    if max_entries == 0 {
        return Err("response cache maxEntries must be greater than 0".to_string());
    }
    if max_entries > MAX_RESPONSE_CACHE_ENTRIES {
        return Err(format!(
            "response cache maxEntries must be less than or equal to {MAX_RESPONSE_CACHE_ENTRIES}"
        ));
    }
    Ok(max_entries)
}

pub(crate) fn reload_from_env() {
    let enabled = env_bool_or(ENV_RESPONSE_CACHE_ENABLED, DEFAULT_RESPONSE_CACHE_ENABLED);
    let ttl_secs = env_u64_or(ENV_RESPONSE_CACHE_TTL_SECS, DEFAULT_RESPONSE_CACHE_TTL_SECS).max(1);
    let max_entries = env_usize_or(
        ENV_RESPONSE_CACHE_MAX_ENTRIES,
        DEFAULT_RESPONSE_CACHE_MAX_ENTRIES,
    )
    .clamp(1, MAX_RESPONSE_CACHE_ENTRIES);
    RESPONSE_CACHE_ENABLED.store(enabled, Ordering::Relaxed);
    RESPONSE_CACHE_TTL_SECS.store(ttl_secs, Ordering::Relaxed);
    RESPONSE_CACHE_MAX_ENTRIES.store(max_entries, Ordering::Relaxed);
    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    state.purge_expired(Instant::now());
    state.trim_to_capacity(max_entries);
}

pub(crate) fn response_cache_enabled() -> bool {
    ensure_runtime_loaded();
    RESPONSE_CACHE_ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn current_response_cache_ttl_secs() -> u64 {
    ensure_runtime_loaded();
    RESPONSE_CACHE_TTL_SECS.load(Ordering::Relaxed).max(1)
}

pub(crate) fn current_response_cache_max_entries() -> usize {
    ensure_runtime_loaded();
    RESPONSE_CACHE_MAX_ENTRIES
        .load(Ordering::Relaxed)
        .clamp(1, MAX_RESPONSE_CACHE_ENTRIES)
}

pub(crate) fn current_response_cache_config() -> ResponseCacheConfigSnapshot {
    ResponseCacheConfigSnapshot {
        enabled: response_cache_enabled(),
        ttl_secs: current_response_cache_ttl_secs(),
        max_entries: current_response_cache_max_entries(),
    }
}

pub(crate) fn set_response_cache_enabled(enabled: bool) -> bool {
    ensure_runtime_loaded();
    RESPONSE_CACHE_ENABLED.store(enabled, Ordering::Relaxed);
    std::env::set_var(ENV_RESPONSE_CACHE_ENABLED, if enabled { "1" } else { "0" });
    enabled
}

pub(crate) fn set_response_cache_ttl_secs(ttl_secs: u64) -> Result<u64, String> {
    ensure_runtime_loaded();
    let ttl_secs = normalize_ttl_secs(ttl_secs)?;
    RESPONSE_CACHE_TTL_SECS.store(ttl_secs, Ordering::Relaxed);
    std::env::set_var(ENV_RESPONSE_CACHE_TTL_SECS, ttl_secs.to_string());
    Ok(ttl_secs)
}

pub(crate) fn set_response_cache_max_entries(max_entries: usize) -> Result<usize, String> {
    ensure_runtime_loaded();
    let max_entries = normalize_max_entries(max_entries)?;
    RESPONSE_CACHE_MAX_ENTRIES.store(max_entries, Ordering::Relaxed);
    std::env::set_var(ENV_RESPONSE_CACHE_MAX_ENTRIES, max_entries.to_string());
    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    state.purge_expired(Instant::now());
    state.trim_to_capacity(max_entries);
    Ok(max_entries)
}

pub(crate) fn current_response_cache_stats() -> ResponseCacheStatsSnapshot {
    ensure_runtime_loaded();
    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    state.purge_expired(Instant::now());
    let total = state.hit_count.saturating_add(state.miss_count);
    ResponseCacheStatsSnapshot {
        enabled: response_cache_enabled(),
        ttl_secs: current_response_cache_ttl_secs(),
        max_entries: current_response_cache_max_entries(),
        entry_count: state.entries.len(),
        estimated_bytes: state.estimated_bytes,
        hit_count: state.hit_count,
        miss_count: state.miss_count,
        hit_rate_percent: if total == 0 {
            0.0
        } else {
            (state.hit_count as f64 / total as f64) * 100.0
        },
    }
}

pub(crate) fn clear_response_cache() -> ResponseCacheStatsSnapshot {
    ensure_runtime_loaded();
    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    state.clear();
    drop(state);
    current_response_cache_stats()
}

pub(crate) fn is_cacheable_request_path(path: &str) -> bool {
    path.starts_with("/v1/responses")
        || path.starts_with("/v1/chat/completions")
        || path.starts_with("/v1/completions")
        || path.starts_with("/v1/messages")
        || path.starts_with("/v1/embeddings")
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(canonicalize_json).collect::<Vec<_>>())
        }
        Value::Object(object) => {
            let mut pairs = object.into_iter().collect::<Vec<_>>();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            let mut normalized = Map::new();
            for (key, value) in pairs {
                normalized.insert(key, canonicalize_json(value));
            }
            Value::Object(normalized)
        }
        other => other,
    }
}

pub(crate) fn build_response_cache_key(path: &str, body: &[u8]) -> Option<String> {
    ensure_runtime_loaded();
    if !response_cache_enabled() || !is_cacheable_request_path(path) || body.is_empty() {
        return None;
    }

    let mut value = serde_json::from_slice::<Value>(body).ok()?;
    let object = value.as_object_mut()?;
    if object
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    if !object.contains_key("input")
        && !object.contains_key("messages")
        && !object.contains_key("prompt")
    {
        return None;
    }
    object.remove("stream");

    let normalized = canonicalize_json(value);
    let normalized_body = serde_json::to_vec(&normalized).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(&normalized_body);
    Some(format!("{:x}", hasher.finalize()))
}

pub(in crate::gateway) fn lookup_response_cache(key: &str) -> Option<CachedGatewayResponse> {
    ensure_runtime_loaded();
    if !response_cache_enabled() {
        return None;
    }

    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    let now = Instant::now();
    state.purge_expired(now);
    let response = state.entries.get(key).cloned().map(|entry| entry.response);
    if let Some(response) = response {
        state.hit_count = state.hit_count.saturating_add(1);
        state.touch_key(key);
        Some(response)
    } else {
        state.miss_count = state.miss_count.saturating_add(1);
        None
    }
}

pub(in crate::gateway) fn store_response_cache_entry(
    key: Option<&str>,
    status_code: u16,
    content_type: &str,
    body: &[u8],
    usage: RequestLogUsage,
    actual_model: Option<&str>,
) {
    let Some(key) = key.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    ensure_runtime_loaded();
    if !response_cache_enabled() || !(200..400).contains(&status_code) || body.is_empty() {
        return;
    }

    let ttl_secs = current_response_cache_ttl_secs();
    let max_entries = current_response_cache_max_entries();
    let response = CachedGatewayResponse {
        status_code,
        content_type: content_type.trim().to_string(),
        body: body.to_vec(),
        usage,
        actual_model: actual_model
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    };
    let estimated_bytes = key.len()
        + response.content_type.len()
        + response.body.len()
        + response.actual_model.as_deref().map(str::len).unwrap_or(0)
        + 64;
    let entry = ResponseCacheEntry {
        response,
        expires_at: Instant::now() + Duration::from_secs(ttl_secs),
        estimated_bytes,
    };

    let mut state = crate::lock_utils::lock_recover(response_cache_state(), "response_cache_state");
    state.purge_expired(Instant::now());
    state.remove_key(key);
    state.entries.insert(key.to_string(), entry);
    state.touch_key(key);
    state.estimated_bytes = state
        .entries
        .values()
        .map(|item| item.estimated_bytes)
        .sum();
    state.trim_to_capacity(max_entries);
}

fn push_optional_header(headers: &mut Vec<Header>, name: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
        headers.push(header);
    }
}

pub(crate) fn append_cache_status_header(headers: &mut Vec<Header>, value: &str) {
    push_optional_header(headers, RESPONSE_CACHE_HEADER_NAME, Some(value));
}

pub(in crate::gateway) fn respond_with_cached_response(
    request: Request,
    trace_id: &str,
    cached: &CachedGatewayResponse,
) -> Result<(), String> {
    let mut headers = Vec::new();
    push_optional_header(&mut headers, "Content-Type", Some(&cached.content_type));
    push_optional_header(
        &mut headers,
        crate::error_codes::TRACE_ID_HEADER_NAME,
        Some(trace_id),
    );
    push_optional_header(
        &mut headers,
        ACTUAL_MODEL_HEADER_NAME,
        cached.actual_model.as_deref(),
    );
    append_cache_status_header(&mut headers, "HIT");
    let body = cached.body.clone();
    let response = Response::new(
        StatusCode(cached.status_code),
        headers,
        std::io::Cursor::new(body.clone()),
        Some(body.len()),
        None,
    );
    request.respond(response).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static RESPONSE_CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_test() -> std::sync::MutexGuard<'static, ()> {
        RESPONSE_CACHE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn reset_response_cache_state() {
        clear_response_cache();
        set_response_cache_enabled(false);
        let _ = set_response_cache_ttl_secs(DEFAULT_RESPONSE_CACHE_TTL_SECS);
        let _ = set_response_cache_max_entries(DEFAULT_RESPONSE_CACHE_MAX_ENTRIES);
    }

    #[test]
    fn build_response_cache_key_skips_stream_requests() {
        let _lock = lock_test();
        reset_response_cache_state();
        set_response_cache_enabled(true);

        let body = br#"{"model":"gpt-5.3-codex","input":"hello","stream":true}"#;
        assert!(build_response_cache_key("/v1/responses", body).is_none());

        reset_response_cache_state();
    }

    #[test]
    fn response_cache_entry_expires_after_ttl() {
        let _lock = lock_test();
        reset_response_cache_state();
        set_response_cache_enabled(true);
        let _ = set_response_cache_ttl_secs(1);

        store_response_cache_entry(
            Some("ttl-key"),
            200,
            "application/json",
            br#"{"ok":true}"#,
            RequestLogUsage::default(),
            Some("gpt-5.3-codex"),
        );
        assert!(lookup_response_cache("ttl-key").is_some());

        std::thread::sleep(Duration::from_millis(1_100));
        assert!(lookup_response_cache("ttl-key").is_none());

        reset_response_cache_state();
    }

    #[test]
    fn response_cache_evicts_oldest_entry_when_capacity_is_exceeded() {
        let _lock = lock_test();
        reset_response_cache_state();
        set_response_cache_enabled(true);
        let _ = set_response_cache_max_entries(1);

        store_response_cache_entry(
            Some("first-key"),
            200,
            "application/json",
            br#"{"id":"first"}"#,
            RequestLogUsage::default(),
            Some("gpt-5.3-codex"),
        );
        store_response_cache_entry(
            Some("second-key"),
            200,
            "application/json",
            br#"{"id":"second"}"#,
            RequestLogUsage::default(),
            Some("gpt-5.3-codex"),
        );

        assert!(lookup_response_cache("first-key").is_none());
        assert!(lookup_response_cache("second-key").is_some());

        reset_response_cache_state();
    }
}
