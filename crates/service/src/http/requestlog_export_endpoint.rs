use axum::extract::Query;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response as AxumResponse};
use codexmanager_core::rpc::types::{RequestLogExportParams, RequestLogExportResult};

fn validate_export_headers(headers: &HeaderMap) -> Option<AxumResponse> {
    match headers
        .get("X-CodexManager-Rpc-Token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(token) => {
            if !crate::rpc_auth_token_matches(token) {
                return Some((StatusCode::UNAUTHORIZED, "unauthorized").into_response());
            }
        }
        None => return Some((StatusCode::UNAUTHORIZED, "unauthorized").into_response()),
    }

    None
}

fn build_export_response(result: RequestLogExportResult) -> AxumResponse {
    let content_type = match result.format.trim().to_ascii_lowercase().as_str() {
        "json" => "application/json; charset=utf-8",
        _ => "text/csv; charset=utf-8",
    };
    let disposition = format!("attachment; filename=\"{}\"", result.file_name);

    let mut response = result.content.into_response();
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

pub(crate) async fn handle_requestlog_export_http(
    headers: HeaderMap,
    Query(params): Query<RequestLogExportParams>,
) -> AxumResponse {
    if let Some(response) = validate_export_headers(&headers) {
        return response;
    }

    match tokio::task::spawn_blocking(move || crate::requestlog_export::export_request_logs(params))
        .await
    {
        Ok(Ok(result)) => build_export_response(result),
        Ok(Err(err)) => (StatusCode::BAD_REQUEST, err).into_response(),
        Err(err) => {
            log::error!("requestlog export blocking task failed: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error: request log export failed",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_export_response;
    use axum::body::to_bytes;
    use axum::http::header;
    use codexmanager_core::rpc::types::RequestLogExportResult;

    #[tokio::test(flavor = "current_thread")]
    async fn export_response_sets_download_headers() {
        let response = build_export_response(RequestLogExportResult {
            format: "csv".to_string(),
            file_name: "requestlogs.csv".to_string(),
            content: "traceId\nabc\n".to_string(),
            record_count: 1,
        });

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/csv; charset=utf-8")
        );
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some("attachment; filename=\"requestlogs.csv\"")
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert_eq!(body.as_ref(), b"traceId\nabc\n");
    }
}
