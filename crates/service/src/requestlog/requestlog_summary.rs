use codexmanager_core::rpc::types::{RequestLogFilterParams, RequestLogFilterSummaryResult};
use codexmanager_core::storage::RequestLogFilterInput;

use crate::storage_helpers::open_storage;

use super::list::{normalize_filter_params, to_storage_filters};

pub(crate) fn read_request_log_filter_summary(
    params: RequestLogFilterParams,
) -> Result<RequestLogFilterSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let params = normalize_filter_params(params);
    let total_count = storage
        .count_request_logs_filtered(RequestLogFilterInput {
            query: params.query.as_deref(),
            status_filter: None,
            key_id: params.key_id.as_deref(),
            key_ids: params.key_ids.as_slice(),
            model: params.model.as_deref(),
            time_from: params.time_from,
            time_to: params.time_to,
        })
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let filtered = storage
        .summarize_request_logs_filtered_with_filters(to_storage_filters(&params, None, None))
        .map_err(|err| format!("summarize request logs failed: {err}"))?;

    Ok(RequestLogFilterSummaryResult {
        total_count,
        filtered_count: filtered.count,
        success_count: filtered.success_count,
        error_count: filtered.error_count,
        total_tokens: filtered.total_tokens,
    })
}
