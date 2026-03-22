use codexmanager_core::rpc::types::ApiKeyModelFallbackConfig;

use crate::storage_helpers::open_storage;

const MAX_MODEL_CHAIN_LEN: usize = 8;

pub(crate) fn read_api_key_model_fallback(
    key_id: &str,
) -> Result<ApiKeyModelFallbackConfig, String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let config = storage
        .find_api_key_model_fallback_by_id(key_id)
        .map_err(|e| e.to_string())?;

    Ok(match config {
        Some(config) => ApiKeyModelFallbackConfig {
            key_id: config.key_id,
            model_chain: parse_model_chain(config.model_chain_json.as_str()),
        },
        None => ApiKeyModelFallbackConfig {
            key_id: key_id.to_string(),
            model_chain: Vec::new(),
        },
    })
}

pub(crate) fn update_api_key_model_fallback(
    key_id: &str,
    model_chain: Vec<String>,
) -> Result<(), String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .find_api_key_by_id(key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;

    let normalized = normalize_model_chain(model_chain)?;
    storage
        .upsert_api_key_model_fallback(key_id, normalized.as_slice())
        .map_err(|e| e.to_string())
}

pub(crate) fn parse_model_chain(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let trimmed = item.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

fn normalize_model_chain(model_chain: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for model in model_chain {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            continue;
        }
        if normalized.iter().any(|item| item == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }

    if normalized.len() > MAX_MODEL_CHAIN_LEN {
        return Err(format!(
            "model_chain supports at most {MAX_MODEL_CHAIN_LEN} models"
        ));
    }

    Ok(normalized)
}
