use serde::Serialize;
use std::sync::atomic::Ordering;

use super::{
    parse_interval_secs, BACKGROUND_TASKS_CONFIG_LOADED, BACKGROUND_TASK_RESTART_REQUIRED_KEYS,
    AUTO_REGISTER_POOL_ENABLED, AUTO_REGISTER_READY_ACCOUNT_COUNT,
    AUTO_REGISTER_READY_REMAIN_PERCENT, DEFAULT_AUTO_REGISTER_READY_ACCOUNT_COUNT,
    DEFAULT_AUTO_REGISTER_READY_REMAIN_PERCENT,
    AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED, AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
    AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
    AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
    DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
    DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
    DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
    DEFAULT_GATEWAY_KEEPALIVE_INTERVAL_SECS, DEFAULT_HTTP_STREAM_WORKER_FACTOR,
    DEFAULT_HTTP_STREAM_WORKER_MIN, DEFAULT_HTTP_WORKER_FACTOR, DEFAULT_HTTP_WORKER_MIN,
    DEFAULT_TOKEN_REFRESH_POLL_INTERVAL_SECS, DEFAULT_USAGE_POLL_INTERVAL_SECS,
    DEFAULT_USAGE_REFRESH_WORKERS, DEFAULT_SESSION_PROBE_INTERVAL_SECS,
    DEFAULT_SESSION_PROBE_SAMPLE_SIZE, ENV_DISABLE_POLLING, ENV_GATEWAY_KEEPALIVE_ENABLED,
    ENV_GATEWAY_KEEPALIVE_INTERVAL_SECS, ENV_HTTP_STREAM_WORKER_FACTOR, ENV_HTTP_STREAM_WORKER_MIN,
    ENV_HTTP_WORKER_FACTOR, ENV_HTTP_WORKER_MIN, ENV_TOKEN_REFRESH_POLLING_ENABLED,
    ENV_TOKEN_REFRESH_POLL_INTERVAL_SECS, ENV_USAGE_POLLING_ENABLED,
    ENV_USAGE_POLL_INTERVAL_SECS, ENV_AUTO_REGISTER_POOL_ENABLED,
    ENV_AUTO_REGISTER_READY_ACCOUNT_COUNT, ENV_AUTO_REGISTER_READY_REMAIN_PERCENT,
    ENV_SESSION_PROBE_INTERVAL_SECS, ENV_SESSION_PROBE_POLLING_ENABLED,
    ENV_SESSION_PROBE_SAMPLE_SIZE,
    ENV_AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED,
    ENV_AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
    ENV_AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
    ENV_AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
    GATEWAY_KEEPALIVE_ENABLED, GATEWAY_KEEPALIVE_INTERVAL_SECS, HTTP_STREAM_WORKER_FACTOR,
    HTTP_STREAM_WORKER_MIN, HTTP_WORKER_FACTOR, HTTP_WORKER_MIN,
    MIN_GATEWAY_KEEPALIVE_INTERVAL_SECS, MIN_SESSION_PROBE_INTERVAL_SECS,
    MIN_TOKEN_REFRESH_POLL_INTERVAL_SECS, MIN_USAGE_POLL_INTERVAL_SECS,
    SESSION_PROBE_INTERVAL_SECS, SESSION_PROBE_POLLING_ENABLED, SESSION_PROBE_SAMPLE_SIZE,
    TOKEN_REFRESH_POLLING_ENABLED,
    TOKEN_REFRESH_POLL_INTERVAL_SECS_ATOMIC, USAGE_POLLING_ENABLED, USAGE_POLL_INTERVAL_SECS,
    USAGE_REFRESH_WORKERS, USAGE_REFRESH_WORKERS_ENV,
};

const ENV_ACCOUNT_COOLDOWN_AUTH_SECS: &str = "CODEXMANAGER_ACCOUNT_COOLDOWN_AUTH_SECS";
const ENV_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS: &str =
    "CODEXMANAGER_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS";
const ENV_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS: &str =
    "CODEXMANAGER_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS";
const ENV_ACCOUNT_COOLDOWN_NETWORK_SECS: &str = "CODEXMANAGER_ACCOUNT_COOLDOWN_NETWORK_SECS";
const ENV_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS: &str = "CODEXMANAGER_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS";
const ENV_ACCOUNT_COOLDOWN_DEACTIVATED_SECS: &str =
    "CODEXMANAGER_ACCOUNT_COOLDOWN_DEACTIVATED_SECS";
const DEFAULT_ACCOUNT_COOLDOWN_AUTH_SECS: u64 = 300;
const DEFAULT_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS: u64 = 45;
const DEFAULT_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS: u64 = 30;
const DEFAULT_ACCOUNT_COOLDOWN_NETWORK_SECS: u64 = 20;
const DEFAULT_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS: u64 = 1800;
const DEFAULT_ACCOUNT_COOLDOWN_DEACTIVATED_SECS: u64 = 21600;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackgroundTasksSettings {
    usage_polling_enabled: bool,
    usage_poll_interval_secs: u64,
    gateway_keepalive_enabled: bool,
    gateway_keepalive_interval_secs: u64,
    token_refresh_polling_enabled: bool,
    token_refresh_poll_interval_secs: u64,
    session_probe_polling_enabled: bool,
    session_probe_interval_secs: u64,
    session_probe_sample_size: usize,
    usage_refresh_workers: usize,
    http_worker_factor: usize,
    http_worker_min: usize,
    http_stream_worker_factor: usize,
    http_stream_worker_min: usize,
    auto_register_pool_enabled: bool,
    auto_register_ready_account_count: usize,
    auto_register_ready_remain_percent: u64,
    auto_disable_risky_accounts_enabled: bool,
    auto_disable_risky_accounts_failure_threshold: usize,
    auto_disable_risky_accounts_health_score_threshold: usize,
    auto_disable_risky_accounts_lookback_mins: u64,
    account_cooldown_auth_secs: u64,
    account_cooldown_rate_limited_secs: u64,
    account_cooldown_server_error_secs: u64,
    account_cooldown_network_secs: u64,
    account_cooldown_low_quota_secs: u64,
    account_cooldown_deactivated_secs: u64,
    requires_restart_keys: Vec<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BackgroundTasksSettingsPatch {
    pub usage_polling_enabled: Option<bool>,
    pub usage_poll_interval_secs: Option<u64>,
    pub gateway_keepalive_enabled: Option<bool>,
    pub gateway_keepalive_interval_secs: Option<u64>,
    pub token_refresh_polling_enabled: Option<bool>,
    pub token_refresh_poll_interval_secs: Option<u64>,
    pub session_probe_polling_enabled: Option<bool>,
    pub session_probe_interval_secs: Option<u64>,
    pub session_probe_sample_size: Option<usize>,
    pub usage_refresh_workers: Option<usize>,
    pub http_worker_factor: Option<usize>,
    pub http_worker_min: Option<usize>,
    pub http_stream_worker_factor: Option<usize>,
    pub http_stream_worker_min: Option<usize>,
    pub auto_register_pool_enabled: Option<bool>,
    pub auto_register_ready_account_count: Option<usize>,
    pub auto_register_ready_remain_percent: Option<u64>,
    pub auto_disable_risky_accounts_enabled: Option<bool>,
    pub auto_disable_risky_accounts_failure_threshold: Option<usize>,
    pub auto_disable_risky_accounts_health_score_threshold: Option<usize>,
    pub auto_disable_risky_accounts_lookback_mins: Option<u64>,
    pub account_cooldown_auth_secs: Option<u64>,
    pub account_cooldown_rate_limited_secs: Option<u64>,
    pub account_cooldown_server_error_secs: Option<u64>,
    pub account_cooldown_network_secs: Option<u64>,
    pub account_cooldown_low_quota_secs: Option<u64>,
    pub account_cooldown_deactivated_secs: Option<u64>,
}

pub(crate) fn background_tasks_settings() -> BackgroundTasksSettings {
    ensure_background_tasks_config_loaded();
    BackgroundTasksSettings {
        usage_polling_enabled: USAGE_POLLING_ENABLED.load(Ordering::Relaxed),
        usage_poll_interval_secs: USAGE_POLL_INTERVAL_SECS.load(Ordering::Relaxed),
        gateway_keepalive_enabled: GATEWAY_KEEPALIVE_ENABLED.load(Ordering::Relaxed),
        gateway_keepalive_interval_secs: GATEWAY_KEEPALIVE_INTERVAL_SECS.load(Ordering::Relaxed),
        token_refresh_polling_enabled: TOKEN_REFRESH_POLLING_ENABLED.load(Ordering::Relaxed),
        token_refresh_poll_interval_secs: TOKEN_REFRESH_POLL_INTERVAL_SECS_ATOMIC
            .load(Ordering::Relaxed),
        session_probe_polling_enabled: SESSION_PROBE_POLLING_ENABLED.load(Ordering::Relaxed),
        session_probe_interval_secs: SESSION_PROBE_INTERVAL_SECS.load(Ordering::Relaxed),
        session_probe_sample_size: SESSION_PROBE_SAMPLE_SIZE.load(Ordering::Relaxed),
        usage_refresh_workers: USAGE_REFRESH_WORKERS.load(Ordering::Relaxed),
        http_worker_factor: HTTP_WORKER_FACTOR.load(Ordering::Relaxed),
        http_worker_min: HTTP_WORKER_MIN.load(Ordering::Relaxed),
        http_stream_worker_factor: HTTP_STREAM_WORKER_FACTOR.load(Ordering::Relaxed),
        http_stream_worker_min: HTTP_STREAM_WORKER_MIN.load(Ordering::Relaxed),
        auto_register_pool_enabled: AUTO_REGISTER_POOL_ENABLED.load(Ordering::Relaxed),
        auto_register_ready_account_count: AUTO_REGISTER_READY_ACCOUNT_COUNT
            .load(Ordering::Relaxed),
        auto_register_ready_remain_percent: AUTO_REGISTER_READY_REMAIN_PERCENT
            .load(Ordering::Relaxed),
        auto_disable_risky_accounts_enabled: AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED
            .load(Ordering::Relaxed),
        auto_disable_risky_accounts_failure_threshold:
            AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD.load(Ordering::Relaxed),
        auto_disable_risky_accounts_health_score_threshold:
            AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD.load(Ordering::Relaxed),
        auto_disable_risky_accounts_lookback_mins: AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS
            .load(Ordering::Relaxed),
        account_cooldown_auth_secs: account_cooldown_auth_secs(),
        account_cooldown_rate_limited_secs: account_cooldown_rate_limited_secs(),
        account_cooldown_server_error_secs: account_cooldown_server_error_secs(),
        account_cooldown_network_secs: account_cooldown_network_secs(),
        account_cooldown_low_quota_secs: account_cooldown_low_quota_secs(),
        account_cooldown_deactivated_secs: account_cooldown_deactivated_secs(),
        requires_restart_keys: BACKGROUND_TASK_RESTART_REQUIRED_KEYS.to_vec(),
    }
}

pub(crate) fn set_background_tasks_settings(
    patch: BackgroundTasksSettingsPatch,
) -> BackgroundTasksSettings {
    ensure_background_tasks_config_loaded();

    if let Some(enabled) = patch.usage_polling_enabled {
        USAGE_POLLING_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(ENV_USAGE_POLLING_ENABLED, if enabled { "1" } else { "0" });
        if enabled {
            std::env::remove_var(ENV_DISABLE_POLLING);
        } else {
            std::env::set_var(ENV_DISABLE_POLLING, "1");
        }
    }
    if let Some(secs) = patch.usage_poll_interval_secs {
        let normalized = secs.max(MIN_USAGE_POLL_INTERVAL_SECS);
        USAGE_POLL_INTERVAL_SECS.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_USAGE_POLL_INTERVAL_SECS, normalized.to_string());
    }
    if let Some(enabled) = patch.gateway_keepalive_enabled {
        GATEWAY_KEEPALIVE_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(
            ENV_GATEWAY_KEEPALIVE_ENABLED,
            if enabled { "1" } else { "0" },
        );
    }
    if let Some(secs) = patch.gateway_keepalive_interval_secs {
        let normalized = secs.max(MIN_GATEWAY_KEEPALIVE_INTERVAL_SECS);
        GATEWAY_KEEPALIVE_INTERVAL_SECS.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_GATEWAY_KEEPALIVE_INTERVAL_SECS, normalized.to_string());
    }
    if let Some(enabled) = patch.token_refresh_polling_enabled {
        TOKEN_REFRESH_POLLING_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(
            ENV_TOKEN_REFRESH_POLLING_ENABLED,
            if enabled { "1" } else { "0" },
        );
    }
    if let Some(secs) = patch.token_refresh_poll_interval_secs {
        let normalized = secs.max(MIN_TOKEN_REFRESH_POLL_INTERVAL_SECS);
        TOKEN_REFRESH_POLL_INTERVAL_SECS_ATOMIC.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_TOKEN_REFRESH_POLL_INTERVAL_SECS, normalized.to_string());
    }
    if let Some(enabled) = patch.session_probe_polling_enabled {
        SESSION_PROBE_POLLING_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(
            ENV_SESSION_PROBE_POLLING_ENABLED,
            if enabled { "1" } else { "0" },
        );
    }
    if let Some(secs) = patch.session_probe_interval_secs {
        let normalized = secs.max(MIN_SESSION_PROBE_INTERVAL_SECS);
        SESSION_PROBE_INTERVAL_SECS.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_SESSION_PROBE_INTERVAL_SECS, normalized.to_string());
    }
    if let Some(value) = patch.session_probe_sample_size {
        let normalized = value.max(1);
        SESSION_PROBE_SAMPLE_SIZE.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_SESSION_PROBE_SAMPLE_SIZE, normalized.to_string());
    }
    if let Some(workers) = patch.usage_refresh_workers {
        let normalized = workers.max(1);
        USAGE_REFRESH_WORKERS.store(normalized, Ordering::Relaxed);
        std::env::set_var(USAGE_REFRESH_WORKERS_ENV, normalized.to_string());
    }
    if let Some(value) = patch.http_worker_factor {
        let normalized = value.max(1);
        HTTP_WORKER_FACTOR.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_HTTP_WORKER_FACTOR, normalized.to_string());
    }
    if let Some(value) = patch.http_worker_min {
        let normalized = value.max(1);
        HTTP_WORKER_MIN.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_HTTP_WORKER_MIN, normalized.to_string());
    }
    if let Some(value) = patch.http_stream_worker_factor {
        let normalized = value.max(1);
        HTTP_STREAM_WORKER_FACTOR.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_HTTP_STREAM_WORKER_FACTOR, normalized.to_string());
    }
    if let Some(value) = patch.http_stream_worker_min {
        let normalized = value.max(1);
        HTTP_STREAM_WORKER_MIN.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_HTTP_STREAM_WORKER_MIN, normalized.to_string());
    }
    if let Some(enabled) = patch.auto_register_pool_enabled {
        AUTO_REGISTER_POOL_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(ENV_AUTO_REGISTER_POOL_ENABLED, if enabled { "1" } else { "0" });
    }
    if let Some(value) = patch.auto_register_ready_account_count {
        let normalized = value.max(1);
        AUTO_REGISTER_READY_ACCOUNT_COUNT.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_AUTO_REGISTER_READY_ACCOUNT_COUNT, normalized.to_string());
    }
    if let Some(value) = patch.auto_register_ready_remain_percent {
        let normalized = value.min(100);
        AUTO_REGISTER_READY_REMAIN_PERCENT.store(normalized, Ordering::Relaxed);
        std::env::set_var(ENV_AUTO_REGISTER_READY_REMAIN_PERCENT, normalized.to_string());
    }
    if let Some(enabled) = patch.auto_disable_risky_accounts_enabled {
        AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED.store(enabled, Ordering::Relaxed);
        std::env::set_var(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED,
            if enabled { "1" } else { "0" },
        );
    }
    if let Some(value) = patch.auto_disable_risky_accounts_failure_threshold {
        let normalized = value.max(1);
        AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD.store(normalized, Ordering::Relaxed);
        std::env::set_var(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
            normalized.to_string(),
        );
    }
    if let Some(value) = patch.auto_disable_risky_accounts_health_score_threshold {
        let normalized = value.clamp(1, 200);
        AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD.store(normalized, Ordering::Relaxed);
        std::env::set_var(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
            normalized.to_string(),
        );
    }
    if let Some(value) = patch.auto_disable_risky_accounts_lookback_mins {
        let normalized = value.max(1);
        AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS.store(normalized, Ordering::Relaxed);
        std::env::set_var(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
            normalized.to_string(),
        );
    }
    if let Some(value) = patch.account_cooldown_auth_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_AUTH_SECS, value.to_string());
    }
    if let Some(value) = patch.account_cooldown_rate_limited_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS, value.to_string());
    }
    if let Some(value) = patch.account_cooldown_server_error_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS, value.to_string());
    }
    if let Some(value) = patch.account_cooldown_network_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_NETWORK_SECS, value.to_string());
    }
    if let Some(value) = patch.account_cooldown_low_quota_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS, value.to_string());
    }
    if let Some(value) = patch.account_cooldown_deactivated_secs {
        std::env::set_var(ENV_ACCOUNT_COOLDOWN_DEACTIVATED_SECS, value.to_string());
    }

    background_tasks_settings()
}

pub(crate) fn reload_background_tasks_runtime_from_env() {
    reload_background_tasks_from_env();
}

pub(super) fn ensure_background_tasks_config_loaded() {
    let _ = BACKGROUND_TASKS_CONFIG_LOADED.get_or_init(reload_background_tasks_from_env);
}

fn reload_background_tasks_from_env() {
    let usage_polling_default_enabled = std::env::var(ENV_DISABLE_POLLING).is_err();
    USAGE_POLLING_ENABLED.store(
        env_bool_or(ENV_USAGE_POLLING_ENABLED, usage_polling_default_enabled),
        Ordering::Relaxed,
    );
    USAGE_POLL_INTERVAL_SECS.store(
        parse_interval_secs(
            std::env::var(ENV_USAGE_POLL_INTERVAL_SECS).ok().as_deref(),
            DEFAULT_USAGE_POLL_INTERVAL_SECS,
            MIN_USAGE_POLL_INTERVAL_SECS,
        ),
        Ordering::Relaxed,
    );
    GATEWAY_KEEPALIVE_ENABLED.store(
        env_bool_or(ENV_GATEWAY_KEEPALIVE_ENABLED, true),
        Ordering::Relaxed,
    );
    GATEWAY_KEEPALIVE_INTERVAL_SECS.store(
        parse_interval_secs(
            std::env::var(ENV_GATEWAY_KEEPALIVE_INTERVAL_SECS)
                .ok()
                .as_deref(),
            DEFAULT_GATEWAY_KEEPALIVE_INTERVAL_SECS,
            MIN_GATEWAY_KEEPALIVE_INTERVAL_SECS,
        ),
        Ordering::Relaxed,
    );
    TOKEN_REFRESH_POLLING_ENABLED.store(
        env_bool_or(ENV_TOKEN_REFRESH_POLLING_ENABLED, true),
        Ordering::Relaxed,
    );
    TOKEN_REFRESH_POLL_INTERVAL_SECS_ATOMIC.store(
        parse_interval_secs(
            std::env::var(ENV_TOKEN_REFRESH_POLL_INTERVAL_SECS)
                .ok()
                .as_deref(),
            DEFAULT_TOKEN_REFRESH_POLL_INTERVAL_SECS,
            MIN_TOKEN_REFRESH_POLL_INTERVAL_SECS,
        ),
        Ordering::Relaxed,
    );
    SESSION_PROBE_POLLING_ENABLED.store(
        env_bool_or(ENV_SESSION_PROBE_POLLING_ENABLED, false),
        Ordering::Relaxed,
    );
    SESSION_PROBE_INTERVAL_SECS.store(
        parse_interval_secs(
            std::env::var(ENV_SESSION_PROBE_INTERVAL_SECS)
                .ok()
                .as_deref(),
            DEFAULT_SESSION_PROBE_INTERVAL_SECS,
            MIN_SESSION_PROBE_INTERVAL_SECS,
        ),
        Ordering::Relaxed,
    );
    SESSION_PROBE_SAMPLE_SIZE.store(
        env_usize_or(
            ENV_SESSION_PROBE_SAMPLE_SIZE,
            DEFAULT_SESSION_PROBE_SAMPLE_SIZE,
        )
        .max(1),
        Ordering::Relaxed,
    );
    USAGE_REFRESH_WORKERS.store(
        env_usize_or(USAGE_REFRESH_WORKERS_ENV, DEFAULT_USAGE_REFRESH_WORKERS).max(1),
        Ordering::Relaxed,
    );
    HTTP_WORKER_FACTOR.store(
        env_usize_or(ENV_HTTP_WORKER_FACTOR, DEFAULT_HTTP_WORKER_FACTOR).max(1),
        Ordering::Relaxed,
    );
    HTTP_WORKER_MIN.store(
        env_usize_or(ENV_HTTP_WORKER_MIN, DEFAULT_HTTP_WORKER_MIN).max(1),
        Ordering::Relaxed,
    );
    HTTP_STREAM_WORKER_FACTOR.store(
        env_usize_or(
            ENV_HTTP_STREAM_WORKER_FACTOR,
            DEFAULT_HTTP_STREAM_WORKER_FACTOR,
        )
        .max(1),
        Ordering::Relaxed,
    );
    HTTP_STREAM_WORKER_MIN.store(
        env_usize_or(ENV_HTTP_STREAM_WORKER_MIN, DEFAULT_HTTP_STREAM_WORKER_MIN).max(1),
        Ordering::Relaxed,
    );
    AUTO_REGISTER_POOL_ENABLED.store(
        env_bool_or(ENV_AUTO_REGISTER_POOL_ENABLED, false),
        Ordering::Relaxed,
    );
    AUTO_REGISTER_READY_ACCOUNT_COUNT.store(
        env_usize_or(
            ENV_AUTO_REGISTER_READY_ACCOUNT_COUNT,
            DEFAULT_AUTO_REGISTER_READY_ACCOUNT_COUNT,
        )
        .max(1),
        Ordering::Relaxed,
    );
    AUTO_REGISTER_READY_REMAIN_PERCENT.store(
        env_u64_or(
            ENV_AUTO_REGISTER_READY_REMAIN_PERCENT,
            DEFAULT_AUTO_REGISTER_READY_REMAIN_PERCENT,
        )
        .min(100),
        Ordering::Relaxed,
    );
    AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED.store(
        env_bool_or(ENV_AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED, false),
        Ordering::Relaxed,
    );
    AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD.store(
        env_usize_or(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
            DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
        )
        .max(1),
        Ordering::Relaxed,
    );
    AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD.store(
        env_usize_or(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
            DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
        )
        .clamp(1, 200),
        Ordering::Relaxed,
    );
    AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS.store(
        env_u64_or(
            ENV_AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
            DEFAULT_AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
        )
        .max(1),
        Ordering::Relaxed,
    );
}

fn env_usize_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_bool_or(name: &str, default: bool) -> bool {
    let Some(raw) = std::env::var(name).ok() else {
        return default;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn env_u64_or(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

pub(crate) fn account_cooldown_auth_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_AUTH_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_AUTH_SECS,
    )
}

pub(crate) fn account_cooldown_rate_limited_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_RATE_LIMITED_SECS,
    )
}

pub(crate) fn account_cooldown_server_error_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_SERVER_ERROR_SECS,
    )
}

pub(crate) fn account_cooldown_network_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_NETWORK_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_NETWORK_SECS,
    )
}

pub(crate) fn account_cooldown_low_quota_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_LOW_QUOTA_SECS,
    )
}

pub(crate) fn account_cooldown_deactivated_secs() -> u64 {
    env_u64_or(
        ENV_ACCOUNT_COOLDOWN_DEACTIVATED_SECS,
        DEFAULT_ACCOUNT_COOLDOWN_DEACTIVATED_SECS,
    )
}
