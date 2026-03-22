use codexmanager_core::rpc::types::ApiKeyResponseCacheConfig;

use crate::storage_helpers::open_storage;

pub(crate) fn read_api_key_response_cache(
    key_id: &str,
) -> Result<ApiKeyResponseCacheConfig, String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let config = storage
        .find_api_key_response_cache_config_by_id(key_id)
        .map_err(|err| err.to_string())?;

    Ok(match config {
        Some(config) => ApiKeyResponseCacheConfig {
            key_id: config.key_id,
            enabled: config.enabled,
        },
        None => ApiKeyResponseCacheConfig {
            key_id: key_id.to_string(),
            enabled: false,
        },
    })
}

pub(crate) fn update_api_key_response_cache(key_id: &str, enabled: bool) -> Result<(), String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .find_api_key_by_id(key_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;
    storage
        .upsert_api_key_response_cache_config(key_id, enabled)
        .map_err(|err| err.to_string())
}
