use codexmanager_core::rpc::types::{
    AuditLogFilterParams, AuditLogItem, AuditLogListParams, AuditLogListResult,
};
use codexmanager_core::storage::{AuditLog, AuditLogFilterInput};

use crate::storage_helpers::open_storage;

pub(crate) fn normalize_filter_params(params: AuditLogFilterParams) -> AuditLogFilterParams {
    AuditLogFilterParams {
        action: normalize_optional_text(params.action),
        object_type: normalize_optional_text(params.object_type),
        object_id: normalize_optional_text(params.object_id),
        time_from: params.time_from,
        time_to: params.time_to,
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub(crate) fn to_storage_filters<'a>(params: &'a AuditLogFilterParams) -> AuditLogFilterInput<'a> {
    AuditLogFilterInput {
        action: params.action.as_deref(),
        object_type: params.object_type.as_deref(),
        object_id: params.object_id.as_deref(),
        time_from: params.time_from,
        time_to: params.time_to,
    }
}

pub(crate) fn to_audit_log_item(item: AuditLog) -> AuditLogItem {
    AuditLogItem {
        id: item.id,
        action: item.action,
        object_type: item.object_type,
        object_id: item.object_id,
        operator: item.operator,
        changes: serde_json::from_str::<serde_json::Value>(&item.changes_json)
            .unwrap_or(serde_json::Value::Null),
        created_at: item.created_at,
    }
}

pub(crate) fn read_audit_log_page(
    params: AuditLogListParams,
) -> Result<AuditLogListResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let normalized = params.normalized();
    let offset = (normalized.page - 1) * normalized.page_size;
    let filters = normalize_filter_params(normalized.filters);
    let total = storage
        .count_audit_logs_filtered(to_storage_filters(&filters))
        .map_err(|err| format!("count audit logs failed: {err}"))?;
    let items = storage
        .list_audit_logs_paginated_filtered(
            to_storage_filters(&filters),
            offset,
            normalized.page_size,
        )
        .map_err(|err| format!("list audit logs failed: {err}"))?
        .into_iter()
        .map(to_audit_log_item)
        .collect::<Vec<_>>();

    Ok(AuditLogListResult {
        items,
        total,
        page: normalized.page,
        page_size: normalized.page_size,
    })
}
