use tiny_http::{Header, Request};

mod aggregate;
mod openai;
use aggregate::{
    append_output_text, collect_non_stream_json_from_sse_bytes,
    collect_output_text_from_event_fields, collect_response_output_text,
    extract_error_hint_from_body, extract_error_message_from_json, extract_sse_frame_payload,
    inspect_sse_frame, is_response_completed_event_name, looks_like_sse_payload, merge_usage,
    parse_sse_frame_json, parse_usage_from_json, reload_output_text_from_env, usage_has_signal,
    SseTerminal, UpstreamResponseBridgeResult, UpstreamResponseUsage,
};
#[cfg(test)]
use aggregate::{
    output_text_limit_bytes, parse_usage_from_sse_frame, OUTPUT_TEXT_TRUNCATED_MARKER,
};
use openai::{
    apply_openai_stream_meta_defaults, build_chat_fallback_content_chunk,
    build_completion_fallback_text_chunk, extract_openai_completed_output_text,
    map_chunk_has_chat_text, map_chunk_has_completion_text, normalize_chat_chunk_delta_role,
    should_skip_chat_live_text_event, should_skip_completion_live_text_event,
    synthesize_chat_completion_sse_from_json, synthesize_completions_sse_from_json,
    update_openai_stream_meta, OpenAIStreamMeta,
};

pub(crate) struct RespondWithUpstreamArgs<'a> {
    pub(crate) request: Request,
    pub(crate) upstream: reqwest::blocking::Response,
    pub(crate) inflight_guard: super::AccountInFlightGuard,
    pub(crate) response_adapter: super::ResponseAdapter,
    pub(crate) request_path: &'a str,
    pub(crate) tool_name_restore_map: Option<&'a super::ToolNameRestoreMap>,
    pub(crate) is_stream: bool,
    pub(crate) trace_id: Option<&'a str>,
    pub(crate) actual_model_header: Option<&'a str>,
    pub(crate) response_cache_key: Option<&'a str>,
}

pub(super) fn reload_from_env() {
    reload_output_text_from_env();
    stream_readers::reload_from_env();
}

pub(super) fn current_sse_keepalive_interval_ms() -> u64 {
    stream_readers::current_sse_keepalive_interval_ms()
}

pub(super) fn set_sse_keepalive_interval_ms(interval_ms: u64) -> Result<u64, String> {
    stream_readers::set_sse_keepalive_interval_ms(interval_ms)
}

pub(crate) fn summarize_upstream_error_hint_from_body(
    status_code: u16,
    body: &[u8],
) -> Option<String> {
    aggregate::extract_error_hint_from_body(status_code, body)
}

fn push_trace_id_header(headers: &mut Vec<Header>, trace_id: &str) {
    push_optional_static_header(
        headers,
        crate::error_codes::TRACE_ID_HEADER_NAME,
        Some(trace_id),
    );
}

pub(super) fn push_optional_static_header(
    headers: &mut Vec<Header>,
    name: &str,
    value: Option<&str>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
        headers.push(header);
    }
}

mod delivery;
mod stream_readers;
pub(super) fn respond_with_upstream(
    args: RespondWithUpstreamArgs<'_>,
) -> Result<UpstreamResponseBridgeResult, String> {
    delivery::respond_with_upstream(args)
}
pub(super) use stream_readers::{
    AnthropicSseReader, OpenAIChatCompletionsSseReader, OpenAICompletionsSseReader,
    PassthroughSseCollector, PassthroughSseUsageReader, SseKeepAliveFrame,
};

#[cfg(test)]
#[path = "../tests/http_bridge_tests.rs"]
mod tests;
