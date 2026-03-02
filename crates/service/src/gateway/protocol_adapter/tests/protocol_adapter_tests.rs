use super::{adapt_request_for_protocol, ResponseAdapter};
use crate::apikey_profile::{PROTOCOL_ANTHROPIC_NATIVE, PROTOCOL_OPENAI_COMPAT};

#[test]
fn openai_chat_completions_passthrough_without_responses_adaptation() {
    let body = br#"{"model":"gpt-5.3-codex","messages":[{"role":"user","content":"hi"}]}"#.to_vec();
    let adapted =
        adapt_request_for_protocol(PROTOCOL_OPENAI_COMPAT, "/v1/chat/completions", body.clone())
            .expect("adapt request");
    assert_eq!(adapted.path, "/v1/chat/completions");
    assert_eq!(adapted.body, body);
    assert_eq!(adapted.response_adapter, ResponseAdapter::Passthrough);
}

#[test]
fn openai_responses_passthrough_keeps_responses_path() {
    let body = br#"{"model":"gpt-5.3-codex","input":"hi"}"#.to_vec();
    let adapted = adapt_request_for_protocol(PROTOCOL_OPENAI_COMPAT, "/v1/responses", body.clone())
        .expect("adapt request");
    assert_eq!(adapted.path, "/v1/responses");
    assert_eq!(adapted.body, body);
    assert_eq!(adapted.response_adapter, ResponseAdapter::Passthrough);
}

#[test]
fn anthropic_messages_are_the_only_path_adapted_to_responses() {
    let body =
        br#"{"model":"claude-3-5-sonnet","messages":[{"role":"user","content":"hello"}]}"#.to_vec();
    let adapted = adapt_request_for_protocol(PROTOCOL_ANTHROPIC_NATIVE, "/v1/messages", body)
        .expect("adapt request");
    assert_eq!(adapted.path, "/v1/responses");
    assert_ne!(adapted.response_adapter, ResponseAdapter::Passthrough);
}

#[test]
fn anthropic_chat_completions_still_passthrough() {
    let body =
        br#"{"model":"gpt-5.3-codex","messages":[{"role":"user","content":"hello"}]}"#.to_vec();
    let adapted = adapt_request_for_protocol(
        PROTOCOL_ANTHROPIC_NATIVE,
        "/v1/chat/completions",
        body.clone(),
    )
    .expect("adapt request");
    assert_eq!(adapted.path, "/v1/chat/completions");
    assert_eq!(adapted.body, body);
    assert_eq!(adapted.response_adapter, ResponseAdapter::Passthrough);
}
