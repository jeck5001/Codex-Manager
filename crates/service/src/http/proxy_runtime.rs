use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{header, Request as HttpRequest, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::routing::{any, get, post};
use axum::Router;
use reqwest::Client;
use std::io;
use std::time::Instant;

use crate::http::proxy_bridge::run_proxy_server;
use crate::http::proxy_request::{build_target_url, filter_request_headers};
use crate::http::proxy_response::{merge_upstream_headers, text_error_response};

#[derive(Clone)]
struct ProxyState {
    backend_base_url: String,
    client: Client,
}

fn log_proxy_error(status: StatusCode, target_url: &str, message: &str) {
    log::warn!(
        "event=front_proxy_error code={} status={} target_url={} message={}",
        crate::error_codes::classify_message(message).as_str(),
        status.as_u16(),
        target_url,
        message
    );
}

fn build_backend_base_url(backend_addr: &str) -> String {
    format!("http://{backend_addr}")
}

fn build_local_backend_client() -> Result<Client, reqwest::Error> {
    Client::builder().no_proxy().build()
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    request: HttpRequest<Body>,
) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let target_url = build_target_url(&state.backend_base_url, &parts.uri);
    let max_body_bytes = crate::gateway::front_proxy_max_body_bytes();

    if let Some(content_length) = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
    {
        if content_length > max_body_bytes as u64 {
            let message = format!("request body too large: content-length={content_length}");
            log_proxy_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                target_url.as_str(),
                message.as_str(),
            );
            return text_error_response(StatusCode::PAYLOAD_TOO_LARGE, message);
        }
    }

    let outbound_headers = filter_request_headers(&parts.headers);
    let body_bytes = match to_bytes(body, max_body_bytes).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let message = format!("request body too large: content-length>{max_body_bytes}");
            log_proxy_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                target_url.as_str(),
                message.as_str(),
            );
            return text_error_response(StatusCode::PAYLOAD_TOO_LARGE, message);
        }
    };

    let mut builder = state.client.request(parts.method, target_url.as_str());
    builder = builder.headers(outbound_headers);
    builder = builder.body(body_bytes);

    let upstream = match builder.send().await {
        Ok(response) => response,
        Err(err) => {
            let message = format!("backend proxy error: {err}");
            log_proxy_error(
                StatusCode::BAD_GATEWAY,
                target_url.as_str(),
                message.as_str(),
            );
            return text_error_response(StatusCode::BAD_GATEWAY, message);
        }
    };

    let response_builder = merge_upstream_headers(
        Response::builder().status(upstream.status()),
        upstream.headers(),
    );

    match response_builder.body(Body::from_stream(upstream.bytes_stream())) {
        Ok(response) => response,
        Err(err) => {
            let message = format!("build response failed: {err}");
            log_proxy_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                target_url.as_str(),
                message.as_str(),
            );
            text_error_response(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

async fn access_log(request: HttpRequest<Body>, next: Next) -> Response<Body> {
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let started_at = Instant::now();

    let response = next.run(request).await;
    let status = response.status();
    let elapsed_ms = started_at.elapsed().as_millis();

    log::info!(
        "event=http_access method={} path={} status={} duration_ms={}",
        method,
        path,
        status.as_u16(),
        elapsed_ms
    );

    response
}

fn build_front_proxy_app(state: ProxyState) -> Router {
    Router::new()
        .route("/rpc", post(crate::http::rpc_endpoint::handle_rpc_http))
        .route(
            "/export/auditlogs",
            get(crate::http::auditlog_export_endpoint::handle_auditlog_export_http),
        )
        .route(
            "/export/requestlogs",
            get(crate::http::requestlog_export_endpoint::handle_requestlog_export_http),
        )
        .fallback(any(proxy_handler))
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

pub(crate) fn run_front_proxy(addr: &str, backend_addr: &str) -> io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;

    runtime.block_on(async move {
        let client = build_local_backend_client().map_err(io::Error::other)?;
        let state = ProxyState {
            backend_base_url: build_backend_base_url(backend_addr),
            client,
        };
        let app = build_front_proxy_app(state);
        run_proxy_server(addr, app).await
    })
}

#[cfg(test)]
#[path = "tests/proxy_runtime_tests.rs"]
mod tests;
