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
    response_adapter: super::ResponseAdapter,
) -> Result<(), String> {
    match response_adapter {
        super::ResponseAdapter::Passthrough => {
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
        super::ResponseAdapter::AnthropicJson | super::ResponseAdapter::AnthropicSse => {
            let status = StatusCode(upstream.status().as_u16());
            let mut headers = Vec::new();
            for (name, value) in upstream.headers().iter() {
                let name_str = name.as_str();
                if name_str.eq_ignore_ascii_case("transfer-encoding")
                    || name_str.eq_ignore_ascii_case("content-length")
                    || name_str.eq_ignore_ascii_case("connection")
                    || name_str.eq_ignore_ascii_case("content-type")
                {
                    continue;
                }
                if let Ok(header) = Header::from_bytes(name_str.as_bytes(), value.as_bytes()) {
                    headers.push(header);
                }
            }
            let upstream_content_type = upstream
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());
            let upstream_body = upstream
                .bytes()
                .map(|v| v.to_vec())
                .map_err(|err| format!("read upstream body failed: {err}"))?;

            let (body, content_type) = match super::protocol_adapter::adapt_upstream_response(
                response_adapter,
                upstream_content_type.as_deref(),
                &upstream_body,
            ) {
                Ok(result) => result,
                Err(err) => (
                    super::protocol_adapter::build_anthropic_error_body(&format!(
                        "response conversion failed: {err}"
                    )),
                    "application/json",
                ),
            };
            if let Ok(content_type_header) =
                Header::from_bytes(b"Content-Type".as_slice(), content_type.as_bytes())
            {
                headers.push(content_type_header);
            }

            let len = Some(body.len());
            let response = Response::new(status, headers, std::io::Cursor::new(body), len, None);
            let _ = request.respond(response);
            Ok(())
        }
    }
}
