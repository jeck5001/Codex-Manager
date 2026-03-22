use codexmanager_core::storage::now_ts;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const ROUTE_LATENCY_TTL_SECS: i64 = 24 * 60 * 60;
const ROUTE_LATENCY_CLEANUP_INTERVAL_SECS: i64 = 60;
const DEFAULT_EMA_ALPHA: f64 = 0.35;

#[derive(Debug, Clone, Default)]
struct RouteLatencyRecord {
    avg_latency_ms: f64,
    sample_count: u32,
    updated_at: i64,
}

#[derive(Default)]
struct RouteLatencyState {
    entries: HashMap<String, RouteLatencyRecord>,
    last_cleanup_at: i64,
}

static ROUTE_LATENCY: OnceLock<Mutex<RouteLatencyState>> = OnceLock::new();

pub(crate) fn record_route_latency(account_id: &str, duration_ms: u128) {
    let normalized = account_id.trim();
    if normalized.is_empty() {
        return;
    }
    let duration_ms = duration_ms.min(i64::MAX as u128) as f64;
    let lock = ROUTE_LATENCY.get_or_init(|| Mutex::new(RouteLatencyState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_latency_state");
    let now = now_ts();
    maybe_cleanup_route_latency(&mut state, now);
    let entry = state.entries.entry(normalized.to_string()).or_default();
    if entry.sample_count == 0 {
        entry.avg_latency_ms = duration_ms;
    } else {
        entry.avg_latency_ms =
            entry.avg_latency_ms * (1.0 - DEFAULT_EMA_ALPHA) + duration_ms * DEFAULT_EMA_ALPHA;
    }
    entry.sample_count = entry.sample_count.saturating_add(1);
    entry.updated_at = now;
}

pub(crate) fn average_route_latency_ms(account_id: &str) -> Option<i64> {
    let normalized = account_id.trim();
    if normalized.is_empty() {
        return None;
    }
    let lock = ROUTE_LATENCY.get_or_init(|| Mutex::new(RouteLatencyState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_latency_state");
    let now = now_ts();
    let record = state.entries.get(normalized).cloned()?;
    if route_latency_record_expired(&record, now) {
        state.entries.remove(normalized);
        return None;
    }
    Some(record.avg_latency_ms.round() as i64)
}

pub(super) fn clear_runtime_state() {
    let lock = ROUTE_LATENCY.get_or_init(|| Mutex::new(RouteLatencyState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_latency_state");
    state.entries.clear();
    state.last_cleanup_at = 0;
}

#[cfg(test)]
pub(crate) fn clear_route_latency_for_tests() {
    clear_runtime_state();
}

fn maybe_cleanup_route_latency(state: &mut RouteLatencyState, now: i64) {
    if state.last_cleanup_at != 0
        && now.saturating_sub(state.last_cleanup_at) < ROUTE_LATENCY_CLEANUP_INTERVAL_SECS
    {
        return;
    }
    state.last_cleanup_at = now;
    state
        .entries
        .retain(|_, value| !route_latency_record_expired(value, now));
}

fn route_latency_record_expired(record: &RouteLatencyRecord, now: i64) -> bool {
    record.updated_at + ROUTE_LATENCY_TTL_SECS <= now
}

#[cfg(test)]
mod tests {
    use super::{average_route_latency_ms, clear_route_latency_for_tests, record_route_latency};

    #[test]
    fn route_latency_ema_tracks_recent_values() {
        clear_route_latency_for_tests();
        record_route_latency("acc-1", 100);
        assert_eq!(average_route_latency_ms("acc-1"), Some(100));
        record_route_latency("acc-1", 200);
        let avg = average_route_latency_ms("acc-1").expect("avg");
        assert!((135..=170).contains(&avg));
    }
}
