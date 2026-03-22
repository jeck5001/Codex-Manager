use codexmanager_core::rpc::types::ApiKeyRateLimitConfig;

use crate::storage_helpers::open_storage;

pub(crate) fn read_api_key_rate_limit(key_id: &str) -> Result<ApiKeyRateLimitConfig, String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let config = storage
        .find_api_key_rate_limit_by_id(key_id)
        .map_err(|e| e.to_string())?;

    Ok(match config {
        Some(config) => ApiKeyRateLimitConfig {
            key_id: config.key_id,
            rpm: config.rpm,
            tpm: config.tpm,
            daily_limit: config.daily_limit,
        },
        None => ApiKeyRateLimitConfig {
            key_id: key_id.to_string(),
            rpm: None,
            tpm: None,
            daily_limit: None,
        },
    })
}

pub(crate) fn update_api_key_rate_limit(
    key_id: &str,
    rpm: Option<i64>,
    tpm: Option<i64>,
    daily_limit: Option<i64>,
) -> Result<(), String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .find_api_key_by_id(key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;

    storage
        .upsert_api_key_rate_limit(
            key_id,
            normalize_limit(rpm, "rpm")?,
            normalize_limit(tpm, "tpm")?,
            normalize_limit(daily_limit, "daily_limit")?,
        )
        .map_err(|e| e.to_string())
}

fn normalize_limit(value: Option<i64>, field: &str) -> Result<Option<i64>, String> {
    match value {
        None => Ok(None),
        Some(value) if value <= 0 => Err(format!("{field} must be greater than 0")),
        Some(value) => Ok(Some(value)),
    }
}
