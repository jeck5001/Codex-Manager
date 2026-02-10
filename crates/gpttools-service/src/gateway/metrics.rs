use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static ACCOUNT_INFLIGHT: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();
static GATEWAY_TOTAL_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static GATEWAY_ACTIVE_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static GATEWAY_FAILOVER_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
static GATEWAY_COOLDOWN_MARKS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GatewayMetricsSnapshot {
    pub total_requests: usize,
    pub active_requests: usize,
    pub account_inflight_total: usize,
    pub failover_attempts: usize,
    pub cooldown_marks: usize,
}

pub(crate) struct GatewayRequestGuard;

impl Drop for GatewayRequestGuard {
    fn drop(&mut self) {
        GATEWAY_ACTIVE_REQUESTS.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(crate) fn begin_gateway_request() -> GatewayRequestGuard {
    GATEWAY_TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
    GATEWAY_ACTIVE_REQUESTS.fetch_add(1, Ordering::Relaxed);
    GatewayRequestGuard
}

pub(crate) fn record_gateway_failover_attempt() {
    GATEWAY_FAILOVER_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_gateway_cooldown_mark() {
    GATEWAY_COOLDOWN_MARKS.fetch_add(1, Ordering::Relaxed);
}

fn account_inflight_total() -> usize {
    let lock = ACCOUNT_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(map) = lock.lock() else {
        return 0;
    };
    map.values().copied().sum()
}

pub(crate) fn gateway_metrics_snapshot() -> GatewayMetricsSnapshot {
    GatewayMetricsSnapshot {
        total_requests: GATEWAY_TOTAL_REQUESTS.load(Ordering::Relaxed),
        active_requests: GATEWAY_ACTIVE_REQUESTS.load(Ordering::Relaxed),
        account_inflight_total: account_inflight_total(),
        failover_attempts: GATEWAY_FAILOVER_ATTEMPTS.load(Ordering::Relaxed),
        cooldown_marks: GATEWAY_COOLDOWN_MARKS.load(Ordering::Relaxed),
    }
}

pub(crate) fn gateway_metrics_prometheus() -> String {
    let m = gateway_metrics_snapshot();
    format!(
        "gpttools_gateway_requests_total {}\n\
gpttools_gateway_requests_active {}\n\
gpttools_gateway_account_inflight_total {}\n\
gpttools_gateway_failover_attempts_total {}\n\
gpttools_gateway_cooldown_marks_total {}\n",
        m.total_requests,
        m.active_requests,
        m.account_inflight_total,
        m.failover_attempts,
        m.cooldown_marks,
    )
}

pub(crate) fn account_inflight_count(account_id: &str) -> usize {
    let lock = ACCOUNT_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(map) = lock.lock() else {
        return 0;
    };
    map.get(account_id).copied().unwrap_or(0)
}

pub(crate) struct AccountInFlightGuard {
    account_id: String,
}

impl Drop for AccountInFlightGuard {
    fn drop(&mut self) {
        let lock = ACCOUNT_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(mut map) = lock.lock() else {
            return;
        };
        if let Some(value) = map.get_mut(&self.account_id) {
            if *value > 1 {
                *value -= 1;
            } else {
                map.remove(&self.account_id);
            }
        }
    }
}

pub(crate) fn acquire_account_inflight(account_id: &str) -> AccountInFlightGuard {
    let lock = ACCOUNT_INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut map) = lock.lock() {
        let entry = map.entry(account_id.to_string()).or_insert(0);
        *entry += 1;
    }
    AccountInFlightGuard {
        account_id: account_id.to_string(),
    }
}
