use codexmanager_core::storage::{now_ts, RequestLog, RequestTokenStat, Storage};

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RequestLogUsage {
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub reasoning_output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RequestLogTraceContext<'a> {
    pub trace_id: Option<&'a str>,
    pub original_path: Option<&'a str>,
    pub adapted_path: Option<&'a str>,
    pub response_adapter: Option<super::ResponseAdapter>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RequestLogEntry<'a> {
    pub key_id: Option<&'a str>,
    pub account_id: Option<&'a str>,
    pub request_path: &'a str,
    pub method: &'a str,
    pub model: Option<&'a str>,
    pub reasoning_effort: Option<&'a str>,
    pub upstream_url: Option<&'a str>,
    pub status_code: Option<u16>,
    pub usage: RequestLogUsage,
    pub error: Option<&'a str>,
    pub duration_ms: Option<u128>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RequestLogRouteMeta<'a> {
    pub attempted_account_ids: Option<&'a [String]>,
    pub requested_model: Option<&'a str>,
    pub model_fallback_path: Option<&'a [String]>,
}

const MODEL_PRICE_PER_1K_TOKENS: &[(&str, f64, f64, f64)] = &[
    // OpenAI 官方价格（单位：USD / 1K tokens）。按模型前缀匹配，越具体越靠前。
    // gpt-5.3-codex 暂未公开价格，临时按 gpt-5.2-codex 计费。
    ("gpt-5.3-codex", 0.00175, 0.000175, 0.014),
    ("gpt-5.2-codex", 0.00175, 0.000175, 0.014),
    ("gpt-5.2", 0.00175, 0.000175, 0.014),
    ("gpt-5.1-codex-mini", 0.00025, 0.000025, 0.002),
    ("gpt-5.1-codex-max", 0.00125, 0.000125, 0.01),
    ("gpt-5.1-codex", 0.00125, 0.000125, 0.01),
    ("gpt-5.1", 0.00125, 0.000125, 0.01),
    ("gpt-5-codex", 0.00125, 0.000125, 0.01),
    ("gpt-5", 0.00125, 0.000125, 0.01),
    // 兼容旧模型：缓存输入按输入同价处理，保持历史口径稳定。
    ("gpt-4.1", 0.002, 0.002, 0.008),
    ("gpt-4o", 0.0025, 0.0025, 0.01),
    ("gpt-4", 0.03, 0.03, 0.06),
    ("claude-3-7", 0.003, 0.003, 0.015),
    ("claude-3-5", 0.003, 0.003, 0.015),
    ("claude-3", 0.003, 0.003, 0.015),
];

fn resolve_model_price_per_1k(
    normalized: &str,
    input_tokens_total: i64,
) -> Option<(f64, f64, f64)> {
    // OpenAI 官方定价：gpt-5.4 / gpt-5.4-pro 在输入超过 272K 时切换到更高档位。
    // gpt-5.4-pro 官方未提供 cached input 单价，这里按普通输入价计算，避免低估费用。
    if normalized.starts_with("gpt-5.4-pro") {
        if input_tokens_total > 272_000 {
            return Some((0.06, 0.06, 0.27));
        }
        return Some((0.03, 0.03, 0.18));
    }
    if normalized.starts_with("gpt-5.4") {
        if input_tokens_total > 272_000 {
            return Some((0.005, 0.0005, 0.0225));
        }
        return Some((0.0025, 0.00025, 0.015));
    }
    MODEL_PRICE_PER_1K_TOKENS
        .iter()
        .find(|(prefix, _, _, _)| normalized.starts_with(prefix))
        .map(|(_, input, cached_input, output)| (*input, *cached_input, *output))
}

fn estimate_cost_usd(
    model: Option<&str>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) -> f64 {
    let normalized = model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let Some(normalized) = normalized else {
        return 0.0;
    };
    let input_tokens_total = input_tokens.unwrap_or(0).max(0);
    let Some((in_per_1k, cached_in_per_1k, out_per_1k)) =
        resolve_model_price_per_1k(&normalized, input_tokens_total)
    else {
        return 0.0;
    };
    let in_tokens_total = input_tokens_total as f64;
    let cached_in_tokens = (cached_input_tokens.unwrap_or(0).max(0) as f64).min(in_tokens_total);
    let billable_in_tokens = (in_tokens_total - cached_in_tokens).max(0.0);
    let out_tokens = output_tokens.unwrap_or(0).max(0) as f64;
    (billable_in_tokens / 1000.0) * in_per_1k
        + (cached_in_tokens / 1000.0) * cached_in_per_1k
        + (out_tokens / 1000.0) * out_per_1k
}

fn normalize_token(value: Option<i64>) -> Option<i64> {
    value.map(|v| v.max(0))
}

fn normalize_duration_ms(value: Option<u128>) -> Option<i64> {
    value.map(|duration| duration.min(i64::MAX as u128) as i64)
}

fn is_inference_path(path: &str) -> bool {
    path.starts_with("/v1/responses")
        || path.starts_with("/v1/chat/completions")
        || path.starts_with("/v1/messages")
}

fn response_adapter_label(value: super::ResponseAdapter) -> &'static str {
    match value {
        super::ResponseAdapter::Passthrough => "Passthrough",
        super::ResponseAdapter::AnthropicJson => "AnthropicJson",
        super::ResponseAdapter::AnthropicSse => "AnthropicSse",
        super::ResponseAdapter::OpenAIChatCompletionsJson => "OpenAIChatCompletionsJson",
        super::ResponseAdapter::OpenAIChatCompletionsSse => "OpenAIChatCompletionsSse",
        super::ResponseAdapter::OpenAICompletionsJson => "OpenAICompletionsJson",
        super::ResponseAdapter::OpenAICompletionsSse => "OpenAICompletionsSse",
    }
}

pub(super) fn write_request_log(
    storage: &Storage,
    trace_context: RequestLogTraceContext<'_>,
    entry: RequestLogEntry<'_>,
) {
    write_request_log_with_attempts_and_model_fallback(
        storage,
        trace_context,
        entry,
        RequestLogRouteMeta::default(),
    );
}

pub(super) fn write_request_log_with_attempts_and_model_fallback(
    storage: &Storage,
    trace_context: RequestLogTraceContext<'_>,
    entry: RequestLogEntry<'_>,
    route_meta: RequestLogRouteMeta<'_>,
) {
    let original_path = trace_context.original_path.unwrap_or(entry.request_path);
    let adapted_path = trace_context.adapted_path.unwrap_or(entry.request_path);
    let initial_account_id = route_meta
        .attempted_account_ids
        .and_then(|items| items.first())
        .map(String::as_str);
    let attempted_account_ids_json = route_meta
        .attempted_account_ids
        .filter(|items| !items.is_empty())
        .and_then(|items| serde_json::to_string(items).ok());
    let model_fallback_path_json = route_meta
        .model_fallback_path
        .filter(|items| items.len() > 1)
        .and_then(|items| serde_json::to_string(items).ok());
    let input_tokens = normalize_token(entry.usage.input_tokens);
    let cached_input_tokens = normalize_token(entry.usage.cached_input_tokens);
    let output_tokens = normalize_token(entry.usage.output_tokens);
    let total_tokens = normalize_token(entry.usage.total_tokens);
    let reasoning_output_tokens = normalize_token(entry.usage.reasoning_output_tokens);
    let duration_ms = normalize_duration_ms(entry.duration_ms);
    let created_at = now_ts();
    super::metrics::record_gateway_latency_sample(created_at, duration_ms);
    let estimated_cost_usd = estimate_cost_usd(
        entry.model,
        input_tokens,
        cached_input_tokens,
        output_tokens,
    );
    super::trace_log::log_failed_request(
        created_at,
        trace_context.trace_id,
        entry.key_id,
        entry.account_id,
        entry.method,
        entry.request_path,
        Some(original_path),
        Some(adapted_path),
        entry.model,
        entry.reasoning_effort,
        entry.upstream_url,
        entry.status_code,
        entry.error,
        duration_ms,
    );
    let success = entry
        .status_code
        .map(|status| (200..300).contains(&status))
        .unwrap_or(false);
    let input_zero_or_missing = input_tokens.unwrap_or(0) == 0;
    let cached_zero_or_missing = cached_input_tokens.unwrap_or(0) == 0;
    let output_zero_or_missing = output_tokens.unwrap_or(0) == 0;
    let total_zero_or_missing = total_tokens.unwrap_or(0) == 0;
    let reasoning_zero_or_missing = reasoning_output_tokens.unwrap_or(0) == 0;
    if success
        && is_inference_path(entry.request_path)
        && input_zero_or_missing
        && cached_zero_or_missing
        && output_zero_or_missing
        && total_zero_or_missing
        && reasoning_zero_or_missing
    {
        log::warn!(
            "event=gateway_token_usage_missing path={} status={} account_id={} key_id={} model={}",
            entry.request_path,
            entry.status_code.unwrap_or(0),
            entry.account_id.unwrap_or("-"),
            entry.key_id.unwrap_or("-"),
            entry.model.unwrap_or("-"),
        );
    }
    // 记录请求最终结果（而非内部重试明细），保证 UI 一次请求只展示一条记录。
    let (request_log_id, token_stat_error) = match storage.insert_request_log_with_token_stat(
        &RequestLog {
            trace_id: trace_context.trace_id.map(|v| v.to_string()),
            key_id: entry.key_id.map(|v| v.to_string()),
            account_id: entry.account_id.map(|v| v.to_string()),
            initial_account_id: initial_account_id.map(str::to_string),
            attempted_account_ids_json,
            route_strategy: Some(super::current_route_strategy().to_string()),
            requested_model: route_meta.requested_model.map(|value| value.to_string()),
            model_fallback_path_json,
            request_path: entry.request_path.to_string(),
            original_path: Some(original_path.to_string()),
            adapted_path: Some(adapted_path.to_string()),
            method: entry.method.to_string(),
            model: entry.model.map(|v| v.to_string()),
            reasoning_effort: entry.reasoning_effort.map(|v| v.to_string()),
            response_adapter: trace_context
                .response_adapter
                .map(response_adapter_label)
                .map(str::to_string),
            upstream_url: entry.upstream_url.map(|v| v.to_string()),
            status_code: entry.status_code.map(i64::from),
            duration_ms,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            reasoning_output_tokens: None,
            estimated_cost_usd: None,
            error: entry.error.map(|v| v.to_string()),
            created_at,
        },
        &RequestTokenStat {
            request_log_id: 0,
            key_id: entry.key_id.map(|v| v.to_string()),
            account_id: entry.account_id.map(|v| v.to_string()),
            model: entry.model.map(|v| v.to_string()),
            input_tokens,
            cached_input_tokens,
            output_tokens,
            total_tokens,
            reasoning_output_tokens,
            estimated_cost_usd: Some(estimated_cost_usd),
            created_at,
        },
    ) {
        Ok(result) => result,
        Err(err) => {
            let err_text = err.to_string();
            super::metrics::record_db_error(err_text.as_str());
            log::error!(
                "event=gateway_request_log_insert_failed path={} status={} account_id={} key_id={} err={}",
                entry.request_path,
                entry.status_code.unwrap_or(0),
                entry.account_id.unwrap_or("-"),
                entry.key_id.unwrap_or("-"),
                err_text
            );
            return;
        }
    };

    if let Some(err) = token_stat_error {
        let err_text = err.to_string();
        super::metrics::record_db_error(err_text.as_str());
        log::error!(
            "event=gateway_request_token_stat_insert_failed path={} status={} account_id={} key_id={} request_log_id={} err={}",
            entry.request_path,
            entry.status_code.unwrap_or(0),
            entry.account_id.unwrap_or("-"),
            entry.key_id.unwrap_or("-"),
            request_log_id,
            err_text
        );
    }
}

#[cfg(test)]
#[path = "tests/request_log_tests.rs"]
mod tests;
