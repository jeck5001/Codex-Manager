use reqwest::header::HeaderValue;
use serde_json::Value;

pub(crate) fn extract_request_model(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let value = serde_json::from_slice::<Value>(body).ok()?;
    value
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

pub(crate) fn extract_request_reasoning_effort(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let value = serde_json::from_slice::<Value>(body).ok()?;
    // 兼容 responses 风格：{ "reasoning": { "effort": "medium" } }
    value
        .get("reasoning")
        .and_then(|v| v.get("effort"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        // 兼容潜在直传字段：{ "reasoning_effort": "medium" }
        .or_else(|| {
            value
                .get("reasoning_effort")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
        })
}

pub(crate) fn extract_request_stream(body: &[u8]) -> Option<bool> {
    if body.is_empty() {
        return None;
    }
    let value = serde_json::from_slice::<Value>(body).ok()?;
    value.get("stream").and_then(Value::as_bool)
}

#[cfg(test)]
pub(crate) fn should_drop_incoming_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.eq_ignore_ascii_case("Authorization")
        || name.eq_ignore_ascii_case("x-api-key")
        || name.eq_ignore_ascii_case("Host")
        || name.eq_ignore_ascii_case("Content-Length")
        // 中文注释：Claude SDK/CLI 会附带 anthropic/x-stainless 指纹头；
        // 直接透传到 ChatGPT upstream 会提高 challenge 概率，这里统一剔除。
        || lower.starts_with("anthropic-")
        || lower.starts_with("x-stainless-")
        // 中文注释：resume 会携带旧会话的账号头；若不剔除会把请求强行绑定到过期/耗尽账号，导致无法切换候选账号。
        || name.eq_ignore_ascii_case("ChatGPT-Account-Id")
}

#[cfg(test)]
pub(crate) fn should_drop_session_affinity_header(name: &str) -> bool {
    // 中文注释：session_id / turn-state 属于会话粘性信号，正常直连时应保留；
    // 仅在 failover 到其他账号时剔除，避免继续命中旧账号会话路由导致“切换无效”。
    name.eq_ignore_ascii_case("session_id") || name.eq_ignore_ascii_case("x-codex-turn-state")
}

#[cfg(test)]
pub(crate) fn should_drop_incoming_header_for_failover(name: &str) -> bool {
    should_drop_incoming_header(name) || should_drop_session_affinity_header(name)
}

pub(crate) fn is_upstream_challenge_response(
    status_code: u16,
    content_type: Option<&HeaderValue>,
) -> bool {
    let is_html = content_type
        .and_then(|v| v.to_str().ok())
        .map(is_html_content_type)
        .unwrap_or(false);
    // 中文注释：403 并不总是 Cloudflare challenge（也可能是上游业务鉴权错误），
    // 仅在明确 HTML challenge 或 429 限流时按 challenge 处理，避免误导排障方向。
    is_html || status_code == 429
}

pub(crate) fn is_html_content_type(value: &str) -> bool {
    value.trim().to_ascii_lowercase().starts_with("text/html")
}

pub(crate) fn normalize_models_path(path: &str) -> String {
    let is_models_path = path == "/v1/models" || path.starts_with("/v1/models?");
    if !is_models_path {
        return path.to_string();
    }
    let has_client_version = path
        .split_once('?')
        .map(|(_, query)| {
            query.split('&').any(|part| {
                part.split('=')
                    .next()
                    .is_some_and(|key| key.eq_ignore_ascii_case("client_version"))
            })
        })
        .unwrap_or(false);
    if has_client_version {
        return path.to_string();
    }
    let client_version = super::DEFAULT_MODELS_CLIENT_VERSION.to_string();
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}client_version={client_version}")
}
