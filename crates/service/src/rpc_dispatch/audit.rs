use codexmanager_core::rpc::types::{
    AuditLogExportParams, AuditLogFilterParams, AuditLogListParams, JsonRpcRequest,
    JsonRpcResponse,
};

use crate::{audit_export, audit_list};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "audit/list" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<AuditLogListParams>)
                .transpose()
                .map(|params| params.unwrap_or_default())
                .map(AuditLogListParams::normalized)
                .map_err(|err| format!("invalid audit/list params: {err}"));
            super::value_or_error(params.and_then(audit_list::read_audit_log_page))
        }
        "audit/export" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<AuditLogExportParams>)
                .transpose()
                .map(|params| params.unwrap_or_default())
                .map_err(|err| format!("invalid audit/export params: {err}"));
            super::value_or_error(params.and_then(audit_export::export_audit_logs))
        }
        "audit/summary" => {
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<AuditLogFilterParams>)
                .transpose()
                .map(|params| params.unwrap_or_default())
                .map_err(|err| format!("invalid audit/summary params: {err}"));
            super::value_or_error(params.and_then(|filters| {
                let normalized = audit_list::normalize_filter_params(filters);
                let storage =
                    crate::storage_helpers::open_storage().ok_or_else(|| "open storage failed".to_string())?;
                let total = storage
                    .count_audit_logs_filtered(audit_list::to_storage_filters(&normalized))
                    .map_err(|err| format!("count audit logs failed: {err}"))?;
                Ok(serde_json::json!({ "totalCount": total }))
            }))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
