use gpttools_core::storage::{now_ts, RequestLog, Storage};

pub(super) fn write_request_log(
    storage: &Storage,
    key_id: Option<&str>,
    request_path: &str,
    method: &str,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    upstream_url: Option<&str>,
    status_code: Option<u16>,
    error: Option<&str>,
) {
    // 记录每次网关转发结果，便于在 UI 里按模型/错误检索问题。
    let _ = storage.insert_request_log(&RequestLog {
        key_id: key_id.map(|v| v.to_string()),
        request_path: request_path.to_string(),
        method: method.to_string(),
        model: model.map(|v| v.to_string()),
        reasoning_effort: reasoning_effort.map(|v| v.to_string()),
        upstream_url: upstream_url.map(|v| v.to_string()),
        status_code: status_code.map(|v| i64::from(v)),
        error: error.map(|v| v.to_string()),
        created_at: now_ts(),
    });
}
