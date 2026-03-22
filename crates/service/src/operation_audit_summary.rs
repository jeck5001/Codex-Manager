use codexmanager_core::rpc::types::OperationAuditItem;

const DEFAULT_OPERATION_AUDIT_LIMIT: i64 = 8;

pub(crate) fn read_recent_operation_audits() -> Result<Vec<OperationAuditItem>, String> {
    let storage =
        crate::storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let events = storage
        .list_recent_events_by_type(
            crate::operation_audit::operation_audit_event_type(),
            0,
            DEFAULT_OPERATION_AUDIT_LIMIT,
        )
        .map_err(|err| format!("list operation audit events failed: {err}"))?;

    Ok(events
        .into_iter()
        .map(|event| {
            let parsed = serde_json::from_str::<serde_json::Value>(&event.message).ok();
            let action = parsed
                .as_ref()
                .and_then(|value| value.get("action"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("operation");
            let label = parsed
                .as_ref()
                .and_then(|value| value.get("label"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(action);
            let detail = parsed
                .as_ref()
                .and_then(|value| value.get("detail"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .unwrap_or(event.message.as_str());

            OperationAuditItem {
                action: action.to_string(),
                label: label.to_string(),
                detail: detail.to_string(),
                account_id: event.account_id,
                created_at: Some(event.created_at),
            }
        })
        .collect())
}
