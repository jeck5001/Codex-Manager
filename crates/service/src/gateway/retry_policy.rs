use serde::Serialize;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

const DEFAULT_RETRY_POLICY_MAX_RETRIES: usize = 3;
const MAX_RETRY_POLICY_MAX_RETRIES: usize = 10;
const DEFAULT_FIXED_BACKOFF_DELAY_MS: u64 = 250;
const DEFAULT_EXPONENTIAL_BACKOFF_BASE_MS: u64 = 150;
const DEFAULT_EXPONENTIAL_BACKOFF_CAP_MS: u64 = 1_500;
const DEFAULT_RETRYABLE_STATUS_CODES: &[u16] = &[429, 500, 502, 503];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryBackoffStrategy {
    Immediate,
    Fixed,
    Exponential,
}

impl RetryBackoffStrategy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Fixed => "fixed",
            Self::Exponential => "exponential",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "immediate" => Ok(Self::Immediate),
            "fixed" => Ok(Self::Fixed),
            "exponential" => Ok(Self::Exponential),
            other => Err(format!(
                "retryPolicyBackoffStrategy must be one of immediate / fixed / exponential, got {other}"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicySnapshot {
    pub max_retries: usize,
    pub backoff_strategy: String,
    pub retryable_status_codes: Vec<u16>,
}

#[derive(Debug, Clone)]
struct RetryPolicyState {
    max_retries: usize,
    backoff_strategy: RetryBackoffStrategy,
    retryable_status_codes: Vec<u16>,
}

impl Default for RetryPolicyState {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_RETRY_POLICY_MAX_RETRIES,
            backoff_strategy: RetryBackoffStrategy::Exponential,
            retryable_status_codes: DEFAULT_RETRYABLE_STATUS_CODES.to_vec(),
        }
    }
}

impl RetryPolicyState {
    fn snapshot(&self) -> RetryPolicySnapshot {
        RetryPolicySnapshot {
            max_retries: self.max_retries,
            backoff_strategy: self.backoff_strategy.as_str().to_string(),
            retryable_status_codes: self.retryable_status_codes.clone(),
        }
    }
}

fn retry_policy_state() -> &'static RwLock<RetryPolicyState> {
    static RETRY_POLICY_STATE: OnceLock<RwLock<RetryPolicyState>> = OnceLock::new();
    RETRY_POLICY_STATE.get_or_init(|| RwLock::new(RetryPolicyState::default()))
}

fn normalize_max_retries(max_retries: usize) -> Result<usize, String> {
    if max_retries > MAX_RETRY_POLICY_MAX_RETRIES {
        return Err(format!(
            "retryPolicyMaxRetries must be less than or equal to {MAX_RETRY_POLICY_MAX_RETRIES}"
        ));
    }
    Ok(max_retries)
}

fn normalize_retryable_status_codes(
    mut retryable_status_codes: Vec<u16>,
) -> Result<Vec<u16>, String> {
    for status in &retryable_status_codes {
        if !(100..=599).contains(status) {
            return Err(format!(
                "retryPolicyRetryableStatusCodes contains invalid HTTP status code: {status}"
            ));
        }
    }
    retryable_status_codes.sort_unstable();
    retryable_status_codes.dedup();
    Ok(retryable_status_codes)
}

fn cap_retry_delay(delay: Duration, deadline: Option<Instant>) -> Option<Duration> {
    let Some(deadline) = deadline else {
        return Some(delay);
    };
    let now = Instant::now();
    if now >= deadline {
        return None;
    }
    Some(delay.min(deadline.saturating_duration_since(now)))
}

pub(crate) fn current_retry_policy() -> RetryPolicySnapshot {
    retry_policy_state()
        .read()
        .expect("retry policy state should not be poisoned")
        .snapshot()
}

pub(crate) fn retry_policy_max_retries() -> usize {
    current_retry_policy().max_retries
}

pub(crate) fn retry_policy_allows_status(status_code: u16) -> bool {
    retry_policy_state()
        .read()
        .expect("retry policy state should not be poisoned")
        .retryable_status_codes
        .contains(&status_code)
}

pub(crate) fn should_failover_status(status_code: u16, has_more_candidates: bool) -> bool {
    if !has_more_candidates {
        return false;
    }
    matches!(status_code, 401 | 403) || retry_policy_allows_status(status_code)
}

pub(crate) fn set_retry_policy(
    max_retries: usize,
    backoff_strategy: &str,
    retryable_status_codes: Vec<u16>,
) -> Result<RetryPolicySnapshot, String> {
    let max_retries = normalize_max_retries(max_retries)?;
    let backoff_strategy = RetryBackoffStrategy::parse(backoff_strategy)?;
    let retryable_status_codes = normalize_retryable_status_codes(retryable_status_codes)?;

    let mut state = retry_policy_state()
        .write()
        .expect("retry policy state should not be poisoned");
    state.max_retries = max_retries;
    state.backoff_strategy = backoff_strategy;
    state.retryable_status_codes = retryable_status_codes;
    Ok(state.snapshot())
}

pub(crate) fn sleep_before_retry(attempt: usize, deadline: Option<Instant>) -> bool {
    let state = retry_policy_state()
        .read()
        .expect("retry policy state should not be poisoned")
        .clone();
    let delay = match state.backoff_strategy {
        RetryBackoffStrategy::Immediate => Duration::from_millis(0),
        RetryBackoffStrategy::Fixed => Duration::from_millis(DEFAULT_FIXED_BACKOFF_DELAY_MS),
        RetryBackoffStrategy::Exponential => {
            let multiplier = 1_u64 << attempt.min(10);
            let delay_ms = DEFAULT_EXPONENTIAL_BACKOFF_BASE_MS
                .saturating_mul(multiplier)
                .min(DEFAULT_EXPONENTIAL_BACKOFF_CAP_MS);
            Duration::from_millis(delay_ms)
        }
    };
    let Some(delay) = cap_retry_delay(delay, deadline) else {
        return false;
    };
    if !delay.is_zero() {
        std::thread::sleep(delay);
    }
    true
}

#[cfg(test)]
pub(crate) fn reset_retry_policy_for_tests() {
    let mut state = retry_policy_state()
        .write()
        .expect("retry policy state should not be poisoned");
    *state = RetryPolicyState::default();
}

#[cfg(test)]
pub(crate) fn retry_policy_test_guard() -> MutexGuard<'static, ()> {
    static RETRY_POLICY_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    crate::lock_utils::lock_recover(
        RETRY_POLICY_TEST_MUTEX.get_or_init(|| Mutex::new(())),
        "retry policy test mutex",
    )
}
