use codexmanager_core::storage::{now_ts, Event, Storage};

pub(crate) fn set_account_status(storage: &Storage, account_id: &str, status: &str, reason: &str) {
    if matches!(
        storage.update_account_status_if_changed(account_id, status),
        Ok(true)
    ) {
        let _ = storage.insert_event(&Event {
            account_id: Some(account_id.to_string()),
            event_type: "account_status_update".to_string(),
            message: format!("status={status} reason={reason}"),
            created_at: now_ts(),
        });
    }
}

pub(crate) fn is_refresh_token_auth_error(err: &str) -> bool {
    let normalized = err.trim().to_ascii_lowercase();
    if normalized.contains("refresh token failed with status 401") {
        return true;
    }

    let status_400_or_403 = normalized.contains("refresh token failed with status 400")
        || normalized.contains("refresh token failed with status 403");
    if !status_400_or_403 {
        return false;
    }

    normalized.contains("invalid_grant")
        || normalized.contains("invalid_request")
        || normalized.contains("invalid refresh token")
        || normalized.contains("refresh token is invalid")
        || normalized.contains("refresh token expired")
        || normalized.contains("refresh token revoked")
        || normalized.contains("token is expired")
        || normalized.contains("token revoked")
}

pub(crate) fn mark_account_inactive_for_refresh_token_error(
    storage: &Storage,
    account_id: &str,
    err: &str,
) -> bool {
    if !is_refresh_token_auth_error(err) {
        return false;
    }
    set_account_status(storage, account_id, "inactive", "refresh_token_invalid");
    true
}
