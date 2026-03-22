use crate::storage_helpers::open_storage;

pub(crate) fn renew_api_key(key_id: &str, expires_at: Option<i64>) -> Result<(), String> {
    if key_id.is_empty() {
        return Err("key id required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let current = storage
        .find_api_key_by_id(key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "api key not found".to_string())?;

    storage
        .update_api_key_expiration(key_id, expires_at)
        .map_err(|e| e.to_string())?;

    let next_status = if current.status == "disabled" {
        "disabled"
    } else {
        "active"
    };
    storage
        .update_api_key_status(key_id, next_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}
