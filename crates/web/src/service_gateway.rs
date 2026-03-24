use super::*;

const MANAGEMENT_SECRET_HEADER: &str = "x-codexmanager-management-secret";

pub(super) fn should_spawn_service() -> bool {
    read_env_trim("CODEXMANAGER_WEB_NO_SPAWN_SERVICE").is_none()
}

fn json_error_response(status: StatusCode, error: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({
            "error": error,
        })),
    )
        .into_response()
}

fn resolve_management_secret(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get(MANAGEMENT_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn management_auth_error_response(headers: &HeaderMap) -> Option<Response> {
    if !codexmanager_service::current_remote_management_enabled() {
        return Some(json_error_response(
            StatusCode::FORBIDDEN,
            "remote_management_disabled",
        ));
    }
    let Some(candidate) = resolve_management_secret(headers) else {
        return Some(json_error_response(
            StatusCode::UNAUTHORIZED,
            "remote_management_secret_required",
        ));
    };
    if !codexmanager_service::verify_remote_management_secret(&candidate) {
        return Some(json_error_response(
            StatusCode::UNAUTHORIZED,
            "remote_management_secret_invalid",
        ));
    }
    None
}

pub(super) async fn tcp_probe(addr: &str) -> bool {
    let addr = addr.trim();
    if addr.is_empty() {
        return false;
    }
    let addr = addr.strip_prefix("http://").unwrap_or(addr);
    let addr = addr.strip_prefix("https://").unwrap_or(addr);
    let addr = addr.split('/').next().unwrap_or(addr);
    tokio::time::timeout(
        Duration::from_millis(250),
        tokio::net::TcpStream::connect(addr),
    )
    .await
    .is_ok()
}

fn service_bin_path(dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return dir.join("codexmanager-service.exe");
    }
    #[cfg(not(target_os = "windows"))]
    {
        return dir.join("codexmanager-service");
    }
}

fn spawn_service_detached(dir: &Path, service_addr: &str) -> std::io::Result<()> {
    let bin = service_bin_path(dir);
    let mut cmd = Command::new(bin);
    let bind_addr = codexmanager_service::listener_bind_addr(service_addr);
    cmd.env("CODEXMANAGER_SERVICE_ADDR", bind_addr);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let _child = cmd.spawn()?;
    Ok(())
}

pub(super) async fn ensure_service_running(
    service_addr: &str,
    dir: &Path,
    spawned_service: &Arc<Mutex<bool>>,
) -> Option<String> {
    if tcp_probe(service_addr).await {
        return None;
    }
    if !should_spawn_service() {
        return Some(format!(
            "service not reachable at {service_addr} (spawn disabled)"
        ));
    }

    let bin = service_bin_path(dir);
    if !bin.is_file() {
        return Some(format!(
            "service not reachable at {service_addr} (missing {})",
            bin.display()
        ));
    }

    if let Err(err) = spawn_service_detached(dir, service_addr) {
        return Some(format!("failed to spawn service: {err}"));
    }
    *spawned_service.lock().await = true;

    for _ in 0..50 {
        if tcp_probe(service_addr).await {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Some(format!(
        "service still not reachable at {service_addr} after spawn"
    ))
}

pub(super) async fn rpc_proxy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_json_content_type(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "{}").into_response();
    }
    let resp = state
        .client
        .post(&state.service_rpc_url)
        .header("content-type", "application/json")
        .header("x-codexmanager-rpc-token", &state.rpc_token)
        .header(
            "x-codexmanager-operator",
            headers
                .get("x-codexmanager-operator")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("web-ui"),
        )
        .body(body)
        .send()
        .await;
    let resp = match resp {
        Ok(v) => v,
        Err(err) => {
            let msg = format!("upstream error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    let status = resp.status();
    let bytes = match resp.bytes().await {
        Ok(v) => v,
        Err(err) => {
            let msg = format!("upstream read error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };
    let mut out = Response::new(axum::body::Body::from(bytes));
    *out.status_mut() = status;
    out.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/json"),
    );
    out
}

pub(super) async fn management_rpc_proxy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(response) = management_auth_error_response(&headers) {
        return response;
    }
    if !is_json_content_type(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "{}").into_response();
    }

    let resp = state
        .client
        .post(&state.service_rpc_url)
        .header("content-type", "application/json")
        .header("x-codexmanager-rpc-token", &state.rpc_token)
        .header(
            "x-codexmanager-operator",
            headers
                .get("x-codexmanager-operator")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("remote-management"),
        )
        .body(body)
        .send()
        .await;
    let resp = match resp {
        Ok(v) => v,
        Err(err) => {
            let msg = format!("upstream error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    let status = resp.status();
    let bytes = match resp.bytes().await {
        Ok(v) => v,
        Err(err) => {
            let msg = format!("upstream read error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };
    let mut out = Response::new(axum::body::Body::from(bytes));
    *out.status_mut() = status;
    out.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/json"),
    );
    out
}

pub(super) async fn management_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = management_auth_error_response(&headers) {
        return response;
    }

    axum::Json(serde_json::json!({
        "enabled": codexmanager_service::current_remote_management_enabled(),
        "secretConfigured": codexmanager_service::remote_management_secret_configured(),
        "serviceAddr": state.service_addr,
        "serviceReachable": tcp_probe(&state.service_addr).await,
        "webAccessPasswordConfigured": codexmanager_service::web_access_password_configured(),
        "webAccessTwoFactorEnabled": codexmanager_service::web_auth_two_factor_enabled(),
    }))
    .into_response()
}

pub(super) async fn requestlog_export_proxy(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let query = request
        .uri()
        .query()
        .map(|value| format!("?{value}"))
        .unwrap_or_default();
    let target_url = format!("http://{}/export/requestlogs{}", state.service_addr, query);
    let resp = state
        .client
        .get(target_url)
        .header("x-codexmanager-rpc-token", &state.rpc_token)
        .send()
        .await;
    let resp = match resp {
        Ok(value) => value,
        Err(err) => {
            let msg = format!("upstream error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    let status = resp.status();
    let forwarded_headers = [
        axum::http::header::CONTENT_TYPE,
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::header::CACHE_CONTROL,
    ]
    .into_iter()
    .filter_map(|header_name| {
        resp.headers()
            .get(&header_name)
            .cloned()
            .map(|value| (header_name, value))
    })
    .collect::<Vec<_>>();
    let mut out = Response::new(axum::body::Body::from_stream(resp.bytes_stream()));
    *out.status_mut() = status;
    for (header_name, value) in forwarded_headers {
        out.headers_mut().insert(header_name, value);
    }
    out
}

pub(super) async fn auditlog_export_proxy(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let query = request
        .uri()
        .query()
        .map(|value| format!("?{value}"))
        .unwrap_or_default();
    let target_url = format!("http://{}/export/auditlogs{}", state.service_addr, query);
    let resp = state
        .client
        .get(target_url)
        .header("x-codexmanager-rpc-token", &state.rpc_token)
        .send()
        .await;
    let resp = match resp {
        Ok(value) => value,
        Err(err) => {
            let msg = format!("upstream error: {err}");
            return (StatusCode::BAD_GATEWAY, msg).into_response();
        }
    };

    let status = resp.status();
    let forwarded_headers = [
        axum::http::header::CONTENT_TYPE,
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::header::CACHE_CONTROL,
    ]
    .into_iter()
    .filter_map(|header_name| {
        resp.headers()
            .get(&header_name)
            .cloned()
            .map(|value| (header_name, value))
    })
    .collect::<Vec<_>>();
    let mut out = Response::new(axum::body::Body::from_stream(resp.bytes_stream()));
    *out.status_mut() = status;
    for (header_name, value) in forwarded_headers {
        out.headers_mut().insert(header_name, value);
    }
    out
}

pub(super) async fn quit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if *state.spawned_service.lock().await {
        let addr = state.service_addr.clone();
        let _ = tokio::task::spawn_blocking(move || {
            codexmanager_service::request_shutdown(&addr);
        })
        .await;
    }
    let _ = state.shutdown_tx.send(true);
    Html("<html><body>OK</body></html>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_management_secret_prefers_explicit_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            MANAGEMENT_SECRET_HEADER,
            HeaderValue::from_static("secret-header"),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret-bearer"),
        );

        assert_eq!(
            resolve_management_secret(&headers).as_deref(),
            Some("secret-header")
        );
    }

    #[test]
    fn resolve_management_secret_accepts_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret-bearer"),
        );

        assert_eq!(
            resolve_management_secret(&headers).as_deref(),
            Some("secret-bearer")
        );
    }

    #[test]
    fn management_auth_error_requires_feature_enablement() {
        let headers = HeaderMap::new();

        let response = management_auth_error_response(&headers).expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
