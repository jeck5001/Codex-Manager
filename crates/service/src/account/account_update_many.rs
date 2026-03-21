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

    crate::operation_audit::record_operation_audit(
        if normalized_status == "disabled" {
            "account_bulk_disable"
        } else {
            "account_bulk_enable"
        },
        if normalized_status == "disabled" {
            "批量禁用账号"
        } else {
            "批量启用账号"
        },
        format!(
            "请求 {} 个，更新 {} 个，跳过 {} 个，失败 {} 个，目标状态 {}",
            result.requested,
            result.updated,
            result.skipped,
            result.failed,
            result.target_status
        ),
    );

    Ok(result)
}

pub(crate) fn update_accounts_tags(
    account_ids: Vec<String>,
    tags: Option<&str>,
) -> Result<UpdateManyResult, String> {
    let normalized_tags = normalize_tags(tags);
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
    let existing_tags_map = storage.list_account_tags().unwrap_or_default();
    let mut result = UpdateManyResult {
        requested: unique.len(),
        updated: 0,
        skipped: 0,
        failed: 0,
        target_status: "tags".to_string(),
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

        let current_tags = existing_tags_map
            .get(&account.id)
            .cloned()
            .flatten()
            .unwrap_or_default();
        if normalize_tags(Some(current_tags.as_str())) == normalized_tags {
            result.skipped += 1;
            result.skipped_account_ids.push(account_id);
            continue;
        }

        storage
            .update_account_tags(&account.id, normalized_tags.as_deref())
            .map_err(|err| err.to_string())?;
        result.updated += 1;
        result.updated_account_ids.push(account.id.clone());
    }

    let detail = if let Some(tags) = normalized_tags.as_deref() {
        format!(
            "请求 {} 个，更新 {} 个，跳过 {} 个，失败 {} 个，标签 {}",
            result.requested, result.updated, result.skipped, result.failed, tags
        )
    } else {
        format!(
            "请求 {} 个，更新 {} 个，跳过 {} 个，失败 {} 个，标签已清空",
            result.requested, result.updated, result.skipped, result.failed
        )
    };
    crate::operation_audit::record_operation_audit(
        "account_bulk_update_tags",
        "批量更新账号标签",
        detail,
    );

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

fn normalize_tags(tags: Option<&str>) -> Option<String> {
    let normalized = tags
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.join(","))
    }
}
