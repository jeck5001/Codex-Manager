use codexmanager_core::rpc::types::{
    GatewayErrorLogListParams, GatewayErrorLogListResult, GatewayErrorLogSummary,
};
use codexmanager_core::storage::GatewayErrorLog;

use crate::storage_helpers::open_storage;

const DEFAULT_GATEWAY_ERROR_LOG_PAGE_SIZE: i64 = 10;
const MAX_GATEWAY_ERROR_LOG_PAGE_SIZE: i64 = 200;

/// 函数 `read_gateway_error_logs`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-04
///
/// # 参数
/// - params: 参数 params
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn read_gateway_error_logs(
    params: GatewayErrorLogListParams,
) -> Result<GatewayErrorLogListResult, String> {
    let params = params.normalized();
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let stage_filter = normalize_stage_filter(params.stage_filter);
    let page_size = normalize_page_size(params.page_size);
    let total = storage
        .count_gateway_error_logs(stage_filter.as_deref())
        .map_err(|err| format!("count gateway error logs failed: {err}"))?;
    let page = clamp_page(params.page, total, page_size);
    let offset = (page - 1) * page_size;
    let items = storage
        .list_gateway_error_logs_paginated(stage_filter.as_deref(), offset, page_size)
        .map_err(|err| format!("list gateway error logs failed: {err}"))?;
    let stages = storage
        .list_gateway_error_log_stages()
        .map_err(|err| format!("list gateway error log stages failed: {err}"))?;

    Ok(GatewayErrorLogListResult {
        items: items.into_iter().map(to_gateway_error_log_summary).collect(),
        total,
        page,
        page_size,
        stages,
    })
}

fn normalize_stage_filter(value: Option<String>) -> Option<String> {
    let normalized = value.unwrap_or_default().trim().to_string();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("all") {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_page_size(value: i64) -> i64 {
    if value < 1 {
        DEFAULT_GATEWAY_ERROR_LOG_PAGE_SIZE
    } else {
        value.min(MAX_GATEWAY_ERROR_LOG_PAGE_SIZE)
    }
}

fn clamp_page(page: i64, total: i64, page_size: i64) -> i64 {
    let normalized_page = page.max(1);
    let total_pages = if total <= 0 {
        1
    } else {
        ((total + page_size - 1) / page_size).max(1)
    };
    normalized_page.min(total_pages)
}

fn to_gateway_error_log_summary(item: GatewayErrorLog) -> GatewayErrorLogSummary {
    GatewayErrorLogSummary {
        trace_id: item.trace_id,
        key_id: item.key_id,
        account_id: item.account_id,
        request_path: item.request_path,
        method: item.method,
        stage: item.stage,
        error_kind: item.error_kind,
        upstream_url: item.upstream_url,
        cf_ray: item.cf_ray,
        status_code: item.status_code,
        compression_enabled: item.compression_enabled,
        compression_retry_attempted: item.compression_retry_attempted,
        message: item.message,
        created_at: item.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_page, normalize_page_size, normalize_stage_filter,
        DEFAULT_GATEWAY_ERROR_LOG_PAGE_SIZE, MAX_GATEWAY_ERROR_LOG_PAGE_SIZE,
    };

    #[test]
    fn gateway_error_log_params_normalize_blank_stage_to_none() {
        assert_eq!(normalize_stage_filter(Some("all".to_string())), None);
        assert_eq!(
            normalize_stage_filter(Some(
                "chatgpt_challenge_retry_without_compression".to_string()
            ))
            .as_deref(),
            Some("chatgpt_challenge_retry_without_compression")
        );
    }

    #[test]
    fn gateway_error_log_page_size_is_bounded() {
        assert_eq!(normalize_page_size(0), DEFAULT_GATEWAY_ERROR_LOG_PAGE_SIZE);
        assert_eq!(
            normalize_page_size(999),
            MAX_GATEWAY_ERROR_LOG_PAGE_SIZE
        );
    }

    #[test]
    fn gateway_error_log_page_is_clamped_to_total_pages() {
        assert_eq!(clamp_page(5, 0, 10), 1);
        assert_eq!(clamp_page(9, 31, 10), 4);
    }
}
