use codexmanager_core::rpc::types::ApiKeyAllowedModelsConfig;

use crate::storage_helpers::open_storage;

const MAX_ALLOWED_MODELS_LEN: usize = 64;

pub(crate) fn read_api_key_allowed_models(
    key_id: &str,
) -> Result<ApiKeyAllowedModelsConfig, String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .find_api_key_by_id(key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;
    let allowed_models = storage
        .find_api_key_allowed_models_by_id(key_id)
        .map_err(|e| e.to_string())?
        .map(|raw| parse_allowed_models(raw.as_str()))
        .unwrap_or_default();
    Ok(ApiKeyAllowedModelsConfig {
        key_id: key_id.to_string(),
        allowed_models,
    })
}

pub(crate) fn update_api_key_allowed_models(
    key_id: &str,
    allowed_models: Vec<String>,
) -> Result<(), String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .find_api_key_by_id(key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;

    let normalized = normalize_allowed_models(allowed_models)?;
    let payload = if normalized.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&normalized)
                .map_err(|err| format!("serialize allowed models failed: {err}"))?,
        )
    };
    storage
        .update_api_key_allowed_models(key_id, payload.as_deref())
        .map_err(|e| e.to_string())
}

pub(crate) fn parse_allowed_models(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let trimmed = item.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

fn normalize_allowed_models(allowed_models: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for model in allowed_models {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            continue;
        }
        if normalized.iter().any(|item| item == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }

    if normalized.len() > MAX_ALLOWED_MODELS_LEN {
        return Err(format!(
            "allowed_models supports at most {MAX_ALLOWED_MODELS_LEN} models"
        ));
    }

    Ok(normalized)
}
