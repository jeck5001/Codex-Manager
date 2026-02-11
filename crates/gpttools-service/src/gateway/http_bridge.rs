use tiny_http::{Header, Request, Response, StatusCode};

use super::AccountInFlightGuard;

pub(super) fn extract_platform_key(request: &Request) -> Option<String> {
    // 从请求头提取平台 Key
    for header in request.headers() {
        if header.field.equiv("Authorization") {
            let value = header.value.as_str();
            if let Some(rest) = value.strip_prefix("Bearer ") {
                return Some(rest.trim().to_string());
            }
        }
        if header.field.equiv("x-api-key") {
            return Some(header.value.as_str().trim().to_string());
        }
    }
    None
}

pub(super) fn respond_with_upstream(
    request: Request,
    upstream: reqwest::blocking::Response,
    _inflight_guard: AccountInFlightGuard,
) -> Result<(), String> {
    let status = StatusCode(upstream.status().as_u16());
    let mut headers = Vec::new();
    for (name, value) in upstream.headers().iter() {
        let name_str = name.as_str();
        if name_str.eq_ignore_ascii_case("transfer-encoding")
            || name_str.eq_ignore_ascii_case("content-length")
            || name_str.eq_ignore_ascii_case("connection")
        {
            continue;
        }
        if let Ok(header) = Header::from_bytes(name_str.as_bytes(), value.as_bytes()) {
            headers.push(header);
        }
    }
    let len = upstream.content_length().map(|v| v as usize);
    let response = Response::new(status, headers, upstream, len, None);
    let _ = request.respond(response);
    Ok(())
}
