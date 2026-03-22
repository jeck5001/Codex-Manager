use codexmanager_core::rpc::types::{
    RequestLogExportParams, RequestLogExportResult, RequestLogSummary,
};

use crate::storage_helpers::open_storage;

use super::list::{normalize_optional_text, normalize_status_filter, to_request_log_summary};

fn normalize_export_format(value: Option<String>) -> Result<&'static str, String> {
    match value
        .unwrap_or_else(|| "csv".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "csv" => Ok("csv"),
        "json" => Ok("json"),
        other => Err(format!("unsupported request log export format: {other}")),
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn push_csv_row(lines: &mut Vec<String>, columns: &[String]) {
    lines.push(
        columns
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
}

fn optional_string(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn optional_i64(value: Option<i64>) -> String {
    value.map(|item| item.to_string()).unwrap_or_default()
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(|item| format!("{item:.6}")).unwrap_or_default()
}

fn json_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

fn build_request_log_export_csv(items: &[RequestLogSummary]) -> String {
    let mut lines = vec![
        "traceId,keyId,accountId,initialAccountId,attemptedAccountIds,routeStrategy,requestedModel,modelFallbackPath,requestPath,originalPath,adaptedPath,method,model,reasoningEffort,responseAdapter,upstreamUrl,statusCode,durationMs,inputTokens,cachedInputTokens,outputTokens,totalTokens,reasoningOutputTokens,estimatedCostUsd,error,createdAt".to_string(),
    ];

    for item in items {
        push_csv_row(
            &mut lines,
            &[
                optional_string(item.trace_id.as_deref()),
                optional_string(item.key_id.as_deref()),
                optional_string(item.account_id.as_deref()),
                optional_string(item.initial_account_id.as_deref()),
                json_string(&item.attempted_account_ids),
                optional_string(item.route_strategy.as_deref()),
                optional_string(item.requested_model.as_deref()),
                json_string(&item.model_fallback_path),
                item.request_path.clone(),
                optional_string(item.original_path.as_deref()),
                optional_string(item.adapted_path.as_deref()),
                item.method.clone(),
                optional_string(item.model.as_deref()),
                optional_string(item.reasoning_effort.as_deref()),
                optional_string(item.response_adapter.as_deref()),
                optional_string(item.upstream_url.as_deref()),
                optional_i64(item.status_code),
                optional_i64(item.duration_ms),
                optional_i64(item.input_tokens),
                optional_i64(item.cached_input_tokens),
                optional_i64(item.output_tokens),
                optional_i64(item.total_tokens),
                optional_i64(item.reasoning_output_tokens),
                optional_f64(item.estimated_cost_usd),
                optional_string(item.error.as_deref()),
                item.created_at.to_string(),
            ],
        );
    }

    lines.join("\n") + "\n"
}

fn build_request_log_export_file_name(format: &str, status_filter: Option<&str>) -> String {
    let scope = status_filter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("all");
    format!("codexmanager-requestlogs-{scope}.{format}")
}

pub(crate) fn export_request_logs(
    params: RequestLogExportParams,
) -> Result<RequestLogExportResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(params.query);
    let status_filter = normalize_status_filter(params.status_filter);
    let format = normalize_export_format(params.format)?;
    let total = storage
        .count_request_logs(query.as_deref(), status_filter.as_deref())
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let items = if total > 0 {
        storage
            .list_request_logs_paginated(query.as_deref(), status_filter.as_deref(), 0, total)
            .map_err(|err| format!("list request logs failed: {err}"))?
            .into_iter()
            .map(to_request_log_summary)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content = match format {
        "csv" => build_request_log_export_csv(&items),
        "json" => serde_json::to_string_pretty(&items)
            .map_err(|err| format!("serialize request logs failed: {err}"))?,
        _ => unreachable!(),
    };

    Ok(RequestLogExportResult {
        format: format.to_string(),
        file_name: build_request_log_export_file_name(format, status_filter.as_deref()),
        content,
        record_count: items.len() as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::build_request_log_export_csv;
    use codexmanager_core::rpc::types::RequestLogSummary;

    #[test]
    fn request_log_export_csv_contains_header_and_items() {
        let csv = build_request_log_export_csv(&[RequestLogSummary {
            trace_id: Some("trc-export".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-initial".to_string()),
            attempted_account_ids: vec!["acc-initial".to_string(), "acc-export".to_string()],
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path: vec!["o3".to_string(), "gpt-4o".to_string()],
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-4o".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(123),
            input_tokens: Some(10),
            cached_input_tokens: Some(1),
            output_tokens: Some(3),
            total_tokens: Some(13),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        }]);

        assert!(csv.contains("traceId,keyId,accountId"));
        assert!(csv.contains("trc-export"));
        assert!(csv.contains("[\"acc-initial\",\"acc-export\"]"));
    }
}
