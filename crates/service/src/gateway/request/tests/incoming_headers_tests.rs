use super::*;

#[test]
fn strict_bearer_parsing_matches_auth_extraction_behavior() {
    assert_eq!(strict_bearer_token("Bearer abc"), Some("abc".to_string()));
    assert_eq!(strict_bearer_token("bearer abc"), None);
    assert_eq!(strict_bearer_token("Bearer   "), None);
}

#[test]
fn case_insensitive_bearer_parsing_matches_sticky_derivation_behavior() {
    assert_eq!(
        case_insensitive_bearer_token("Bearer abc"),
        Some("abc".to_string())
    );
    assert_eq!(
        case_insensitive_bearer_token("bearer abc"),
        Some("abc".to_string())
    );
    assert_eq!(case_insensitive_bearer_token("basic abc"), None);
    assert_eq!(case_insensitive_bearer_token("bearer   "), None);
}

#[test]
fn goog_api_key_header_is_accepted_as_platform_key() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "x-goog-api-key",
        axum::http::HeaderValue::from_static("platform-key-from-gemini"),
    );

    let snapshot = IncomingHeaderSnapshot::from_http_headers(&headers);
    assert_eq!(snapshot.platform_key(), Some("platform-key-from-gemini"));
    assert!(snapshot.has_x_api_key());
    assert_eq!(
        snapshot.sticky_key_material(),
        Some("platform-key-from-gemini")
    );
}

#[test]
fn codex_headers_are_captured_from_http_headers() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "User-Agent",
        axum::http::HeaderValue::from_static("codex_cli_rs/0.999.0"),
    );
    headers.insert(
        "originator",
        axum::http::HeaderValue::from_static("codex_cli_rs"),
    );
    headers.insert(
        "x-session-affinity",
        axum::http::HeaderValue::from_static("affinity_123"),
    );
    headers.insert(
        "x-codex-parent-thread-id",
        axum::http::HeaderValue::from_static("thread_parent_123"),
    );
    headers.insert(
        "x-codex-window-id",
        axum::http::HeaderValue::from_static("thread_child_123:7"),
    );
    headers.insert(
        "x-codex-other-limit-name",
        axum::http::HeaderValue::from_static("promo_header"),
    );

    let snapshot = IncomingHeaderSnapshot::from_http_headers(&headers);
    assert_eq!(snapshot.user_agent(), Some("codex_cli_rs/0.999.0"));
    assert_eq!(snapshot.originator(), Some("codex_cli_rs"));
    assert_eq!(snapshot.session_affinity(), Some("affinity_123"));
    assert_eq!(snapshot.parent_thread_id(), Some("thread_parent_123"));
    assert_eq!(snapshot.window_id(), Some("thread_child_123:7"));
    assert!(snapshot.passthrough_codex_headers().is_empty());
}
