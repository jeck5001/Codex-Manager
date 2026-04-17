use bytes::Bytes;
use codexmanager_core::rpc::types::{
    RequestLogExportParams, RequestLogExportResult, RequestLogFilterParams, RequestLogSummary,
};
use std::convert::Infallible;

use crate::storage_helpers::open_storage;

use super::list::{normalize_filter_params, to_request_log_summary, to_storage_filters};

const REQUEST_LOG_EXPORT_CSV_HEADER: &str =
    "traceId,keyId,accountId,initialAccountId,attemptedAccountIds,candidateCount,attemptedCount,skippedCount,skippedCooldownCount,skippedInflightCount,routeStrategy,requestedModel,modelFallbackPath,requestPath,originalPath,adaptedPath,method,model,reasoningEffort,responseAdapter,upstreamUrl,statusCode,durationMs,inputTokens,cachedInputTokens,outputTokens,totalTokens,reasoningOutputTokens,estimatedCostUsd,error,createdAt";
const REQUEST_LOG_EXPORT_BATCH_SIZE: i64 = 500;

pub(crate) struct RequestLogExportPlan {
    pub(crate) format: &'static str,
    pub(crate) file_name: String,
    pub(crate) filters: RequestLogFilterParams,
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

fn append_csv_row(output: &mut String, item: &RequestLogSummary) {
    let columns = vec![
        optional_string(item.trace_id.as_deref()),
        optional_string(item.key_id.as_deref()),
        optional_string(item.account_id.as_deref()),
        optional_string(item.initial_account_id.as_deref()),
        json_string(&item.attempted_account_ids),
        optional_i64(item.candidate_count),
        optional_i64(item.attempted_count),
        optional_i64(item.skipped_count),
        optional_i64(item.skipped_cooldown_count),
        optional_i64(item.skipped_inflight_count),
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

fn build_request_log_export_csv(items: &[RequestLogSummary]) -> String {
    let mut lines = vec![REQUEST_LOG_EXPORT_CSV_HEADER.to_string()];

    for item in items {
        push_csv_row(
            &mut lines,
            &[
                optional_string(item.trace_id.as_deref()),
                optional_string(item.key_id.as_deref()),
                optional_string(item.account_id.as_deref()),
                optional_string(item.initial_account_id.as_deref()),
                json_string(&item.attempted_account_ids),
                optional_i64(item.candidate_count),
                optional_i64(item.attempted_count),
                optional_i64(item.skipped_count),
                optional_i64(item.skipped_cooldown_count),
                optional_i64(item.skipped_inflight_count),
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

pub(crate) fn prepare_request_log_export(
    params: RequestLogExportParams,
) -> Result<RequestLogExportPlan, String> {
    let filters = normalize_filter_params(params.filters);
    let format = normalize_export_format(params.format)?;
    let file_name = build_request_log_export_file_name(format, filters.status_filter.as_deref());
    Ok(RequestLogExportPlan {
        format,
        file_name,
        filters,
    })
}

pub(crate) fn stream_request_log_export_chunks(
    plan: RequestLogExportPlan,
    sender: tokio::sync::mpsc::Sender<Result<Bytes, Infallible>>,
) -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let mut offset = 0_i64;
    let mut streamed_any = false;

    if plan.format == "csv" {
        sender
            .blocking_send(Ok(Bytes::from(format!(
                "{REQUEST_LOG_EXPORT_CSV_HEADER}\n"
            ))))
            .map_err(|_| "request log export stream closed".to_string())?;
    }

    loop {
        let rows = storage
            .list_request_logs_paginated_filtered(
                to_storage_filters(&plan.filters, None, None),
                offset,
                REQUEST_LOG_EXPORT_BATCH_SIZE,
            )
            .map_err(|err| format!("list request logs failed: {err}"))?;
        if rows.is_empty() {
            break;
        }

        let items = rows
            .into_iter()
            .map(to_request_log_summary)
            .collect::<Vec<_>>();
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
                            .map_err(|err| format!("serialize request logs failed: {err}"))?,
                    );
                }
            }
            _ => unreachable!(),
        }

        if !chunk.is_empty() {
            sender
                .blocking_send(Ok(Bytes::from(chunk)))
                .map_err(|_| "request log export stream closed".to_string())?;
        }
        offset += items.len() as i64;
    }

    if plan.format == "json" {
        let tail = if streamed_any { "\n]" } else { "[]" };
        sender
            .blocking_send(Ok(Bytes::from(tail.to_string())))
            .map_err(|_| "request log export stream closed".to_string())?;
    }

    Ok(())
}

pub(crate) fn export_request_logs(
    params: RequestLogExportParams,
) -> Result<RequestLogExportResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let plan = prepare_request_log_export(params)?;
    let total = storage
        .count_request_logs_filtered(to_storage_filters(&plan.filters, None, None))
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let items = if total > 0 {
        storage
            .list_request_logs_paginated_filtered(
                to_storage_filters(&plan.filters, None, None),
                0,
                total,
            )
            .map_err(|err| format!("list request logs failed: {err}"))?
            .into_iter()
            .map(to_request_log_summary)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content = match plan.format {
        "csv" => build_request_log_export_csv(&items),
        "json" => serde_json::to_string_pretty(&items)
            .map_err(|err| format!("serialize request logs failed: {err}"))?,
        _ => unreachable!(),
    };

    Ok(RequestLogExportResult {
        format: plan.format.to_string(),
        file_name: plan.file_name,
        content,
        record_count: items.len() as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_request_log_export_csv, prepare_request_log_export, stream_request_log_export_chunks,
        RequestLogExportPlan,
    };
    use codexmanager_core::rpc::types::{
        RequestLogExportParams, RequestLogFilterParams, RequestLogSummary,
    };
    use codexmanager_core::storage::{now_ts, RequestLog, Storage};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::MutexGuard;

    static TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    struct TestDbScope {
        _env_lock: MutexGuard<'static, ()>,
        _db_guard: EnvGuard,
        db_path: PathBuf,
    }

    impl Drop for TestDbScope {
        fn drop(&mut self) {
            crate::storage_helpers::clear_storage_cache_for_tests();
            remove_sqlite_test_artifacts(&self.db_path);
        }
    }

    fn new_test_db_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{prefix}-{}-{}-{}.db",
            std::process::id(),
            now_ts(),
            TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        path
    }

    fn setup_test_db(prefix: &str) -> (TestDbScope, Storage) {
        let env_lock = crate::lock_utils::process_env_test_guard();
        crate::storage_helpers::clear_storage_cache_for_tests();
        let db_path = new_test_db_path(prefix);
        let db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
        let storage = Storage::open(&db_path).expect("open db");
        storage.init().expect("init schema");
        (
            TestDbScope {
                _env_lock: env_lock,
                _db_guard: db_guard,
                db_path,
            },
            storage,
        )
    }

    fn remove_sqlite_test_artifacts(db_path: &PathBuf) {
        let _ = fs::remove_file(db_path);
        let shm_path = PathBuf::from(format!("{}-shm", db_path.display()));
        let wal_path = PathBuf::from(format!("{}-wal", db_path.display()));
        let _ = fs::remove_file(shm_path);
        let _ = fs::remove_file(wal_path);
    }

    fn sample_request_log(index: i64) -> RequestLog {
        RequestLog {
            trace_id: Some(format!("trc-stream-{index}")),
            key_id: Some("gk-stream".to_string()),
            account_id: Some("acc-stream".to_string()),
            initial_account_id: Some("acc-stream".to_string()),
            attempted_account_ids_json: Some(r#"["acc-stream"]"#.to_string()),
            candidate_count: Some(6),
            attempted_count: Some(1),
            skipped_count: Some(5),
            skipped_cooldown_count: Some(4),
            skipped_inflight_count: Some(1),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(100 + index),
            input_tokens: Some(20),
            cached_input_tokens: Some(0),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000 + index,
        }
    }

    #[test]
    fn request_log_export_csv_contains_header_and_items() {
        let csv = build_request_log_export_csv(&[RequestLogSummary {
            trace_id: Some("trc-export".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-initial".to_string()),
            attempted_account_ids: vec!["acc-initial".to_string(), "acc-export".to_string()],
            candidate_count: Some(12),
            attempted_count: Some(2),
            skipped_count: Some(10),
            skipped_cooldown_count: Some(9),
            skipped_inflight_count: Some(1),
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
        assert!(csv.contains("candidateCount,attemptedCount,skippedCount"));
        assert!(csv.contains("trc-export"));
        assert!(csv.contains("\"[\"\"acc-initial\"\",\"\"acc-export\"\"]\""));
    }

    #[test]
    fn prepare_request_log_export_uses_json_extension() {
        let plan = prepare_request_log_export(RequestLogExportParams {
            format: Some("json".to_string()),
            filters: RequestLogFilterParams::default(),
        })
        .expect("prepare json export");

        assert_eq!(plan.format, "json");
        assert_eq!(plan.file_name, "codexmanager-requestlogs-all.json");
    }

    #[test]
    fn request_log_export_streams_json_in_multiple_chunks() {
        let (_db_scope, storage) = setup_test_db("requestlog-export-stream-json");
        for index in 0..501 {
            storage
                .insert_request_log(&sample_request_log(index))
                .expect("insert request log");
        }

        let plan = RequestLogExportPlan {
            format: "json",
            file_name: "codexmanager-requestlogs-all.json".to_string(),
            filters: RequestLogFilterParams::default(),
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        stream_request_log_export_chunks(plan, tx).expect("stream request logs");

        let mut chunks = Vec::new();
        while let Some(chunk) = rx.blocking_recv() {
            chunks.push(chunk.expect("chunk"));
        }

        assert!(
            chunks.len() >= 3,
            "expected opening batch, follow-up batch, and closing chunk"
        );

        let payload = chunks
            .into_iter()
            .map(|chunk| String::from_utf8(chunk.to_vec()).expect("utf8 chunk"))
            .collect::<String>();
        let items: Vec<RequestLogSummary> =
            serde_json::from_str(&payload).expect("parse streamed json");
        assert_eq!(items.len(), 501);
        assert_eq!(
            items.first().and_then(|item| item.trace_id.as_deref()),
            Some("trc-stream-500")
        );
        assert_eq!(
            items.last().and_then(|item| item.trace_id.as_deref()),
            Some("trc-stream-0")
        );
    }
}
