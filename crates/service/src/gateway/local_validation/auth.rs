use codexmanager_core::storage::{now_ts, ApiKey, Storage};

use crate::storage_helpers::{hash_platform_key, open_storage, StorageHandle};

pub(super) fn open_storage_or_error() -> Result<StorageHandle, super::LocalValidationError> {
    open_storage().ok_or_else(|| super::LocalValidationError::new(500, "storage unavailable"))
}

pub(super) fn load_active_api_key(
    storage: &Storage,
    platform_key: &str,
    request_url: &str,
    debug: bool,
) -> Result<ApiKey, super::LocalValidationError> {
    let key_hash = hash_platform_key(platform_key);
    let api_key = storage.find_api_key_by_hash(&key_hash).map_err(|err| {
        super::LocalValidationError::new(500, format!("storage read failed: {err}"))
    })?;

    let Some(api_key) = api_key else {
        if debug {
            log::warn!(
                "event=gateway_auth_invalid path={} status=403 key_hash_prefix={}",
                request_url,
                &key_hash[..8]
            );
        }
        return Err(super::LocalValidationError::new(403, "invalid api key"));
    };

    if api_key.status == "expired"
        || (api_key.status == "active"
            && api_key
                .expires_at
                .is_some_and(|expires_at| expires_at <= now_ts()))
    {
        if api_key.status != "expired" {
            let _ = storage.update_api_key_status(&api_key.id, "expired");
        }
        if debug {
            log::warn!(
                "event=gateway_auth_expired path={} status=401 key_id={}",
                request_url,
                api_key.id
            );
        }
        return Err(super::LocalValidationError::new(401, "api key expired"));
    }

    if api_key.status != "active" {
        if debug {
            log::warn!(
                "event=gateway_auth_disabled path={} status=403 key_id={}",
                request_url,
                api_key.id
            );
        }
        return Err(super::LocalValidationError::new(403, "api key disabled"));
    }

    Ok(api_key)
}

#[cfg(test)]
#[path = "tests/auth_tests.rs"]
mod tests;
