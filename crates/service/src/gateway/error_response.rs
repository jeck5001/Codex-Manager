use tiny_http::{Header, Response};

pub(super) fn with_trace_id_header<R: std::io::Read>(
    mut response: Response<R>,
    trace_id: Option<&str>,
) -> Response<R> {
    if let Some(trace_id) = trace_id.map(str::trim).filter(|value| !value.is_empty()) {
        if let Ok(header) = Header::from_bytes(
            crate::error_codes::TRACE_ID_HEADER_NAME.as_bytes(),
            trace_id.as_bytes(),
        ) {
            response.add_header(header);
        }
    }
    response
}

pub(super) fn terminal_text_response(
    status_code: u16,
    message: impl Into<String>,
    trace_id: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let message = message.into();
    let mut response = Response::from_string(message.clone()).with_status_code(status_code);
    if let Ok(header) = Header::from_bytes(
        crate::error_codes::ERROR_CODE_HEADER_NAME.as_bytes(),
        crate::error_codes::code_for_message(message.as_str()).as_bytes(),
    ) {
        response.add_header(header);
    }
    with_trace_id_header(response, trace_id)
}

#[cfg(test)]
#[path = "tests/error_response_tests.rs"]
mod tests;
