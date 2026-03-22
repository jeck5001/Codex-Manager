use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use codexmanager_core::storage::{ApiKey, Storage};
use serde_json::Value;

use crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE;

#[derive(Debug, Clone, Default)]
struct BucketState {
    tokens: f64,
    last_refill_ms: i64,
}

#[derive(Debug, Clone, Default)]
struct RateLimitState {
    request_bucket: BucketState,
    token_bucket: BucketState,
    day_index: i64,
    daily_count: i64,
}

static API_KEY_RATE_LIMIT_STATE: OnceLock<Mutex<HashMap<String, RateLimitState>>> = OnceLock::new();

pub(super) fn check_api_key_rate_limit(
    storage: &Storage,
    api_key: &ApiKey,
    body: &[u8],
    request_url: &str,
    debug: bool,
) -> Result<(), super::LocalValidationError> {
    let Some(config) = storage
        .find_api_key_rate_limit_by_id(&api_key.id)
        .map_err(|err| {
            super::LocalValidationError::new(500, format!("storage read failed: {err}"))
        })?
    else {
        return Ok(());
    };

    let now_secs = current_unix_seconds();
    let now_ms = now_secs.saturating_mul(1000);
    let mut state_map = state_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = state_map.get(&api_key.id).cloned().unwrap_or_default();
    let mut draft = current.clone();

    if let Some(limit) = positive_limit(config.rpm) {
        let retry_after_secs = consume_bucket(&mut draft.request_bucket, limit, 60, 1, now_ms)?;
        if retry_after_secs > 0 {
            if debug {
                log::warn!(
                    "event=gateway_rate_limit_exceeded type=rpm path={} key_id={} retry_after_secs={}",
                    request_url,
                    api_key.id,
                    retry_after_secs
                );
            }
            return Err(
                super::LocalValidationError::new(429, "api key rpm limit exceeded")
                    .with_retry_after_secs(retry_after_secs),
            );
        }
    }

    if let Some(limit) = positive_limit(config.tpm) {
        let estimated_tokens = estimate_input_tokens(api_key.protocol_type.as_str(), body);
        let retry_after_secs =
            consume_bucket(&mut draft.token_bucket, limit, 60, estimated_tokens, now_ms)?;
        if retry_after_secs > 0 {
            if debug {
                log::warn!(
                    "event=gateway_rate_limit_exceeded type=tpm path={} key_id={} retry_after_secs={} estimated_tokens={}",
                    request_url,
                    api_key.id,
                    retry_after_secs,
                    estimated_tokens
                );
            }
            return Err(
                super::LocalValidationError::new(429, "api key tpm limit exceeded")
                    .with_retry_after_secs(retry_after_secs),
            );
        }
    }

    if let Some(limit) = positive_limit(config.daily_limit) {
        let retry_after_secs = check_daily_limit(&mut draft, limit, now_secs)?;
        if retry_after_secs > 0 {
            if debug {
                log::warn!(
                    "event=gateway_rate_limit_exceeded type=daily path={} key_id={} retry_after_secs={}",
                    request_url,
                    api_key.id,
                    retry_after_secs
                );
            }
            return Err(
                super::LocalValidationError::new(429, "api key daily limit exceeded")
                    .with_retry_after_secs(retry_after_secs),
            );
        }
    }

    state_map.insert(api_key.id.clone(), draft);
    Ok(())
}

fn positive_limit(value: Option<i64>) -> Option<u64> {
    value.filter(|value| *value > 0).map(|value| value as u64)
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn state_map() -> &'static Mutex<HashMap<String, RateLimitState>> {
    API_KEY_RATE_LIMIT_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn consume_bucket(
    bucket: &mut BucketState,
    capacity: u64,
    window_secs: u64,
    amount: u64,
    now_ms: i64,
) -> Result<u64, super::LocalValidationError> {
    if amount == 0 {
        return Ok(0);
    }
    if amount > capacity {
        return Ok(window_secs.max(1));
    }
    if bucket.last_refill_ms <= 0 {
        bucket.tokens = capacity as f64;
        bucket.last_refill_ms = now_ms;
    } else {
        let elapsed_ms = now_ms.saturating_sub(bucket.last_refill_ms);
        let refill_rate_per_ms = capacity as f64 / (window_secs as f64 * 1000.0);
        bucket.tokens =
            (bucket.tokens + elapsed_ms as f64 * refill_rate_per_ms).min(capacity as f64);
        bucket.last_refill_ms = now_ms;
    }

    let needed = amount as f64;
    if bucket.tokens + f64::EPSILON >= needed {
        bucket.tokens -= needed;
        return Ok(0);
    }

    let missing = needed - bucket.tokens;
    let refill_rate_per_sec = capacity as f64 / window_secs as f64;
    let retry_after_secs = (missing / refill_rate_per_sec).ceil() as u64;
    Ok(retry_after_secs.max(1))
}

fn check_daily_limit(
    state: &mut RateLimitState,
    daily_limit: u64,
    now_secs: i64,
) -> Result<u64, super::LocalValidationError> {
    let day_index = now_secs.div_euclid(86_400);
    if state.day_index != day_index {
        state.day_index = day_index;
        state.daily_count = 0;
    }
    if state.daily_count + 1 > daily_limit as i64 {
        let next_day_start = (day_index + 1) * 86_400;
        return Ok(next_day_start.saturating_sub(now_secs).max(1) as u64);
    }
    state.daily_count += 1;
    Ok(0)
}

fn estimate_input_tokens(protocol_type: &str, body: &[u8]) -> u64 {
    if body.is_empty() {
        return 1;
    }

    if protocol_type == PROTOCOL_ANTHROPIC_NATIVE {
        return estimate_text_tokens(body).max(1);
    }

    estimate_text_tokens(body).max(((body.len() as u64) / 4).max(1))
}

fn estimate_text_tokens(body: &[u8]) -> u64 {
    let Ok(payload) = serde_json::from_slice::<Value>(body) else {
        return 1;
    };
    let char_count = accumulate_text_len(&payload) as u64;
    (char_count / 4).max(1)
}

fn accumulate_text_len(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.iter().map(accumulate_text_len).sum(),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.chars().count();
            }
            if let Some(prompt) = map.get("prompt").and_then(Value::as_str) {
                return prompt.chars().count();
            }
            if let Some(content) = map.get("content") {
                return accumulate_text_len(content);
            }
            if let Some(input) = map.get("input") {
                return accumulate_text_len(input);
            }
            if let Some(messages) = map.get("messages") {
                return accumulate_text_len(messages);
            }
            map.values().map(accumulate_text_len).sum()
        }
        _ => 0,
    }
}

#[cfg(test)]
fn clear_state_for_tests() {
    state_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

#[cfg(test)]
#[path = "tests/rate_limit_tests.rs"]
mod tests;
