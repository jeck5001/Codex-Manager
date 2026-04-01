use codexmanager_core::rpc::types::JsonRpcRequest;
use codexmanager_core::storage::{
    now_ts, Account, RequestLog, RequestTokenStat, Storage, UsageSnapshotRecord,
};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server, StatusCode};

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

static RPC_TEST_ENV_LOCK: Mutex<()> = Mutex::new(());
static RPC_TEST_DIR_SEQ: AtomicUsize = AtomicUsize::new(0);

fn lock_rpc_test_env() -> MutexGuard<'static, ()> {
    // 中文注释：RPC 集成测试依赖进程级环境变量，串行化可避免不同用例互相污染数据库路径。
    RPC_TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn new_test_dir(prefix: &str) -> PathBuf {
    // 中文注释：用进程号 + 自增序号构造临时目录，避免 Windows 复用旧目录导致脏数据串用。
    let seq = RPC_TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("{prefix}-{}-{seq}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

struct RpcTestContext {
    _env_lock: MutexGuard<'static, ()>,
    _db_path_guard: EnvGuard,
    dir: PathBuf,
}

impl RpcTestContext {
    fn new(prefix: &str) -> Self {
        let env_lock = lock_rpc_test_env();
        let dir = new_test_dir(prefix);
        let db_path = dir.join("codexmanager.db");
        let db_path_guard =
            EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
        Self {
            _env_lock: env_lock,
            _db_path_guard: db_path_guard,
            dir,
        }
    }

    fn db_path(&self) -> PathBuf {
        self.dir.join("codexmanager.db")
    }

    fn seed_accounts(&self, count: usize) {
        let storage = Storage::open(self.db_path()).expect("open db");
        storage.init().expect("init schema");
        let now = now_ts();
        for idx in 0..count {
            let sort = idx as i64;
            storage
                .insert_account(&Account {
                    id: format!("acc-{idx}"),
                    label: format!("Account {idx}"),
                    issuer: "https://auth.openai.com".to_string(),
                    chatgpt_account_id: Some(format!("chatgpt-{idx}")),
                    workspace_id: Some(format!("workspace-{idx}")),
                    group_name: Some(format!("group-{}", idx % 2)),
                    sort,
                    status: "active".to_string(),
                    created_at: now + sort,
                    updated_at: now + sort,
                })
                .expect("insert account");
        }
    }
}

impl Drop for RpcTestContext {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn post_rpc_raw(addr: &str, body: &str, headers: &[(&str, &str)]) -> (u16, String) {
    let mut stream = connect_with_retry(addr);
    let mut request = format!("POST /rpc HTTP/1.1\r\nHost: {addr}\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n{}", body.len(), body));
    stream.write_all(request.as_bytes()).expect("write");
    stream.shutdown(std::net::Shutdown::Write).ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read");
    let status = buf
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .expect("status");
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn connect_with_retry(addr: &str) -> TcpStream {
    let mut last_error = None;
    for _ in 0..100 {
        match TcpStream::connect(addr) {
            Ok(stream) => return stream,
            Err(err) => {
                last_error = Some(err);
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    panic!("connect server: {:?}", last_error.expect("connect error"));
}

fn post_rpc(addr: &str, body: &str) -> serde_json::Value {
    let token = codexmanager_service::rpc_auth_token().to_string();
    let (status, body) = post_rpc_raw(
        addr,
        body,
        &[
            ("Content-Type", "application/json"),
            ("X-CodexManager-Rpc-Token", token.as_str()),
        ],
    );
    assert_eq!(status, 200, "unexpected status {status}: {body}");
    serde_json::from_str(&body).expect("parse response")
}

fn encode_base64url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let chunk = ((bytes[index] as u32) << 16)
            | ((bytes[index + 1] as u32) << 8)
            | (bytes[index + 2] as u32);
        out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(TABLE[(chunk & 0x3f) as usize] as char);
        index += 3;
    }
    match bytes.len().saturating_sub(index) {
        1 => {
            let chunk = (bytes[index] as u32) << 16;
            out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = ((bytes[index] as u32) << 16) | ((bytes[index + 1] as u32) << 8);
            out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }
    out
}

fn build_access_token(
    subject: &str,
    email: &str,
    chatgpt_account_id: &str,
    plan_type: &str,
) -> String {
    let header = encode_base64url(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = serde_json::json!({
        "sub": subject,
        "email": email,
        "workspace_id": chatgpt_account_id,
        "https://api.openai.com/auth": {
            "chatgpt_account_id": chatgpt_account_id,
            "chatgpt_plan_type": plan_type
        }
    });
    let payload = encode_base64url(
        serde_json::to_string(&payload)
            .expect("serialize jwt payload")
            .as_bytes(),
    );
    format!("{header}.{payload}.sig")
}

fn start_mock_oauth_token_server(
    status: u16,
    response_body: String,
) -> (
    String,
    std::sync::mpsc::Receiver<String>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock oauth server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let mut request = server.recv().expect("receive oauth request");
        let mut body = String::new();
        request
            .as_reader()
            .read_to_string(&mut body)
            .expect("read oauth request body");
        tx.send(body).expect("send oauth request body");
        let response = Response::from_string(response_body)
            .with_status_code(StatusCode(status))
            .with_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
        request.respond(response).expect("respond oauth request");
    });
    (addr, rx, handle)
}

fn start_mock_session_refresh_server(
    response_body: String,
) -> (
    String,
    std::sync::mpsc::Receiver<String>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock session refresh server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let request = server.recv().expect("receive session refresh request");
        let cookie_header = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Cookie"))
            .map(|header| header.value.as_str().to_string())
            .unwrap_or_default();
        tx.send(cookie_header).expect("send session refresh cookie");
        let response = Response::from_string(response_body)
            .with_status_code(StatusCode(200))
            .with_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
        request
            .respond(response)
            .expect("respond session refresh request");
    });
    (addr, rx, handle)
}

fn start_mock_register_service_server(
    expected_email: String,
    refreshed_access_token: String,
    refreshed_refresh_token: String,
    refreshed_id_token: String,
    cookies: String,
) -> (
    String,
    std::sync::mpsc::Receiver<(String, String)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock register server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..3 {
            let request = server.recv().expect("receive register request");
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();
            tx.send((method.clone(), path.clone()))
                .expect("send register request");

            let response_body = if method == "GET" && path.starts_with("/api/accounts?") {
                serde_json::json!({
                    "accounts": [{
                        "id": 123,
                        "email": expected_email,
                        "account_id": "chatgpt-register-fallback",
                        "workspace_id": "workspace-register-fallback",
                        "cookies": cookies,
                    }]
                })
                .to_string()
            } else if method == "POST" && path == "/api/accounts/123/refresh" {
                serde_json::json!({
                    "success": true,
                    "message": "Token refreshed"
                })
                .to_string()
            } else if method == "GET" && path == "/api/accounts/123/tokens" {
                serde_json::json!({
                    "access_token": refreshed_access_token,
                    "refresh_token": refreshed_refresh_token,
                    "id_token": refreshed_id_token
                })
                .to_string()
            } else {
                serde_json::json!({
                    "error": format!("unexpected request: {method} {path}")
                })
                .to_string()
            };

            let response = Response::from_string(response_body)
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json")
                        .expect("content-type header"),
                );
            request.respond(response).expect("respond register request");
        }
    });
    (addr, rx, handle)
}

fn start_mock_register_service_server_with_session_token_only(
    expected_email: String,
    imported_access_token: String,
    session_token: String,
) -> (
    String,
    std::sync::mpsc::Receiver<(String, String)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock register session-token server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let request = server.recv().expect("receive register request");
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();
            tx.send((method.clone(), path.clone()))
                .expect("send register request");

            let response_body = if method == "GET" && path.starts_with("/api/accounts?") {
                serde_json::json!({
                    "accounts": [{
                        "id": 777,
                        "email": expected_email,
                        "account_id": "chatgpt-session-import",
                        "workspace_id": "workspace-session-import",
                        "cookies": ""
                    }]
                })
                .to_string()
            } else if method == "GET" && path == "/api/accounts/777/tokens" {
                serde_json::json!({
                    "access_token": imported_access_token,
                    "refresh_token": "",
                    "id_token": "",
                    "session_token": session_token
                })
                .to_string()
            } else {
                serde_json::json!({
                    "error": format!("unexpected request: {method} {path}")
                })
                .to_string()
            };

            let response = Response::from_string(response_body)
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json")
                        .expect("content-type header"),
                );
            request.respond(response).expect("respond register request");
        }
    });
    (addr, rx, handle)
}

fn start_mock_register_service_server_email_miss_account_match(
    missed_email: String,
    account_search: String,
    remote_email: String,
    refreshed_access_token: String,
    refreshed_refresh_token: String,
    refreshed_id_token: String,
    cookies: String,
) -> (
    String,
    std::sync::mpsc::Receiver<(String, String)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock register fallback server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..4 {
            let request = server.recv().expect("receive register request");
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();
            tx.send((method.clone(), path.clone()))
                .expect("send register request");

            let response_body = if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&missed_email)
                    ) {
                serde_json::json!({ "accounts": [] }).to_string()
            } else if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&account_search)
                    )
            {
                serde_json::json!({
                    "accounts": [{
                        "id": 321,
                        "email": remote_email,
                        "account_id": account_search,
                        "workspace_id": "workspace-register-fallback",
                        "cookies": cookies,
                    }]
                })
                .to_string()
            } else if method == "POST" && path == "/api/accounts/321/refresh" {
                serde_json::json!({
                    "success": true,
                    "message": "Token refreshed"
                })
                .to_string()
            } else if method == "GET" && path == "/api/accounts/321/tokens" {
                serde_json::json!({
                    "access_token": refreshed_access_token,
                    "refresh_token": refreshed_refresh_token,
                    "id_token": refreshed_id_token
                })
                .to_string()
            } else {
                serde_json::json!({
                    "error": format!("unexpected request: {method} {path}")
                })
                .to_string()
            };

            let response = Response::from_string(response_body)
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json")
                        .expect("content-type header"),
                );
            request.respond(response).expect("respond register request");
        }
    });
    (addr, rx, handle)
}

fn start_mock_recovery_manager_server(
    email: String,
    remote_account_id: String,
    chatgpt_account_id: String,
    workspace_id: String,
    refreshed_access_token: String,
    refreshed_refresh_token: String,
    refreshed_id_token: String,
) -> (
    String,
    std::sync::mpsc::Receiver<(String, serde_json::Value)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock recovery manager server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let mut request = server.recv().expect("receive recovery manager request");
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .expect("read recovery manager body");
            let payload: serde_json::Value =
                serde_json::from_str(&body).expect("parse recovery manager body");
            let method = payload
                .get("method")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            tx.send((method.clone(), payload.clone()))
                .expect("send recovery manager request");

            let response_body = if method == "account/list" {
                serde_json::json!({
                    "id": payload.get("id").cloned().unwrap_or(serde_json::json!(1)),
                    "result": {
                        "items": [{
                            "id": remote_account_id,
                            "label": email,
                            "groupName": "REMOTE",
                            "tags": [],
                            "sort": 0,
                            "status": "active",
                            "healthScore": 100
                        }],
                        "total": 1,
                        "page": 1,
                        "pageSize": 500
                    }
                })
                .to_string()
            } else if method == "account/exportData" {
                serde_json::json!({
                    "id": payload.get("id").cloned().unwrap_or(serde_json::json!(2)),
                    "result": {
                        "totalAccounts": 1,
                        "exported": 1,
                        "skippedMissingToken": 0,
                        "files": [{
                            "fileName": "remote-account.json",
                            "content": serde_json::json!({
                                "tokens": {
                                    "access_token": refreshed_access_token,
                                    "refresh_token": refreshed_refresh_token,
                                    "id_token": refreshed_id_token,
                                    "account_id": remote_account_id,
                                },
                                "meta": {
                                    "label": email,
                                    "issuer": "https://auth.openai.com",
                                    "groupName": "REMOTE",
                                    "status": "active",
                                    "workspaceId": workspace_id,
                                    "chatgptAccountId": chatgpt_account_id,
                                    "exportedAt": 1
                                }
                            })
                            .to_string()
                        }]
                    }
                })
                .to_string()
            } else {
                serde_json::json!({
                    "id": payload.get("id").cloned().unwrap_or(serde_json::json!(999)),
                    "result": {
                        "error": format!("unexpected method: {method}")
                    }
                })
                .to_string()
            };

            let response = Response::from_string(response_body)
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json")
                        .expect("content-type header"),
                );
            request
                .respond(response)
                .expect("respond recovery manager request");
        }
    });
    (addr, rx, handle)
}

fn start_mock_register_service_probe_server() -> (
    String,
    std::sync::mpsc::Receiver<(String, String)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock register probe server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            let Some(request) = server
                .recv_timeout(Duration::from_millis(200))
                .expect("receive register probe request")
            else {
                continue;
            };
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();
            tx.send((method.clone(), path.clone()))
                .expect("send register probe request");

            let response = Response::from_string(
                serde_json::json!({
                    "error": format!("register probe should not be called: {method} {path}")
                })
                .to_string(),
            )
            .with_status_code(StatusCode(500))
            .with_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
            request
                .respond(response)
                .expect("respond register probe request");
        }
    });
    (addr, rx, handle)
}

fn start_mock_register_service_server_auto_register_login(
    old_email: String,
    old_chatgpt_account_id: String,
    old_workspace_id: String,
    new_email: String,
    new_chatgpt_account_id: String,
    new_workspace_id: String,
    refreshed_access_token: String,
    refreshed_refresh_token: String,
    refreshed_id_token: String,
) -> (
    String,
    std::sync::mpsc::Receiver<(String, String)>,
    std::sync::mpsc::Receiver<String>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock auto register server");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = std::sync::mpsc::channel();
    let (body_tx, body_rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..11 {
            let mut request = server.recv().expect("receive auto register request");
            let method = request.method().as_str().to_string();
            let path = request.url().to_string();
            tx.send((method.clone(), path.clone()))
                .expect("send auto register request");

            let response_body = if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&old_email)
                    ) {
                serde_json::json!({ "accounts": [] }).to_string()
            } else if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&old_chatgpt_account_id)
                    )
            {
                serde_json::json!({ "accounts": [] }).to_string()
            } else if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&old_workspace_id)
                    )
            {
                serde_json::json!({ "accounts": [] }).to_string()
            } else if method == "GET" && path == "/api/registration/available-services" {
                serde_json::json!({
                    "customDomain": {
                        "available": true,
                        "services": [{
                            "id": 12,
                            "name": "Recover Domain"
                        }]
                    }
                })
                .to_string()
            } else if method == "POST" && path == "/api/registration/batch" {
                let mut request_body = String::new();
                request
                    .as_reader()
                    .read_to_string(&mut request_body)
                    .expect("read auto register batch body");
                body_tx
                    .send(request_body)
                    .expect("send auto register batch body");
                serde_json::json!({
                    "tasks": [{
                        "task_uuid": "task-auto-1"
                    }]
                })
                .to_string()
            } else if method == "GET" && path == "/api/registration/tasks/task-auto-1" {
                serde_json::json!({
                    "status": "completed",
                    "email_service_id": 12,
                    "result": {
                        "email": new_email,
                        "account_id": new_chatgpt_account_id,
                        "workspace_id": new_workspace_id
                    }
                })
                .to_string()
            } else if method == "GET" && path == "/api/registration/tasks/task-auto-1/logs" {
                serde_json::json!({
                    "logs": []
                })
                .to_string()
            } else if method == "GET"
                && path
                    == format!(
                        "/api/accounts?page=1&page_size=20&search={}",
                        urlencoding::encode(&new_email)
                    )
            {
                serde_json::json!({
                    "accounts": [{
                        "id": 900,
                        "email": new_email,
                        "account_id": new_chatgpt_account_id,
                        "workspace_id": new_workspace_id
                    }]
                })
                .to_string()
            } else if method == "GET" && path == "/api/accounts/900/tokens" {
                serde_json::json!({
                    "access_token": refreshed_access_token,
                    "refresh_token": refreshed_refresh_token,
                    "id_token": refreshed_id_token
                })
                .to_string()
            } else {
                serde_json::json!({
                    "error": format!("unexpected request: {method} {path}")
                })
                .to_string()
            };

            let response = Response::from_string(response_body)
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json")
                        .expect("content-type header"),
                );
            request
                .respond(response)
                .expect("respond auto register request");
        }
    });
    (addr, rx, body_rx, handle)
}

#[test]
fn rpc_initialize_roundtrip() {
    let _ctx = RpcTestContext::new("rpc-initialize");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 1,
        method: "initialize".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    assert_eq!(result.get("server_name").unwrap(), "codexmanager-service");
}

#[test]
fn rpc_account_list_empty_uses_default_pagination() {
    let _ctx = RpcTestContext::new("rpc-account-list-empty");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 2,
        method: "account/list".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");

    let items = result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("items array");
    assert!(items.is_empty(), "expected empty items, got: {result}");
    assert_eq!(
        result.get("total").and_then(|value| value.as_i64()),
        Some(0)
    );
    assert_eq!(result.get("page").and_then(|value| value.as_i64()), Some(1));
    assert_eq!(
        result.get("pageSize").and_then(|value| value.as_i64()),
        Some(5)
    );
}

#[test]
fn rpc_account_list_supports_pagination() {
    let ctx = RpcTestContext::new("rpc-account-list-page");
    ctx.seed_accounts(7);
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 3,
        method: "account/list".to_string(),
        params: Some(serde_json::json!({"page": 2, "pageSize": 3})),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");

    let items = result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("items array");
    assert_eq!(items.len(), 3, "unexpected page size: {result}");
    assert_eq!(
        result.get("total").and_then(|value| value.as_i64()),
        Some(7)
    );
    assert_eq!(result.get("page").and_then(|value| value.as_i64()), Some(2));
    assert_eq!(
        result.get("pageSize").and_then(|value| value.as_i64()),
        Some(3)
    );

    let ids = items
        .iter()
        .map(|value| {
            value
                .get("id")
                .and_then(|value| value.as_str())
                .expect("item id")
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["acc-3", "acc-4", "acc-5"]);
    assert_eq!(
        items[0].get("status").and_then(|value| value.as_str()),
        Some("active")
    );
}

#[test]
fn rpc_app_settings_set_invalid_payload_returns_structured_error() {
    let _ctx = RpcTestContext::new("rpc-app-settings-invalid-payload");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 30,
        method: "appSettings/set".to_string(),
        params: Some(serde_json::json!("invalid-payload")),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");

    let message = result
        .get("error")
        .and_then(|value| value.as_str())
        .expect("error message");
    assert!(
        message.starts_with("invalid app settings payload:"),
        "unexpected message: {message}"
    );
    assert_eq!(
        result.get("errorCode").and_then(|value| value.as_str()),
        Some("invalid_settings_payload")
    );
    let detail = result.get("errorDetail").expect("errorDetail");
    assert_eq!(
        detail.get("code").and_then(|value| value.as_str()),
        Some("invalid_settings_payload")
    );
    assert_eq!(
        detail.get("message").and_then(|value| value.as_str()),
        Some(message)
    );
}

#[test]
fn rpc_app_settings_can_roundtrip_free_account_max_model() {
    let _ctx = RpcTestContext::new("rpc-app-settings-free-max-model");
    let set_server = codexmanager_service::start_one_shot_server().expect("start server");

    let set_req = JsonRpcRequest {
        id: 31,
        method: "appSettings/set".to_string(),
        params: Some(serde_json::json!({
            "freeAccountMaxModel": "gpt-5.3-codex"
        })),
    };
    let set_json = serde_json::to_string(&set_req).expect("serialize");
    let set_resp = post_rpc(&set_server.addr, &set_json);
    let set_result = set_resp.get("result").expect("result");
    assert_eq!(
        set_result
            .get("freeAccountMaxModel")
            .and_then(|value| value.as_str()),
        Some("gpt-5.3-codex")
    );

    let get_server = codexmanager_service::start_one_shot_server().expect("start server");
    let get_req = JsonRpcRequest {
        id: 32,
        method: "appSettings/get".to_string(),
        params: None,
    };
    let get_json = serde_json::to_string(&get_req).expect("serialize");
    let get_resp = post_rpc(&get_server.addr, &get_json);
    let get_result = get_resp.get("result").expect("result");
    assert_eq!(
        get_result
            .get("freeAccountMaxModel")
            .and_then(|value| value.as_str()),
        Some("gpt-5.3-codex")
    );
}

#[test]
fn rpc_app_settings_can_roundtrip_quota_protection_settings() {
    let _ctx = RpcTestContext::new("rpc-app-settings-quota-protection");
    let set_server = codexmanager_service::start_one_shot_server().expect("start server");

    let set_req = JsonRpcRequest {
        id: 33,
        method: "appSettings/set".to_string(),
        params: Some(serde_json::json!({
            "quotaProtectionEnabled": true,
            "quotaProtectionThresholdPercent": 8
        })),
    };
    let set_json = serde_json::to_string(&set_req).expect("serialize");
    let set_resp = post_rpc(&set_server.addr, &set_json);
    let set_result = set_resp.get("result").expect("result");
    assert_eq!(
        set_result
            .get("quotaProtectionEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_result
            .get("quotaProtectionThresholdPercent")
            .and_then(|value| value.as_u64()),
        Some(8)
    );

    let get_server = codexmanager_service::start_one_shot_server().expect("start server");
    let get_req = JsonRpcRequest {
        id: 34,
        method: "appSettings/get".to_string(),
        params: None,
    };
    let get_json = serde_json::to_string(&get_req).expect("serialize");
    let get_resp = post_rpc(&get_server.addr, &get_json);
    let get_result = get_resp.get("result").expect("result");
    assert_eq!(
        get_result
            .get("quotaProtectionEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        get_result
            .get("quotaProtectionThresholdPercent")
            .and_then(|value| value.as_u64()),
        Some(8)
    );
}

#[test]
fn rpc_app_settings_can_roundtrip_auto_register_pool_settings() {
    let _ctx = RpcTestContext::new("rpc-app-settings-auto-register-pool");
    let set_server = codexmanager_service::start_one_shot_server().expect("start server");

    let set_req = JsonRpcRequest {
        id: 35,
        method: "appSettings/set".to_string(),
        params: Some(serde_json::json!({
            "backgroundTasks": {
                "autoRegisterPoolEnabled": true,
                "autoRegisterReadyAccountCount": 3,
                "autoRegisterReadyRemainPercent": 25
            }
        })),
    };
    let set_json = serde_json::to_string(&set_req).expect("serialize");
    let set_resp = post_rpc(&set_server.addr, &set_json);
    let set_result = set_resp.get("result").expect("result");
    let set_background_tasks = set_result
        .get("backgroundTasks")
        .expect("background tasks snapshot");
    assert_eq!(
        set_background_tasks
            .get("autoRegisterPoolEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_background_tasks
            .get("autoRegisterReadyAccountCount")
            .and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        set_background_tasks
            .get("autoRegisterReadyRemainPercent")
            .and_then(|value| value.as_u64()),
        Some(25)
    );

    let get_server = codexmanager_service::start_one_shot_server().expect("start server");
    let get_req = JsonRpcRequest {
        id: 36,
        method: "appSettings/get".to_string(),
        params: None,
    };
    let get_json = serde_json::to_string(&get_req).expect("serialize");
    let get_resp = post_rpc(&get_server.addr, &get_json);
    let get_result = get_resp.get("result").expect("result");
    let get_background_tasks = get_result
        .get("backgroundTasks")
        .expect("background tasks snapshot");
    assert_eq!(
        get_background_tasks
            .get("autoRegisterPoolEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        get_background_tasks
            .get("autoRegisterReadyAccountCount")
            .and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        get_background_tasks
            .get("autoRegisterReadyRemainPercent")
            .and_then(|value| value.as_u64()),
        Some(25)
    );
}

#[test]
fn rpc_app_settings_can_roundtrip_auto_disable_risky_accounts_settings() {
    let _ctx = RpcTestContext::new("rpc-app-settings-auto-disable-risky");
    let set_server = codexmanager_service::start_one_shot_server().expect("start server");

    let set_req = JsonRpcRequest {
        id: 37,
        method: "appSettings/set".to_string(),
        params: Some(serde_json::json!({
            "backgroundTasks": {
                "autoDisableRiskyAccountsEnabled": true,
                "autoDisableRiskyAccountsFailureThreshold": 4,
                "autoDisableRiskyAccountsHealthScoreThreshold": 55,
                "autoDisableRiskyAccountsLookbackMins": 90
            }
        })),
    };
    let set_json = serde_json::to_string(&set_req).expect("serialize");
    let set_resp = post_rpc(&set_server.addr, &set_json);
    let set_result = set_resp.get("result").expect("result");
    let set_background_tasks = set_result
        .get("backgroundTasks")
        .expect("background tasks snapshot");
    assert_eq!(
        set_background_tasks
            .get("autoDisableRiskyAccountsEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_background_tasks
            .get("autoDisableRiskyAccountsFailureThreshold")
            .and_then(|value| value.as_u64()),
        Some(4)
    );
    assert_eq!(
        set_background_tasks
            .get("autoDisableRiskyAccountsHealthScoreThreshold")
            .and_then(|value| value.as_u64()),
        Some(55)
    );
    assert_eq!(
        set_background_tasks
            .get("autoDisableRiskyAccountsLookbackMins")
            .and_then(|value| value.as_u64()),
        Some(90)
    );

    let get_server = codexmanager_service::start_one_shot_server().expect("start server");
    let get_req = JsonRpcRequest {
        id: 38,
        method: "appSettings/get".to_string(),
        params: None,
    };
    let get_json = serde_json::to_string(&get_req).expect("serialize");
    let get_resp = post_rpc(&get_server.addr, &get_json);
    let get_result = get_resp.get("result").expect("result");
    let get_background_tasks = get_result
        .get("backgroundTasks")
        .expect("background tasks snapshot");
    assert_eq!(
        get_background_tasks
            .get("autoDisableRiskyAccountsEnabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        get_background_tasks
            .get("autoDisableRiskyAccountsFailureThreshold")
            .and_then(|value| value.as_u64()),
        Some(4)
    );
    assert_eq!(
        get_background_tasks
            .get("autoDisableRiskyAccountsHealthScoreThreshold")
            .and_then(|value| value.as_u64()),
        Some(55)
    );
    assert_eq!(
        get_background_tasks
            .get("autoDisableRiskyAccountsLookbackMins")
            .and_then(|value| value.as_u64()),
        Some(90)
    );
}

#[test]
fn rpc_account_list_active_filter_uses_backend_filtered_pagination() {
    let ctx = RpcTestContext::new("rpc-account-list-active-filter");
    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    let accounts = [
        ("acc-active-1", "active", 0_i64, Some(20.0)),
        ("acc-active-2", "healthy", 1_i64, Some(30.0)),
        ("acc-low-1", "active", 2_i64, Some(85.0)),
        ("acc-inactive-1", "inactive", 3_i64, Some(10.0)),
        ("acc-no-snapshot", "active", 4_i64, None),
    ];
    for (id, status, sort, used_percent) in accounts {
        storage
            .insert_account(&Account {
                id: id.to_string(),
                label: id.to_string(),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: Some("group-a".to_string()),
                sort,
                status: status.to_string(),
                created_at: now + sort,
                updated_at: now + sort,
            })
            .expect("insert account");
        if let Some(used_percent) = used_percent {
            storage
                .insert_usage_snapshot(&UsageSnapshotRecord {
                    account_id: id.to_string(),
                    used_percent: Some(used_percent),
                    window_minutes: Some(300),
                    resets_at: None,
                    secondary_used_percent: None,
                    secondary_window_minutes: None,
                    secondary_resets_at: None,
                    credits_json: None,
                    captured_at: now + sort,
                })
                .expect("insert usage snapshot");
        }
    }

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let req = JsonRpcRequest {
        id: 30,
        method: "account/list".to_string(),
        params: Some(serde_json::json!({
            "page": 1,
            "pageSize": 2,
            "filter": "active",
            "groupFilter": "group-a"
        })),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    let items = result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("items array");

    assert_eq!(items.len(), 2, "unexpected filtered page size: {result}");
    assert_eq!(
        result.get("total").and_then(|value| value.as_i64()),
        Some(3)
    );
    let ids = items
        .iter()
        .map(|value| {
            value
                .get("id")
                .and_then(|value| value.as_str())
                .expect("item id")
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["acc-active-1", "acc-active-2"]);
}

#[test]
fn rpc_account_delete_many_deletes_requested_accounts() {
    let ctx = RpcTestContext::new("rpc-account-delete-many");
    ctx.seed_accounts(4);
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 11,
        method: "account/deleteMany".to_string(),
        params: Some(serde_json::json!({
            "accountIds": ["acc-1", "acc-3", "missing"]
        })),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");

    assert_eq!(
        result.get("requested").and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        result.get("deleted").and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        result.get("failed").and_then(|value| value.as_u64()),
        Some(1)
    );
    let deleted = result
        .get("deletedAccountIds")
        .and_then(|value| value.as_array())
        .expect("deleted ids");
    assert_eq!(deleted.len(), 2);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let remaining = storage.list_accounts().expect("list remaining");
    let ids = remaining
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["acc-0", "acc-2"]);
}

#[test]
fn rpc_account_update_status_toggles_manual_enable_disable() {
    let ctx = RpcTestContext::new("rpc-account-update-status");
    ctx.seed_accounts(1);

    let disable_server = codexmanager_service::start_one_shot_server().expect("start server");
    let disable_req = JsonRpcRequest {
        id: 12,
        method: "account/update".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-0",
            "status": "disabled"
        })),
    };
    let disable_json = serde_json::to_string(&disable_req).expect("serialize");
    let disable_resp = post_rpc(&disable_server.addr, &disable_json);
    let disable_result = disable_resp.get("result").expect("result");
    assert_eq!(
        disable_result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let disabled = storage
        .find_account_by_id("acc-0")
        .expect("find account")
        .expect("account exists");
    assert_eq!(disabled.status, "disabled");

    let enable_server = codexmanager_service::start_one_shot_server().expect("start server");
    let enable_req = JsonRpcRequest {
        id: 13,
        method: "account/update".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-0",
            "status": "active"
        })),
    };
    let enable_json = serde_json::to_string(&enable_req).expect("serialize");
    let enable_resp = post_rpc(&enable_server.addr, &enable_json);
    let enable_result = enable_resp.get("result").expect("result");
    assert_eq!(
        enable_result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let active = storage
        .find_account_by_id("acc-0")
        .expect("find account")
        .expect("account exists");
    assert_eq!(active.status, "active");
}

#[test]
fn rpc_login_start_returns_url() {
    let _ctx = RpcTestContext::new("rpc-login-start");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 4,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({"type": "chatgpt", "openBrowser": false})),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    let auth_url = result.get("authUrl").and_then(|v| v.as_str()).unwrap();
    let login_id = result.get("loginId").and_then(|v| v.as_str()).unwrap();
    assert!(auth_url.contains("oauth/authorize"));
    assert!(!login_id.is_empty());
}

#[test]
fn rpc_chatgpt_auth_tokens_login_read_logout_roundtrip() {
    let ctx = RpcTestContext::new("rpc-chatgpt-auth-tokens-roundtrip");
    let access_token = build_access_token(
        "sub-external",
        "embedded@example.com",
        "org-embedded",
        "pro",
    );

    let login_req = JsonRpcRequest {
        id: 41,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": access_token,
            "chatgptAccountId": "org-embedded",
            "chatgptPlanType": "pro"
        })),
    };
    let login_json = serde_json::to_string(&login_req).expect("serialize login");
    let login_server = codexmanager_service::start_one_shot_server().expect("start server");
    let login_resp = post_rpc(&login_server.addr, &login_json);
    let login_result = login_resp.get("result").expect("login result");
    let account_id = login_result
        .get("accountId")
        .and_then(|value| value.as_str())
        .expect("account id")
        .to_string();
    assert_eq!(
        login_result.get("type").and_then(|value| value.as_str()),
        Some("chatgptAuthTokens")
    );

    let read_req = JsonRpcRequest {
        id: 42,
        method: "account/read".to_string(),
        params: Some(serde_json::json!({ "refreshToken": false })),
    };
    let read_json = serde_json::to_string(&read_req).expect("serialize read");
    let read_server = codexmanager_service::start_one_shot_server().expect("start server");
    let read_resp = post_rpc(&read_server.addr, &read_json);
    let read_result = read_resp.get("result").expect("read result");
    let account = read_result.get("account").expect("current account");
    assert_eq!(
        read_result.get("authMode").and_then(|value| value.as_str()),
        Some("chatgptAuthTokens")
    );
    assert_eq!(
        account.get("email").and_then(|value| value.as_str()),
        Some("embedded@example.com")
    );
    assert_eq!(
        account.get("planType").and_then(|value| value.as_str()),
        Some("pro")
    );
    assert_eq!(
        account
            .get("chatgptAccountId")
            .and_then(|value| value.as_str()),
        Some("org-embedded")
    );

    let logout_req = JsonRpcRequest {
        id: 43,
        method: "account/logout".to_string(),
        params: None,
    };
    let logout_json = serde_json::to_string(&logout_req).expect("serialize logout");
    let logout_server = codexmanager_service::start_one_shot_server().expect("start server");
    let logout_resp = post_rpc(&logout_server.addr, &logout_json);
    let logout_result = logout_resp.get("result").expect("logout result");
    assert_eq!(
        logout_result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        logout_result
            .get("accountId")
            .and_then(|value| value.as_str()),
        Some(account_id.as_str())
    );

    let read_after_logout_server =
        codexmanager_service::start_one_shot_server().expect("start server");
    let read_after_logout = post_rpc(&read_after_logout_server.addr, &read_json);
    let read_after_logout_result = read_after_logout.get("result").expect("read result");
    assert!(read_after_logout_result.get("account").unwrap().is_null());

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let account = storage
        .find_account_by_id(&account_id)
        .expect("find account")
        .expect("account exists");
    assert_eq!(account.status, "inactive");
}

#[test]
fn rpc_chatgpt_auth_tokens_refresh_updates_access_token() {
    let _ctx = RpcTestContext::new("rpc-chatgpt-auth-tokens-refresh");
    let refreshed_access_token =
        build_access_token("sub-refresh", "refreshed@example.com", "org-refresh", "pro");
    let refresh_response = serde_json::json!({
        "access_token": refreshed_access_token,
        "refresh_token": "refresh-token-new"
    });
    let (issuer, refresh_rx, refresh_join) = start_mock_oauth_token_server(
        200,
        serde_json::to_string(&refresh_response).expect("serialize refresh response"),
    );
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-rpc-refresh");

    let login_req = JsonRpcRequest {
        id: 44,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": build_access_token(
                "sub-refresh",
                "initial@example.com",
                "org-refresh",
                "pro"
            ),
            "refreshToken": "refresh-token-old",
            "chatgptAccountId": "org-refresh"
        })),
    };
    let login_json = serde_json::to_string(&login_req).expect("serialize login");
    let login_server = codexmanager_service::start_one_shot_server().expect("start server");
    let login_resp = post_rpc(&login_server.addr, &login_json);
    let account_id = login_resp
        .get("result")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .expect("account id")
        .to_string();

    let refresh_req = JsonRpcRequest {
        id: 45,
        method: "account/chatgptAuthTokens/refresh".to_string(),
        params: Some(serde_json::json!({
            "reason": "unauthorized",
            "previousAccountId": "org-refresh"
        })),
    };
    let refresh_json = serde_json::to_string(&refresh_req).expect("serialize refresh");
    let refresh_server = codexmanager_service::start_one_shot_server().expect("start server");
    let refresh_rpc_resp = post_rpc(&refresh_server.addr, &refresh_json);
    let refresh_result = refresh_rpc_resp.get("result").expect("refresh result");
    assert_eq!(
        refresh_result
            .get("chatgptAccountId")
            .and_then(|value| value.as_str()),
        Some("org-refresh")
    );
    assert_eq!(
        refresh_result
            .get("accessToken")
            .and_then(|value| value.as_str()),
        Some(refreshed_access_token.as_str())
    );

    let refresh_body = refresh_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive refresh request");
    refresh_join.join().expect("join mock oauth server");
    assert!(refresh_body.contains("grant_type=refresh_token"));
    assert!(refresh_body.contains("refresh_token=refresh-token-old"));

    let storage =
        Storage::open(std::env::var("CODEXMANAGER_DB_PATH").expect("db path")).expect("open db");
    let token = storage
        .find_token_by_account_id(&account_id)
        .expect("find token")
        .expect("token exists");
    assert_eq!(token.access_token, refreshed_access_token);
    assert_eq!(token.refresh_token, "refresh-token-new");
}

#[test]
fn rpc_account_read_refresh_token_uses_session_cookie_fallback_for_chatgpt_auth_tokens() {
    let ctx = RpcTestContext::new("rpc-account-read-refresh-session-cookie");
    let email = "read-refresh@example.com";
    let account_identity = "workspace-read-refresh";
    let expired_access_token =
        build_access_token("subject-read-old", email, account_identity, "free");
    let refreshed_access_token =
        build_access_token("subject-read-new", email, account_identity, "plus");
    let (issuer, _refresh_rx, oauth_join) =
        start_mock_oauth_token_server(401, r#"{"error":"refresh_token_reused"}"#.to_string());
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard =
        EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-read-session-cookie");
    let (session_url, session_rx, session_join) = start_mock_session_refresh_server(
        serde_json::json!({
            "accessToken": refreshed_access_token,
            "expires": "2026-04-01T18:00:00Z"
        })
        .to_string(),
    );
    let _session_url_guard = EnvGuard::set("CODEX_SESSION_REFRESH_URL_OVERRIDE", &session_url);

    let login_req = JsonRpcRequest {
        id: 445,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": expired_access_token,
            "refreshToken": "refresh-token-reused",
            "idToken": "",
            "cookies": "__Secure-next-auth.session-token=session-read-refresh; oai-did=device-read-123",
            "email": email,
            "chatgptAccountId": account_identity,
            "workspaceId": account_identity,
        })),
    };
    let login_json = serde_json::to_string(&login_req).expect("serialize login");
    let login_server = codexmanager_service::start_one_shot_server().expect("start server");
    let login_resp = post_rpc(&login_server.addr, &login_json);
    let account_id = login_resp
        .get("result")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .expect("account id")
        .to_string();

    let read_req = JsonRpcRequest {
        id: 446,
        method: "account/read".to_string(),
        params: Some(serde_json::json!({ "refreshToken": true })),
    };
    let read_json = serde_json::to_string(&read_req).expect("serialize read");
    let read_server = codexmanager_service::start_one_shot_server().expect("start server");
    let read_resp = post_rpc(&read_server.addr, &read_json);
    let read_result = read_resp.get("result").expect("read result");
    let account = read_result.get("account").expect("current account");
    assert_eq!(
        read_result.get("authMode").and_then(|value| value.as_str()),
        Some("chatgptAuthTokens"),
        "read response: {read_resp}"
    );
    assert_eq!(
        account.get("email").and_then(|value| value.as_str()),
        Some(email),
        "read response: {read_resp}"
    );
    assert_eq!(
        account.get("planType").and_then(|value| value.as_str()),
        Some("plus"),
        "read response: {read_resp}"
    );

    oauth_join.join().expect("join oauth server");
    let cookie_header = session_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive session refresh cookie");
    session_join.join().expect("join session server");
    assert!(
        cookie_header.contains("__Secure-next-auth.session-token=session-read-refresh"),
        "unexpected cookie header: {cookie_header}"
    );

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let updated_token = storage
        .find_token_by_account_id(&account_id)
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-reused");
}

#[test]
fn rpc_account_auth_recover_silently_refreshes_and_reactivates_account() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-silent");
    let refreshed_access_token =
        build_access_token("sub-recover", "recovered@example.com", "org-recover", "pro");
    let refresh_response = serde_json::json!({
        "access_token": refreshed_access_token,
        "refresh_token": "refresh-token-recovered"
    });
    let (issuer, refresh_rx, refresh_join) = start_mock_oauth_token_server(
        200,
        serde_json::to_string(&refresh_response).expect("serialize refresh response"),
    );
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-auth-recover");

    let login_req = JsonRpcRequest {
        id: 46,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": build_access_token(
                "sub-recover",
                "initial-recover@example.com",
                "org-recover",
                "pro"
            ),
            "refreshToken": "refresh-token-old",
            "chatgptAccountId": "org-recover"
        })),
    };
    let login_json = serde_json::to_string(&login_req).expect("serialize login");
    let login_server = codexmanager_service::start_one_shot_server().expect("start server");
    let login_resp = post_rpc(&login_server.addr, &login_json);
    let account_id = login_resp
        .get("result")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .expect("account id")
        .to_string();

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage
        .update_account_status(&account_id, "unavailable")
        .expect("mark unavailable");

    let recover_req = JsonRpcRequest {
        id: 47,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": account_id
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server = codexmanager_service::start_one_shot_server().expect("start server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered"),
        "recover response: {recover_resp}"
    );
    assert_eq!(
        recover_result
            .get("accountId")
            .and_then(|value| value.as_str()),
        Some(account_id.as_str())
    );

    let refresh_body = refresh_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive refresh request");
    refresh_join.join().expect("join mock oauth server");
    assert!(refresh_body.contains("grant_type=refresh_token"));
    assert!(refresh_body.contains("refresh_token=refresh-token-old"));

    let updated_account = storage
        .find_account_by_id(&account_id)
        .expect("find account")
        .expect("account exists");
    assert_eq!(updated_account.status, "active");

    let updated_token = storage
        .find_token_by_account_id(&account_id)
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-recovered");
}

#[test]
fn rpc_account_auth_recover_refreshes_target_account_instead_of_current_auth_account() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-target-account");
    let refreshed_access_token = build_access_token(
        "sub-target-recover",
        "target-recover@example.com",
        "org-target-recover",
        "plus",
    );
    let refresh_response = serde_json::json!({
        "access_token": refreshed_access_token,
        "refresh_token": "refresh-token-target-new"
    });
    let (issuer, refresh_rx, refresh_join) = start_mock_oauth_token_server(
        200,
        serde_json::to_string(&refresh_response).expect("serialize refresh response"),
    );
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-target-recover");

    let current_login_req = JsonRpcRequest {
        id: 471,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": build_access_token(
                "sub-current-auth",
                "current-auth@example.com",
                "org-current-auth",
                "pro"
            ),
            "refreshToken": "refresh-token-current-old",
            "chatgptAccountId": "org-current-auth"
        })),
    };
    let current_login_json =
        serde_json::to_string(&current_login_req).expect("serialize current login");
    let current_login_server =
        codexmanager_service::start_one_shot_server().expect("start current login server");
    let current_login_resp = post_rpc(&current_login_server.addr, &current_login_json);
    let current_account_id = current_login_resp
        .get("result")
        .and_then(|value| value.get("accountId"))
        .and_then(|value| value.as_str())
        .expect("current account id")
        .to_string();

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-target-recover".to_string(),
            label: "target-recover@example.com".to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some("org-target-recover".to_string()),
            workspace_id: Some("workspace-target-recover".to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 1,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert target account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-target-recover".to_string(),
            id_token: build_access_token(
                "sub-target-recover",
                "target-recover@example.com",
                "org-target-recover",
                "free",
            ),
            access_token: build_access_token(
                "sub-target-recover",
                "target-recover@example.com",
                "org-target-recover",
                "free",
            ),
            refresh_token: "refresh-token-target-old".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert target token");

    let recover_req = JsonRpcRequest {
        id: 472,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-target-recover",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered"),
        "recover response: {recover_resp}"
    );
    assert_eq!(
        recover_result
            .get("accountId")
            .and_then(|value| value.as_str()),
        Some("acc-target-recover")
    );

    let refresh_body = refresh_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive refresh request");
    refresh_join.join().expect("join mock oauth server");
    assert!(
        refresh_body.contains("refresh_token=refresh-token-target-old"),
        "unexpected refresh body: {refresh_body}"
    );

    let target_token = storage
        .find_token_by_account_id("acc-target-recover")
        .expect("find target token")
        .expect("target token exists");
    assert_eq!(target_token.access_token, refreshed_access_token);
    assert_eq!(target_token.refresh_token, "refresh-token-target-new");

    let current_token = storage
        .find_token_by_account_id(&current_account_id)
        .expect("find current token")
        .expect("current token exists");
    assert_eq!(current_token.refresh_token, "refresh-token-current-old");
}

#[test]
fn rpc_account_auth_recover_returns_error_when_no_noninteractive_recovery_available() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-login");
    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-login".to_string(),
            label: "Recover Login".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt-recover-login".to_string()),
            workspace_id: Some("workspace-recover-login".to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");

    let recover_req = JsonRpcRequest {
        id: 48,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-login",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server = codexmanager_service::start_one_shot_server().expect("start server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        None
    );
    let error_message = recover_result
        .get("error")
        .and_then(|value| value.as_str())
        .expect("error message");
    assert!(
        error_message.contains("missing recoverable email")
            || error_message.contains("register service account not found"),
        "unexpected error: {error_message}"
    );
}

#[test]
fn rpc_account_auth_recover_falls_back_to_register_service_refresh() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-register-fallback");
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-id-register-fallback");
    let email = "recover-register@example.com";
    let expired_access_token = build_access_token(
        "subject-register-old",
        email,
        "chatgpt-register-fallback",
        "free",
    );
    let refreshed_access_token = build_access_token(
        "subject-register-new",
        email,
        "chatgpt-register-fallback",
        "plus",
    );
    let refreshed_id_token = build_access_token(
        "subject-register-new",
        email,
        "chatgpt-register-fallback",
        "plus",
    );
    let (issuer, _refresh_rx, oauth_join) = start_mock_oauth_token_server(
        401,
        r#"{"error":"invalid_grant","error_description":"refresh token expired"}"#.to_string(),
    );
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let (register_url, register_rx, register_join) = start_mock_register_service_server(
        email.to_string(),
        refreshed_access_token.clone(),
        "refresh-token-register-new".to_string(),
        refreshed_id_token.clone(),
        "cf_clearance=register-cookie; oai-did=device-id".to_string(),
    );
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-register".to_string(),
            label: email.to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some("chatgpt-register-fallback".to_string()),
            workspace_id: Some("workspace-register-fallback".to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-recover-register".to_string(),
            id_token: expired_access_token.clone(),
            access_token: expired_access_token,
            refresh_token: "refresh-token-expired".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let recover_req = JsonRpcRequest {
        id: 49,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-register",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server = codexmanager_service::start_one_shot_server().expect("start server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );
    assert_eq!(
        recover_result
            .get("loginId")
            .and_then(|value| value.as_str()),
        None
    );
    assert_eq!(
        recover_result
            .get("authUrl")
            .and_then(|value| value.as_str()),
        None
    );

    oauth_join.join().expect("join oauth server");
    let register_requests = (0..3)
        .map(|_| {
            register_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive register request")
        })
        .collect::<Vec<_>>();
    register_join.join().expect("join register server");
    assert_eq!(
        register_requests,
        vec![
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=recover-register%40example.com"
                    .to_string(),
            ),
            ("POST".to_string(), "/api/accounts/123/refresh".to_string()),
            ("GET".to_string(), "/api/accounts/123/tokens".to_string()),
        ]
    );

    let updated_account = storage
        .find_account_by_id("acc-recover-register")
        .expect("find account")
        .expect("account exists");
    assert_eq!(updated_account.status, "active");

    let updated_token = storage
        .find_token_by_account_id("acc-recover-register")
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-register-new");
    assert_eq!(updated_token.id_token, refreshed_id_token);
}

#[test]
fn rpc_account_auth_recover_falls_back_to_register_account_id_when_email_misses() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-register-account-id");
    let local_email = "guavrybt52g@wwjjff.qzz.io";
    let remote_email = "recover-register@example.com";
    let account_identity = "chatgpt-register-fallback";
    let expired_access_token = build_access_token(
        "subject-register-old",
        local_email,
        account_identity,
        "free",
    );
    let refreshed_access_token = build_access_token(
        "subject-register-new",
        remote_email,
        account_identity,
        "plus",
    );
    let refreshed_id_token = build_access_token(
        "subject-register-new",
        remote_email,
        account_identity,
        "plus",
    );
    let (issuer, _refresh_rx, oauth_join) = start_mock_oauth_token_server(
        401,
        r#"{"error":"invalid_grant","error_description":"refresh token expired"}"#.to_string(),
    );
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let (register_url, register_rx, register_join) =
        start_mock_register_service_server_email_miss_account_match(
            local_email.to_string(),
            account_identity.to_string(),
            remote_email.to_string(),
            refreshed_access_token.clone(),
            "refresh-token-register-new".to_string(),
            refreshed_id_token.clone(),
            "cf_clearance=register-cookie; oai-did=device-id".to_string(),
        );
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-register-account-id".to_string(),
            label: local_email.to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some(account_identity.to_string()),
            workspace_id: Some("workspace-register-fallback".to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-recover-register-account-id".to_string(),
            id_token: expired_access_token.clone(),
            access_token: expired_access_token,
            refresh_token: "refresh-token-expired".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let recover_req = JsonRpcRequest {
        id: 50,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-register-account-id",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server = codexmanager_service::start_one_shot_server().expect("start server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );

    oauth_join.join().expect("join oauth server");
    let register_requests = (0..4)
        .map(|_| {
            register_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive register request")
        })
        .collect::<Vec<_>>();
    register_join.join().expect("join register server");
    assert_eq!(
        register_requests,
        vec![
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=guavrybt52g%40wwjjff.qzz.io".to_string(),
            ),
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=chatgpt-register-fallback".to_string(),
            ),
            ("POST".to_string(), "/api/accounts/321/refresh".to_string()),
            ("GET".to_string(), "/api/accounts/321/tokens".to_string()),
        ]
    );

    let updated_token = storage
        .find_token_by_account_id("acc-recover-register-account-id")
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-register-new");
}

#[test]
fn rpc_account_auth_recover_falls_back_to_session_cookies_when_refresh_token_reused() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-session-cookie");
    let email = "recover-session@example.com";
    let account_identity = "workspace-session-recover";
    let expired_access_token =
        build_access_token("subject-session-old", email, account_identity, "free");
    let refreshed_access_token =
        build_access_token("subject-session-new", email, account_identity, "plus");
    let (issuer, _refresh_rx, oauth_join) =
        start_mock_oauth_token_server(401, r#"{"error":"refresh_token_reused"}"#.to_string());
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-session-cookie");
    let (session_url, session_rx, session_join) = start_mock_session_refresh_server(
        serde_json::json!({
            "accessToken": refreshed_access_token,
            "expires": "2026-03-26T16:00:00Z"
        })
        .to_string(),
    );
    let _session_url_guard = EnvGuard::set("CODEX_SESSION_REFRESH_URL_OVERRIDE", &session_url);

    let login_server = codexmanager_service::start_one_shot_server().expect("start login server");
    let login_req = JsonRpcRequest {
        id: 60,
        method: "account/login/start".to_string(),
        params: Some(serde_json::json!({
            "type": "chatgptAuthTokens",
            "accessToken": expired_access_token,
            "refreshToken": "refresh-token-reused",
            "idToken": "",
            "cookies": "__Secure-next-auth.session-token=session-recover; oai-did=device-123",
            "email": email,
            "chatgptAccountId": account_identity,
            "workspaceId": account_identity,
        })),
    };
    let login_json = serde_json::to_string(&login_req).expect("serialize login");
    let login_resp = post_rpc(&login_server.addr, &login_json);
    let login_result = login_resp.get("result").expect("login result");
    let account_id = login_result
        .get("accountId")
        .and_then(|value| value.as_str())
        .expect("account id");

    let recover_req = JsonRpcRequest {
        id: 61,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": account_id,
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );

    oauth_join.join().expect("join oauth server");
    let cookie_header = session_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive session refresh cookie");
    session_join.join().expect("join session server");
    assert!(
        cookie_header.contains("__Secure-next-auth.session-token=session-recover"),
        "unexpected cookie header: {cookie_header}"
    );

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let updated_token = storage
        .find_token_by_account_id(account_id)
        .expect("find token")
        .expect("token exists");
    assert_eq!(
        updated_token.access_token,
        build_access_token("subject-session-new", email, account_identity, "plus")
    );
    assert_eq!(updated_token.refresh_token, "refresh-token-reused");
}

#[test]
fn rpc_register_import_by_email_uses_remote_session_token_as_cookie_fallback() {
    let ctx = RpcTestContext::new("rpc-register-import-session-token-fallback");
    let email = "import-session@example.com";
    let account_identity = "workspace-session-import";
    let imported_access_token =
        build_access_token("subject-session-import-old", email, account_identity, "free");
    let refreshed_access_token =
        build_access_token("subject-session-import-new", email, account_identity, "plus");
    let remote_session_token = "remote-session-token-777";

    let (register_url, register_rx, register_join) =
        start_mock_register_service_server_with_session_token_only(
            email.to_string(),
            imported_access_token.clone(),
            remote_session_token.to_string(),
        );
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);

    let import_req = JsonRpcRequest {
        id: 62,
        method: "account/register/importByEmail".to_string(),
        params: Some(serde_json::json!({
            "email": email
        })),
    };
    let import_json = serde_json::to_string(&import_req).expect("serialize import request");
    let import_server = codexmanager_service::start_one_shot_server().expect("start import server");
    let import_resp = post_rpc(&import_server.addr, &import_json);
    let import_result = import_resp.get("result").expect("import result");
    let account_id = import_result
        .get("accountId")
        .and_then(|value| value.as_str())
        .expect("imported account id")
        .to_string();

    let (session_url, session_rx, session_join) = start_mock_session_refresh_server(
        serde_json::json!({
            "accessToken": refreshed_access_token,
            "expires": "2026-04-01T18:00:00Z"
        })
        .to_string(),
    );
    let _session_url_guard = EnvGuard::set("CODEX_SESSION_REFRESH_URL_OVERRIDE", &session_url);

    let recover_req = JsonRpcRequest {
        id: 63,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": account_id,
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result.get("status").and_then(|value| value.as_str()),
        Some("recovered")
    );

    let register_requests = (0..2)
        .map(|_| {
            register_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive register request")
        })
        .collect::<Vec<_>>();
    register_join.join().expect("join register server");
    assert_eq!(
        register_requests,
        vec![
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=import-session%40example.com"
                    .to_string(),
            ),
            ("GET".to_string(), "/api/accounts/777/tokens".to_string()),
        ]
    );

    let cookie_header = session_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive session refresh cookie");
    session_join.join().expect("join session server");
    assert!(
        cookie_header.contains("__Secure-next-auth.session-token=remote-session-token-777"),
        "unexpected cookie header: {cookie_header}"
    );

    let storage = Storage::open(ctx.db_path()).expect("open db");
    let updated_token = storage
        .find_token_by_account_id(&account_id)
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "");
}

#[test]
fn rpc_account_auth_recover_uses_remote_manager_when_configured() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-remote-manager");
    let email = "recover-remote@example.com";
    let chatgpt_account_id = "chatgpt-remote-recovery";
    let workspace_id = "workspace-remote-recovery";
    let expired_access_token =
        build_access_token("subject-remote-old", email, chatgpt_account_id, "free");
    let refreshed_access_token =
        build_access_token("subject-remote-new", email, chatgpt_account_id, "plus");
    let refreshed_id_token =
        build_access_token("subject-remote-new", email, chatgpt_account_id, "plus");
    let (issuer, _refresh_rx, oauth_join) =
        start_mock_oauth_token_server(401, r#"{"error":"refresh_token_reused"}"#.to_string());
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-remote-manager");
    let (register_url, register_rx, register_join) = start_mock_register_service_probe_server();
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);
    let (recovery_url, recovery_rx, recovery_join) = start_mock_recovery_manager_server(
        email.to_string(),
        "remote-account-1".to_string(),
        chatgpt_account_id.to_string(),
        workspace_id.to_string(),
        refreshed_access_token.clone(),
        "refresh-token-remote-new".to_string(),
        refreshed_id_token.clone(),
    );
    let _recovery_guard = EnvGuard::set("CODEXMANAGER_ACCOUNT_RECOVERY_SOURCE_URL", &recovery_url);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-remote-manager".to_string(),
            label: email.to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some(chatgpt_account_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-recover-remote-manager".to_string(),
            id_token: expired_access_token.clone(),
            access_token: expired_access_token,
            refresh_token: "refresh-token-reused".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let recover_req = JsonRpcRequest {
        id: 62,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-remote-manager",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );

    oauth_join.join().expect("join oauth server");
    register_join.join().expect("join register miss server");
    assert!(
        register_rx
            .recv_timeout(Duration::from_millis(200))
            .is_err(),
        "register service should not be used when remote recovery source is configured"
    );

    let recovery_requests = (0..2)
        .map(|_| {
            recovery_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive recovery manager request")
        })
        .collect::<Vec<_>>();
    recovery_join.join().expect("join recovery manager server");
    assert_eq!(recovery_requests[0].0, "account/list");
    assert_eq!(recovery_requests[1].0, "account/exportData");
    assert_eq!(
        recovery_requests[1]
            .1
            .get("params")
            .and_then(|value| value.get("accountIds"))
            .and_then(|value| value.as_array())
            .expect("accountIds"),
        &vec![serde_json::json!("remote-account-1")]
    );

    let updated_account = storage
        .find_account_by_id("acc-recover-remote-manager")
        .expect("find account")
        .expect("account exists");
    assert_eq!(updated_account.status, "active");

    let updated_token = storage
        .find_token_by_account_id("acc-recover-remote-manager")
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-remote-new");
    assert_eq!(updated_token.id_token, refreshed_id_token);
}

#[test]
fn rpc_account_auth_recover_prefers_remote_manager_when_configured() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-prefer-remote-manager");
    let email = "recover-preferred@example.com";
    let chatgpt_account_id = "chatgpt-remote-preferred";
    let workspace_id = "workspace-remote-preferred";
    let expired_access_token =
        build_access_token("subject-remote-old", email, chatgpt_account_id, "free");
    let refreshed_access_token =
        build_access_token("subject-remote-new", email, chatgpt_account_id, "plus");
    let refreshed_id_token =
        build_access_token("subject-remote-new", email, chatgpt_account_id, "plus");
    let (issuer, _refresh_rx, oauth_join) =
        start_mock_oauth_token_server(401, r#"{"error":"refresh_token_reused"}"#.to_string());
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard = EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-remote-preferred");
    let (register_url, register_rx, register_join) = start_mock_register_service_probe_server();
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);
    let (recovery_url, recovery_rx, recovery_join) = start_mock_recovery_manager_server(
        email.to_string(),
        "remote-account-9".to_string(),
        chatgpt_account_id.to_string(),
        workspace_id.to_string(),
        refreshed_access_token.clone(),
        "refresh-token-remote-new".to_string(),
        refreshed_id_token.clone(),
    );
    let _recovery_guard = EnvGuard::set("CODEXMANAGER_ACCOUNT_RECOVERY_SOURCE_URL", &recovery_url);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-remote-preferred".to_string(),
            label: email.to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some(chatgpt_account_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-recover-remote-preferred".to_string(),
            id_token: expired_access_token.clone(),
            access_token: expired_access_token,
            refresh_token: "refresh-token-reused".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let recover_req = JsonRpcRequest {
        id: 63,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-remote-preferred",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );

    oauth_join.join().expect("join oauth server");
    recovery_join.join().expect("join recovery manager server");
    let recovery_requests = (0..2)
        .map(|_| {
            recovery_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive recovery manager request")
        })
        .collect::<Vec<_>>();
    assert_eq!(recovery_requests[0].0, "account/list");
    assert_eq!(recovery_requests[1].0, "account/exportData");

    register_join.join().expect("join register server");
    assert!(
        register_rx
            .recv_timeout(Duration::from_millis(200))
            .is_err(),
        "register service should not be used when remote recovery source is configured"
    );

    let updated_token = storage
        .find_token_by_account_id("acc-recover-remote-preferred")
        .expect("find token")
        .expect("token exists");
    assert_eq!(updated_token.access_token, refreshed_access_token);
    assert_eq!(updated_token.refresh_token, "refresh-token-remote-new");
    assert_eq!(updated_token.id_token, refreshed_id_token);
}

#[test]
fn rpc_account_auth_recover_auto_registers_new_account_when_all_existing_recovery_fails() {
    let ctx = RpcTestContext::new("rpc-account-auth-recover-auto-register-login");
    let old_email = "recover-old@example.com";
    let old_chatgpt_account_id = "chatgpt-old-recovery";
    let old_workspace_id = "workspace-old-recovery";
    let new_email = "recover-new@example.com";
    let new_chatgpt_account_id = "chatgpt-new-recovery";
    let new_workspace_id = "workspace-new-recovery";
    let expired_access_token =
        build_access_token("subject-old", old_email, old_chatgpt_account_id, "free");
    let refreshed_access_token =
        build_access_token("subject-new", new_email, new_chatgpt_account_id, "plus");
    let refreshed_id_token =
        build_access_token("subject-new", new_email, new_chatgpt_account_id, "plus");
    let (issuer, _refresh_rx, oauth_join) =
        start_mock_oauth_token_server(401, r#"{"error":"refresh_token_reused"}"#.to_string());
    let _issuer_guard = EnvGuard::set("CODEXMANAGER_ISSUER", &issuer);
    let _client_id_guard =
        EnvGuard::set("CODEXMANAGER_CLIENT_ID", "client-test-auto-register-login");
    let (register_url, register_rx, register_body_rx, register_join) =
        start_mock_register_service_server_auto_register_login(
            old_email.to_string(),
            old_chatgpt_account_id.to_string(),
            old_workspace_id.to_string(),
            new_email.to_string(),
            new_chatgpt_account_id.to_string(),
            new_workspace_id.to_string(),
            refreshed_access_token.clone(),
            "refresh-token-new".to_string(),
            refreshed_id_token.clone(),
        );
    let _register_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);

    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-recover-auto-register".to_string(),
            label: old_email.to_string(),
            issuer: issuer.clone(),
            chatgpt_account_id: Some(old_chatgpt_account_id.to_string()),
            workspace_id: Some(old_workspace_id.to_string()),
            group_name: Some("TEAM".to_string()),
            sort: 0,
            status: "unavailable".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&codexmanager_core::storage::Token {
            account_id: "acc-recover-auto-register".to_string(),
            id_token: expired_access_token.clone(),
            access_token: expired_access_token,
            refresh_token: "refresh-token-reused".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");

    let recover_req = JsonRpcRequest {
        id: 64,
        method: "account/auth/recover".to_string(),
        params: Some(serde_json::json!({
            "accountId": "acc-recover-auto-register",
            "openBrowser": false
        })),
    };
    let recover_json = serde_json::to_string(&recover_req).expect("serialize recover");
    let recover_server =
        codexmanager_service::start_one_shot_server().expect("start recover server");
    let recover_resp = post_rpc(&recover_server.addr, &recover_json);
    let recover_result = recover_resp.get("result").expect("recover result");
    assert_eq!(
        recover_result
            .get("status")
            .and_then(|value| value.as_str()),
        Some("recovered")
    );
    let recovered_account_id = recover_result
        .get("accountId")
        .and_then(|value| value.as_str())
        .expect("recovered account id");
    assert_ne!(recovered_account_id, "acc-recover-auto-register");

    oauth_join.join().expect("join oauth server");
    let register_batch_body = register_body_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive auto register batch body");
    let register_requests = (0..11)
        .map(|_| {
            register_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("receive auto register request")
        })
        .collect::<Vec<_>>();
    register_join.join().expect("join auto register server");
    assert_eq!(
        register_requests,
        vec![
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=recover-old%40example.com".to_string(),
            ),
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=chatgpt-old-recovery".to_string(),
            ),
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=workspace-old-recovery".to_string(),
            ),
            (
                "GET".to_string(),
                "/api/registration/available-services".to_string()
            ),
            ("POST".to_string(), "/api/registration/batch".to_string()),
            (
                "GET".to_string(),
                "/api/registration/tasks/task-auto-1".to_string()
            ),
            (
                "GET".to_string(),
                "/api/registration/tasks/task-auto-1/logs".to_string()
            ),
            (
                "GET".to_string(),
                "/api/registration/tasks/task-auto-1".to_string()
            ),
            (
                "GET".to_string(),
                "/api/registration/tasks/task-auto-1/logs".to_string()
            ),
            (
                "GET".to_string(),
                "/api/accounts?page=1&page_size=20&search=recover-new%40example.com".to_string(),
            ),
            ("GET".to_string(), "/api/accounts/900/tokens".to_string()),
        ]
    );
    assert!(
        register_batch_body.contains("\"register_mode\":\"any_auto\""),
        "unexpected register batch body: {register_batch_body}"
    );
    assert!(
        register_batch_body.contains("\"email_service_type\":\"custom_domain\""),
        "unexpected register batch body: {register_batch_body}"
    );

    let recovered_token = storage
        .find_token_by_account_id(recovered_account_id)
        .expect("find recovered token")
        .expect("recovered token exists");
    assert_eq!(recovered_token.access_token, refreshed_access_token);
    assert_eq!(recovered_token.refresh_token, "refresh-token-new");
    assert_eq!(recovered_token.id_token, refreshed_id_token);

    let old_account = storage
        .find_account_by_id("acc-recover-auto-register")
        .expect("find old account")
        .expect("old account exists");
    assert_eq!(old_account.status, "unavailable");
}

#[test]
fn rpc_usage_read_empty() {
    let _ctx = RpcTestContext::new("rpc-usage-read");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 5,
        method: "account/usage/read".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    assert!(result.get("snapshot").is_some());
}

#[test]
fn rpc_login_status_pending() {
    let _ctx = RpcTestContext::new("rpc-login-status");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 6,
        method: "account/login/status".to_string(),
        params: Some(serde_json::json!({"loginId": "login-1"})),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    assert!(result.get("status").is_some());
}

#[test]
fn rpc_usage_list_empty() {
    let _ctx = RpcTestContext::new("rpc-usage-list");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 7,
        method: "account/usage/list".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    let items = result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("items array");
    assert!(
        items.is_empty(),
        "expected empty usage items, got: {result}"
    );
}

#[test]
fn rpc_usage_aggregate_returns_backend_summary() {
    let ctx = RpcTestContext::new("rpc-usage-aggregate");
    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");
    let now = now_ts();

    storage
        .insert_account(&Account {
            id: "acc-pro".to_string(),
            label: "Pro".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert pro account");
    storage
        .insert_account(&Account {
            id: "acc-free".to_string(),
            label: "Free".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 1,
            status: "active".to_string(),
            created_at: now + 1,
            updated_at: now + 1,
        })
        .expect("insert free account");

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-pro".to_string(),
            used_percent: Some(10.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: Some(40.0),
            secondary_window_minutes: Some(10080),
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert pro usage");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-free".to_string(),
            used_percent: Some(20.0),
            window_minutes: Some(10080),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: Some(r#"{"planType":"free"}"#.to_string()),
            captured_at: now,
        })
        .expect("insert free usage");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let req = JsonRpcRequest {
        id: 71,
        method: "account/usage/aggregate".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");

    assert_eq!(
        result
            .get("primaryBucketCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        result
            .get("primaryRemainPercent")
            .and_then(|value| value.as_i64()),
        Some(90)
    );
    assert_eq!(
        result
            .get("secondaryBucketCount")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        result
            .get("secondaryRemainPercent")
            .and_then(|value| value.as_i64()),
        Some(70)
    );
}

#[test]
fn rpc_requestlog_list_and_summary_support_pagination() {
    let ctx = RpcTestContext::new("rpc-requestlog-page");
    let storage = Storage::open(ctx.db_path()).expect("open db");
    storage.init().expect("init schema");

    for index in 0..4_i64 {
        let created_at = now_ts() + index;
        let status_code = if index < 2 { Some(200) } else { Some(502) };
        let request_log_id = storage
            .insert_request_log(&RequestLog {
                trace_id: Some(format!("trc-page-{index}")),
                key_id: Some("gk-page".to_string()),
                account_id: Some("acc-page".to_string()),
                initial_account_id: Some("acc-free".to_string()),
                attempted_account_ids_json: Some(r#"["acc-free","acc-page"]"#.to_string()),
                route_strategy: Some("least-latency".to_string()),
                requested_model: None,
                model_fallback_path_json: None,
                request_path: "/v1/responses".to_string(),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some("gpt-5".to_string()),
                reasoning_effort: Some("medium".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
                status_code,
                duration_ms: Some(500 + index),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: if status_code == Some(502) {
                    Some("stream interrupted".to_string())
                } else {
                    None
                },
                created_at,
            })
            .expect("insert request log");
        storage
            .insert_request_token_stat(&RequestTokenStat {
                request_log_id,
                key_id: Some("gk-page".to_string()),
                account_id: Some("acc-page".to_string()),
                model: Some("gpt-5".to_string()),
                input_tokens: Some(10),
                cached_input_tokens: Some(1),
                output_tokens: Some(2),
                total_tokens: Some(20 + index),
                reasoning_output_tokens: Some(0),
                estimated_cost_usd: Some(0.01),
                created_at,
            })
            .expect("insert token stat");
    }

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let list_req = JsonRpcRequest {
        id: 72,
        method: "requestlog/list".to_string(),
        params: Some(serde_json::json!({
            "page": 2,
            "pageSize": 1,
            "statusFilter": "5xx"
        })),
    };
    let list_json = serde_json::to_string(&list_req).expect("serialize requestlog list");
    let list_resp = post_rpc(&server.addr, &list_json);
    let list_result = list_resp.get("result").expect("requestlog list result");
    assert_eq!(
        list_result.get("total").and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        list_result.get("page").and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        list_result.get("pageSize").and_then(|value| value.as_i64()),
        Some(1)
    );
    let items = list_result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("requestlog items");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("traceId").and_then(|value| value.as_str()),
        Some("trc-page-2")
    );
    assert_eq!(
        items[0]
            .get("initialAccountId")
            .and_then(|value| value.as_str()),
        Some("acc-free")
    );
    assert_eq!(
        items[0]
            .get("attemptedAccountIds")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(
        items[0]
            .get("routeStrategy")
            .and_then(|value| value.as_str()),
        Some("least-latency")
    );

    let summary_server = codexmanager_service::start_one_shot_server().expect("start server");
    let summary_req = JsonRpcRequest {
        id: 73,
        method: "requestlog/summary".to_string(),
        params: Some(serde_json::json!({
            "statusFilter": "5xx"
        })),
    };
    let summary_json = serde_json::to_string(&summary_req).expect("serialize requestlog summary");
    let summary_resp = post_rpc(&summary_server.addr, &summary_json);
    let summary_result = summary_resp
        .get("result")
        .expect("requestlog summary result");
    assert_eq!(
        summary_result
            .get("totalCount")
            .and_then(|value| value.as_i64()),
        Some(4)
    );
    assert_eq!(
        summary_result
            .get("filteredCount")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        summary_result
            .get("errorCount")
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        summary_result
            .get("totalTokens")
            .and_then(|value| value.as_i64()),
        Some(45)
    );
}

#[test]
fn rpc_rejects_missing_token() {
    let _ctx = RpcTestContext::new("rpc-missing-token");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 8,
        method: "initialize".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let (status, _) = post_rpc_raw(&server.addr, &json, &[("Content-Type", "application/json")]);
    assert_eq!(status, 401);
}

#[test]
fn rpc_rejects_cross_site_origin() {
    let _ctx = RpcTestContext::new("rpc-cross-site-origin");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 9,
        method: "initialize".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let token = codexmanager_service::rpc_auth_token().to_string();
    let (status, _) = post_rpc_raw(
        &server.addr,
        &json,
        &[
            ("Content-Type", "application/json"),
            ("X-CodexManager-Rpc-Token", token.as_str()),
            ("Origin", "https://evil.example"),
            ("Sec-Fetch-Site", "cross-site"),
        ],
    );
    assert_eq!(status, 403);
}

#[test]
fn rpc_accepts_loopback_origin() {
    let _ctx = RpcTestContext::new("rpc-loopback-origin");
    let server = codexmanager_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 10,
        method: "initialize".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let token = codexmanager_service::rpc_auth_token().to_string();
    let (status, body) = post_rpc_raw(
        &server.addr,
        &json,
        &[
            ("Content-Type", "application/json"),
            ("X-CodexManager-Rpc-Token", token.as_str()),
            ("Origin", "http://localhost:5173"),
            ("Sec-Fetch-Site", "same-site"),
        ],
    );
    assert_eq!(status, 200, "unexpected status {status}: {body}");
}
