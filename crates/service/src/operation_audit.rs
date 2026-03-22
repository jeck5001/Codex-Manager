use codexmanager_core::storage::{now_ts, Event};
use serde_json::json;

const OPERATION_AUDIT_EVENT_TYPE: &str = "operation_audit";

pub(crate) fn record_operation_audit(action: &str, label: &str, detail: impl Into<String>) {
    let Some(storage) = crate::storage_helpers::open_storage() else {
        return;
    };
    let payload = json!({
        "action": action,
        "label": label,
        "detail": detail.into(),
    });
    let _ = storage.insert_event(&Event {
        account_id: None,
        event_type: OPERATION_AUDIT_EVENT_TYPE.to_string(),
        message: payload.to_string(),
        created_at: now_ts(),
    });
}

pub(crate) fn operation_audit_event_type() -> &'static str {
    OPERATION_AUDIT_EVENT_TYPE
}
