use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response as AxumResponse};
use codexmanager_core::rpc::types::AuditLogExportParams;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

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

fn build_export_response(format: &str, file_name: &str, body: Body) -> AxumResponse {
    let content_type = match format.trim().to_ascii_lowercase().as_str() {
        "json" => "application/json; charset=utf-8",
        _ => "text/csv; charset=utf-8",
    };
    let disposition = format!("attachment; filename=\"{file_name}\"");

    let mut response = body.into_response();
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

pub(crate) async fn handle_auditlog_export_http(
    headers: HeaderMap,
    Query(params): Query<AuditLogExportParams>,
) -> AxumResponse {
    if let Some(response) = validate_export_headers(&headers) {
        return response;
    }

    let plan = match crate::audit_export::prepare_audit_log_export(params) {
        Ok(plan) => plan,
        Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
    };

    let format = plan.format.to_string();
    let file_name = plan.file_name.clone();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, Infallible>>(8);
    tokio::task::spawn_blocking(move || {
        if let Err(err) = crate::audit_export::stream_audit_log_export_chunks(plan, tx) {
            log::error!("auditlog export streaming task failed: {}", err);
        }
    });

    build_export_response(
        &format,
        &file_name,
        Body::from_stream(ReceiverStream::new(rx)),
    )
}
