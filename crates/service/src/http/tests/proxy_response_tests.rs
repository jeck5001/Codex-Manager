use super::{merge_upstream_headers, text_error_response, text_response};
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;

/// 函数 `text_response_sets_status_and_plain_text_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn text_response_sets_status_and_plain_text_header() {
    let response = text_response(StatusCode::BAD_GATEWAY, "proxy failed");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
}

/// 函数 `text_error_response_sets_error_code_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn text_error_response_sets_error_code_header() {
    let response = text_error_response(StatusCode::BAD_GATEWAY, "backend proxy error: refused");
    assert_eq!(
        response
            .headers()
            .get(crate::error_codes::ERROR_CODE_HEADER_NAME)
            .and_then(|value| value.to_str().ok()),
        Some("backend_proxy_error")
    );
}

/// 函数 `merge_upstream_headers_filters_hop_by_hop_and_content_length`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn merge_upstream_headers_filters_hop_by_hop_and_content_length() {
    let mut upstream_headers = reqwest::header::HeaderMap::new();
    upstream_headers.insert(
        "content-type",
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    upstream_headers.insert(
        "content-length",
        reqwest::header::HeaderValue::from_static("64"),
    );
    upstream_headers.insert(
        "connection",
        reqwest::header::HeaderValue::from_static("close"),
    );

    let response = merge_upstream_headers(
        axum::http::Response::builder().status(StatusCode::OK),
        &upstream_headers,
    )
    .body(Body::empty())
    .expect("response should build");

    assert!(response.headers().contains_key("content-type"));
    assert!(!response.headers().contains_key("content-length"));
    assert!(!response.headers().contains_key("connection"));
}
