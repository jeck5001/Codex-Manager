use codexmanager_core::storage::Account;
use serde::Serialize;
use std::collections::HashSet;

use crate::{account_status, storage_helpers::open_storage};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateManyError {
    account_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateManyResult {
    requested: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    target_status: String,
    updated_account_ids: Vec<String>,
    skipped_account_ids: Vec<String>,
    errors: Vec<UpdateManyError>,
}

pub(crate) fn update_accounts_status(
    account_ids: Vec<String>,
    status: &str,
) -> Result<UpdateManyResult, String> {
    let normalized_status = normalize_account_status(status)?;
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for account_id in account_ids {
        let normalized = account_id.trim();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            unique.push(normalized.to_string());
        }
    }

    if unique.is_empty() {
        return Err("missing accountIds".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut result = UpdateManyResult {
        requested: unique.len(),
        updated: 0,
        skipped: 0,
        failed: 0,
        target_status: normalized_status.to_string(),
        updated_account_ids: Vec::new(),
        skipped_account_ids: Vec::new(),
        errors: Vec::new(),
    };

    for account_id in unique {
        let account = match storage
            .find_account_by_id(&account_id)
            .map_err(|err| err.to_string())?
        {
            Some(value) => value,
            None => {
                result.failed += 1;
                result.errors.push(UpdateManyError {
                    account_id,
                    message: "account not found".to_string(),
                });
                continue;
            }
        };

        if account_status_matches(&account, normalized_status) {
            result.skipped += 1;
            result.skipped_account_ids.push(account_id);
            continue;
        }

        let reason = if normalized_status == "disabled" {
            "manual_disable_many"
        } else {
            "manual_enable_many"
        };
        account_status::set_account_status(&storage, &account_id, normalized_status, reason);
        result.updated += 1;
        result.updated_account_ids.push(account_id);
    }

    Ok(result)
}

fn account_status_matches(account: &Account, status: &str) -> bool {
    match normalize_account_status(account.status.as_str()) {
        Ok(current) => current == status,
        Err(_) => false,
    }
}

fn normalize_account_status(status: &str) -> Result<&'static str, String> {
    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "active" => Ok("active"),
        "disabled" | "inactive" => Ok("disabled"),
        _ => Err(format!("unsupported account status: {status}")),
    }
}
