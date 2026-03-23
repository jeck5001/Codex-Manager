use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive};
use axum::response::Sse;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

type SessionMap = Arc<Mutex<HashMap<String, UnboundedSender<Value>>>>;
type SseEventStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

#[derive(Clone, Default)]
struct HttpSseState {
    sessions: SessionMap,
}

#[derive(Debug)]
enum HttpSseError {
    Disabled(String),
    MissingSession,
    SessionClosed,
}

impl HttpSseError {
    fn into_status_message(self) -> (StatusCode, String) {
        match self {
            Self::Disabled(message) => (StatusCode::FORBIDDEN, message),
            Self::MissingSession => (
                StatusCode::NOT_FOUND,
                "MCP SSE session not found; reconnect /sse first".to_string(),
            ),
            Self::SessionClosed => (
                StatusCode::GONE,
                "MCP SSE session is already closed; reconnect /sse first".to_string(),
            ),
        }
    }
}

#[derive(Deserialize)]
struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: String,
}

pub fn run_http_sse_server() -> Result<(), String> {
    crate::portable::bootstrap_current_process();
    crate::storage_helpers::initialize_storage()
        .map_err(|err| format!("initialize storage failed: {err}"))?;
    crate::sync_runtime_settings_from_storage();

    let port = crate::current_mcp_port();
    let addr = format!("127.0.0.1:{port}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("build MCP HTTP SSE runtime failed: {err}"))?;

    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr.as_str())
            .await
            .map_err(|err| format!("bind MCP HTTP SSE listener failed on {addr}: {err}"))?;
        eprintln!("codexmanager-mcp HTTP SSE listening on http://{addr}/sse");
        axum::serve(listener, build_http_sse_app())
            .await
            .map_err(|err| format!("serve MCP HTTP SSE failed: {err}"))
    })
}

fn build_http_sse_app() -> Router {
    Router::new()
        .route("/sse", get(handle_sse_connect))
        .route("/message", post(handle_message_post))
        .with_state(HttpSseState::default())
}

async fn handle_sse_connect(
    State(state): State<HttpSseState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let (_session_id, endpoint_url, receiver) =
        create_sse_session(&state, &headers).map_err(HttpSseError::into_status_message)?;
    Ok(build_sse_response(endpoint_url, receiver))
}

async fn handle_message_post(
    State(state): State<HttpSseState>,
    Query(query): Query<MessageQuery>,
    Json(payload): Json<Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    dispatch_session_message(&state, query.session_id.as_str(), payload)
        .map(|_| StatusCode::ACCEPTED)
        .map_err(HttpSseError::into_status_message)
}

fn build_sse_response(endpoint_url: String, receiver: UnboundedReceiver<Value>) -> Response {
    let initial = stream::once(async move {
        Ok(Event::default().event("endpoint").data(
            json!({
                "messageUrl": endpoint_url
            })
            .to_string(),
        ))
    });
    let responses = UnboundedReceiverStream::new(receiver)
        .map(|response| Ok(Event::default().event("message").data(response.to_string())));
    let stream: SseEventStream = Box::pin(initial.chain(responses));
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keepalive"),
        )
        .into_response()
}

fn create_sse_session(
    state: &HttpSseState,
    headers: &HeaderMap,
) -> Result<(String, String, UnboundedReceiver<Value>), HttpSseError> {
    crate::mcp::session::ensure_server_enabled().map_err(HttpSseError::Disabled)?;

    let session_id = generate_session_id();
    let endpoint_url = build_message_endpoint_url(headers, session_id.as_str());
    let (sender, receiver) = mpsc::unbounded_channel();
    state
        .sessions
        .lock()
        .expect("lock MCP SSE sessions")
        .insert(session_id.clone(), sender);

    Ok((session_id, endpoint_url, receiver))
}

fn dispatch_session_message(
    state: &HttpSseState,
    session_id: &str,
    payload: Value,
) -> Result<(), HttpSseError> {
    crate::mcp::session::ensure_server_enabled().map_err(HttpSseError::Disabled)?;

    let response = crate::mcp::session::handle_jsonrpc_request(payload);
    let Some(response) = response else {
        return Ok(());
    };

    let sender = {
        let sessions = state.sessions.lock().expect("lock MCP SSE sessions");
        sessions
            .get(session_id)
            .cloned()
            .ok_or(HttpSseError::MissingSession)?
    };

    if sender.send(response).is_ok() {
        return Ok(());
    }

    state
        .sessions
        .lock()
        .expect("lock MCP SSE sessions")
        .remove(session_id);
    Err(HttpSseError::SessionClosed)
}

fn build_message_endpoint_url(headers: &HeaderMap, session_id: &str) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("http");
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("127.0.0.1:{}", crate::current_mcp_port()));
    format!("{scheme}://{host}/message?sessionId={session_id}")
}

fn generate_session_id() -> String {
    format!(
        "mcp-{}-{:016x}",
        codexmanager_core::storage::now_ts(),
        rand::random::<u64>()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_message_endpoint_url, create_sse_session, dispatch_session_message, HttpSseError,
        HttpSseState,
    };
    use axum::http::{header, HeaderMap, HeaderValue};
    use codexmanager_core::storage::{now_ts, Storage};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::MutexGuard;

    static TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    struct TestDbScope {
        _env_lock: MutexGuard<'static, ()>,
        _db_guard: EnvGuard,
        db_path: PathBuf,
    }

    impl Drop for TestDbScope {
        fn drop(&mut self) {
            crate::storage_helpers::clear_storage_cache_for_tests();
            let _ = fs::remove_file(&self.db_path);
            let _ = fs::remove_file(format!("{}-shm", self.db_path.display()));
            let _ = fs::remove_file(format!("{}-wal", self.db_path.display()));
        }
    }

    fn setup_test_db(prefix: &str) -> (TestDbScope, Storage) {
        let env_lock = crate::lock_utils::process_env_test_guard();
        crate::storage_helpers::clear_storage_cache_for_tests();
        let mut db_path = std::env::temp_dir();
        db_path.push(format!(
            "{prefix}-{}-{}-{}.db",
            std::process::id(),
            now_ts(),
            TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
        let storage = Storage::open(&db_path).expect("open db");
        storage.init().expect("init schema");
        (
            TestDbScope {
                _env_lock: env_lock,
                _db_guard: db_guard,
                db_path,
            },
            storage,
        )
    }

    #[test]
    fn http_sse_builds_message_endpoint_from_request_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("localhost:48762"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));

        let endpoint = build_message_endpoint_url(&headers, "session-1");

        assert_eq!(
            endpoint,
            "https://localhost:48762/message?sessionId=session-1"
        );
    }

    #[test]
    fn http_sse_session_dispatches_initialize_response() {
        let (_db_scope, _storage) = setup_test_db("mcp-http-sse-init");
        crate::set_mcp_enabled(true).expect("enable mcp");

        let state = HttpSseState::default();
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("localhost:48762"));

        let (session_id, _endpoint_url, mut receiver) =
            create_sse_session(&state, &headers).expect("create sse session");

        dispatch_session_message(
            &state,
            session_id.as_str(),
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26"
                }
            }),
        )
        .expect("dispatch initialize");

        let response = receiver.try_recv().expect("receive initialize response");
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn http_sse_session_rejects_when_mcp_is_disabled() {
        let (_db_scope, _storage) = setup_test_db("mcp-http-sse-disabled");
        crate::set_mcp_enabled(false).expect("disable mcp");

        let state = HttpSseState::default();
        let error = dispatch_session_message(
            &state,
            "missing-session",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26"
                }
            }),
        )
        .expect_err("disabled request should fail");

        assert!(matches!(error, HttpSseError::Disabled(message) if message.contains("设置中禁用")));
    }
}
