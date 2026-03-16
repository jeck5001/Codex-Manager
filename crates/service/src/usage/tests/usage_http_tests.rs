use super::usage_http_client;
use reqwest::StatusCode;

#[test]
fn usage_http_client_is_cloneable() {
    let first = usage_http_client();
    let second = usage_http_client();
    let first_ptr = &first as *const reqwest::blocking::Client;
    let second_ptr = &second as *const reqwest::blocking::Client;
    assert_ne!(first_ptr, second_ptr);
}

#[test]
fn refresh_token_status_error_omits_empty_body() {
    assert_eq!(
        super::format_refresh_token_status_error(StatusCode::FORBIDDEN, "   "),
        "refresh token failed with status 403 Forbidden"
    );
}

#[test]
fn refresh_token_status_error_includes_body_snippet() {
    assert_eq!(
        super::format_refresh_token_status_error(
            StatusCode::BAD_REQUEST,
            "{\n  \"error\": \"invalid_grant\"\n}"
        ),
        "refresh token failed with status 400 Bad Request: { \"error\": \"invalid_grant\" }"
    );
}
