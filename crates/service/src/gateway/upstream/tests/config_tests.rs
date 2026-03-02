use reqwest::header::HeaderValue;

use super::{should_try_openai_fallback, should_try_openai_fallback_by_status};

#[test]
fn fallback_status_trigger_is_limited_to_responses_path() {
    assert!(should_try_openai_fallback_by_status(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses",
        429
    ));
    assert!(!should_try_openai_fallback_by_status(
        "https://chatgpt.com/backend-api/codex",
        "/v1/chat/completions",
        429
    ));
}

#[test]
fn fallback_content_type_trigger_is_limited_to_responses_path() {
    let html = HeaderValue::from_static("text/html; charset=utf-8");
    assert!(should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/responses",
        Some(&html)
    ));
    assert!(should_try_openai_fallback(
        "https://chatgpt.com/backend-api/codex",
        "/v1/chat/completions",
        Some(&html)
    ));
}
