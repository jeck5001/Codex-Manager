use bytes::Bytes;
use codexmanager_core::rpc::types::{
    AuditLogExportParams, AuditLogExportResult, AuditLogFilterParams, AuditLogItem,
};
use std::convert::Infallible;

use crate::storage_helpers::open_storage;

use super::list::{normalize_filter_params, to_audit_log_item, to_storage_filters};

const AUDIT_LOG_EXPORT_CSV_HEADER: &str =
    "id,action,objectType,objectId,operator,changes,createdAt";
const AUDIT_LOG_EXPORT_BATCH_SIZE: i64 = 500;

pub(crate) struct AuditLogExportPlan {
    pub(crate) format: &'static str,
    pub(crate) file_name: String,
    pub(crate) filters: AuditLogFilterParams,
}

fn normalize_export_format(value: Option<String>) -> Result<&'static str, String> {
    match value
        .unwrap_or_else(|| "csv".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "csv" => Ok("csv"),
        "json" => Ok("json"),
        other => Err(format!("unsupported audit log export format: {other}")),
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn optional_string(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn json_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn append_csv_row(output: &mut String, item: &AuditLogItem) {
    let columns = [
        item.id.to_string(),
        item.action.clone(),
        item.object_type.clone(),
        optional_string(item.object_id.as_deref()),
        item.operator.clone(),
        json_string(&item.changes),
        item.created_at.to_string(),
    ];
    output.push_str(
        &columns
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push('\n');
}

fn build_audit_log_export_csv(items: &[AuditLogItem]) -> String {
    let mut output = String::from(AUDIT_LOG_EXPORT_CSV_HEADER);
    output.push('\n');
    for item in items {
        append_csv_row(&mut output, item);
    }
    output
}

fn build_audit_log_export_file_name(format: &str, action: Option<&str>) -> String {
    let scope = action
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("all");
    format!("codexmanager-auditlogs-{scope}.{format}")
}

pub(crate) fn prepare_audit_log_export(
    params: AuditLogExportParams,
) -> Result<AuditLogExportPlan, String> {
    let filters = normalize_filter_params(params.filters);
    let format = normalize_export_format(params.format)?;
    let file_name = build_audit_log_export_file_name(format, filters.action.as_deref());
    Ok(AuditLogExportPlan {
        format,
        file_name,
        filters,
    })
}

pub(crate) fn stream_audit_log_export_chunks(
    plan: AuditLogExportPlan,
    sender: tokio::sync::mpsc::Sender<Result<Bytes, Infallible>>,
) -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let mut offset = 0_i64;
    let mut streamed_any = false;

    if plan.format == "csv" {
        sender
            .blocking_send(Ok(Bytes::from(format!("{AUDIT_LOG_EXPORT_CSV_HEADER}\n"))))
            .map_err(|_| "audit log export stream closed".to_string())?;
    }

    loop {
        let rows = storage
            .list_audit_logs_paginated_filtered(
                to_storage_filters(&plan.filters),
                offset,
                AUDIT_LOG_EXPORT_BATCH_SIZE,
            )
            .map_err(|err| format!("list audit logs failed: {err}"))?;
        if rows.is_empty() {
            break;
        }

        let items = rows.into_iter().map(to_audit_log_item).collect::<Vec<_>>();
        let mut chunk = String::new();
        match plan.format {
            "csv" => {
                for item in &items {
                    append_csv_row(&mut chunk, item);
                }
            }
            "json" => {
                for item in &items {
                    if streamed_any {
                        chunk.push(',');
                    } else {
                        chunk.push('[');
                        streamed_any = true;
                    }
                    chunk.push('\n');
                    chunk.push_str(
                        &serde_json::to_string(item)
                            .map_err(|err| format!("serialize audit logs failed: {err}"))?,
                    );
                }
            }
            _ => unreachable!(),
        }
        if !chunk.is_empty() {
            sender
                .blocking_send(Ok(Bytes::from(chunk)))
                .map_err(|_| "audit log export stream closed".to_string())?;
        }
        offset += items.len() as i64;
    }

    if plan.format == "json" {
        let tail = if streamed_any { "\n]" } else { "[]" };
        sender
            .blocking_send(Ok(Bytes::from(tail.to_string())))
            .map_err(|_| "audit log export stream closed".to_string())?;
    }

    Ok(())
}

pub(crate) fn export_audit_logs(
    params: AuditLogExportParams,
) -> Result<AuditLogExportResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let plan = prepare_audit_log_export(params)?;
    let total = storage
        .count_audit_logs_filtered(to_storage_filters(&plan.filters))
        .map_err(|err| format!("count audit logs failed: {err}"))?;
    let items = if total > 0 {
        storage
            .list_audit_logs_paginated_filtered(to_storage_filters(&plan.filters), 0, total)
            .map_err(|err| format!("list audit logs failed: {err}"))?
            .into_iter()
            .map(to_audit_log_item)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let content = match plan.format {
        "json" => serde_json::to_string_pretty(&items)
            .map_err(|err| format!("serialize audit logs failed: {err}"))?,
        _ => build_audit_log_export_csv(&items),
    };
    Ok(AuditLogExportResult {
        format: plan.format.to_string(),
        file_name: plan.file_name,
        content,
        record_count: items.len() as i64,
    })
}
