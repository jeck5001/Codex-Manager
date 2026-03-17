use super::{should_failover_after_fallback_non_success, summarize_fallback_non_success};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;

#[test]
fn fallback_non_success_5xx_does_not_failover_even_with_more_candidates() {
    assert!(!should_failover_after_fallback_non_success(500, true));
    assert!(!should_failover_after_fallback_non_success(503, true));
}

#[test]
fn fallback_non_success_auth_and_rate_limit_can_failover_when_candidates_remain() {
    assert!(should_failover_after_fallback_non_success(401, true));
    assert!(should_failover_after_fallback_non_success(403, true));
    assert!(should_failover_after_fallback_non_success(404, true));
    assert!(should_failover_after_fallback_non_success(429, true));
}

#[test]
fn fallback_non_success_never_failover_without_more_candidates() {
    assert!(!should_failover_after_fallback_non_success(401, false));
    assert!(!should_failover_after_fallback_non_success(429, false));
    assert!(!should_failover_after_fallback_non_success(500, false));
}

#[test]
fn fallback_non_success_summary_includes_debug_headers_and_body_hint() {
    let mut headers = HeaderMap::new();
    headers.insert("x-oai-request-id", HeaderValue::from_static("req_fallback"));
    headers.insert("cf-ray", HeaderValue::from_static("ray_fallback"));
    headers.insert(
        "x-openai-authorization-error",
        HeaderValue::from_static("expired_session"),
    );
    headers.insert(
        "x-error-json",
        HeaderValue::from_static("{\"identity_error_code\":\"org_membership_required\"}"),
    );

    let message = summarize_fallback_non_success(
        403,
        403,
        &headers,
        b"<html><title>Just a moment...</title><body>Cloudflare</body></html>",
    );

    assert!(
        message.contains("status=403"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("Cloudflare 安全验证页"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("primary_status=403"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("request_id=req_fallback"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("cf_ray=ray_fallback"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("auth_error=expired_session"),
        "unexpected summary: {message}"
    );
    assert!(
        message.contains("identity_error_code=org_membership_required"),
        "unexpected summary: {message}"
    );
}

#[test]
fn fallback_non_success_summary_uses_plain_body_when_no_structured_hint_exists() {
    let headers = HeaderMap::new();
    let message = summarize_fallback_non_success(404, 404, &headers, b"plain upstream error");

    assert!(
        message.contains("body=plain upstream error"),
        "unexpected summary: {message}"
    );
}
