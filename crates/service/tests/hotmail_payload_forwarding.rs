use codexmanager_core::rpc::types::JsonRpcRequest;
use codexmanager_service::start_one_shot_server;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server, StatusCode};

static REGISTER_ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard(&'static str, Option<PathBuf>);

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key).map(PathBuf::from);
        std::env::set_var(key, value);
        EnvGuard(key, original)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.1 {
            std::env::set_var(self.0, value);
        } else {
            std::env::remove_var(self.0);
        }
    }
}

fn post_rpc(addr: &str, body: &str) -> Value {
    let mut stream = TcpStream::connect(addr).expect("connect rpc server");
    let token = codexmanager_service::rpc_auth_token().to_string();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-CodexManager-Rpc-Token: {token}\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .expect("write rpc request");
    stream.shutdown(std::net::Shutdown::Write).ok();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).expect("read rpc response");
    let payload = String::from_utf8(buf).expect("utf8 rpc response");
    let body = payload.split("\r\n\r\n").nth(1).expect("find rpc body");
    serde_json::from_str(body).expect("parse rpc json")
}

fn capture_register_requests(
    count: usize,
) -> (
    String,
    mpsc::Receiver<(String, Value)>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start register capture");
    let addr = format!("http://{}", server.server_addr());
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..count {
            let request = server.recv().expect("receive register request");
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .expect("read register body");
            let parsed: Value = if body.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&body).expect("parse register body")
            };
            tx.send((request.url().to_string(), parsed))
                .expect("send register body");
            let response_body = match request.url() {
                "/api/hotmail/batches" => {
                    json!({"batch_id": "batch-1", "total": 2, "completed": 0, "success": 0, "failed": 0, "finished": false, "cancelled": false, "logs": [], "artifacts": []})
                }
                "/api/hotmail/batches/batch-1" => {
                    json!({"batch_id": "batch-1", "total": 2, "completed": 1, "success": 1, "failed": 0, "finished": false, "cancelled": false, "logs": ["ok"], "artifacts": []})
                }
                "/api/hotmail/batches/batch-1/cancel" => {
                    json!({"success": true, "batch_id": "batch-1"})
                }
                "/api/hotmail/batches/batch-1/continue" => {
                    json!({"batch_id": "batch-1", "total": 2, "completed": 1, "success": 1, "failed": 0, "finished": false, "cancelled": false, "logs": ["continued"], "artifacts": []})
                }
                "/api/hotmail/batches/batch-1/abandon" => {
                    json!({"batch_id": "batch-1", "total": 2, "completed": 1, "success": 0, "failed": 1, "finished": false, "cancelled": false, "logs": ["abandoned"], "artifacts": []})
                }
                "/api/hotmail/batches/batch-1/artifacts" => {
                    json!({"batch_id": "batch-1", "artifacts": [{"filename": "batch-1.txt", "path": "/tmp/batch-1.txt", "size": 12}]})
                }
                other => panic!("unexpected path: {other}"),
            };
            let response = Response::from_string(response_body.to_string())
                .with_status_code(StatusCode(200))
                .with_header(
                    Header::from_bytes("Content-Type", "application/json").expect("header"),
                );
            request.respond(response).expect("respond register request");
        }
    });
    (addr, rx, handle)
}

fn send_rpc_request(req: &JsonRpcRequest) -> Value {
    let server = start_one_shot_server().expect("start rpc server");
    let response = post_rpc(
        &server.addr,
        &serde_json::to_string(req).expect("serialize rpc"),
    );
    server.join();
    response
}

#[test]
fn rpc_hotmail_forwarding_proxies_batch_endpoints() {
    let _lock = REGISTER_ENV_LOCK.lock().unwrap();
    let (register_url, body_rx, handle) = capture_register_requests(6);
    let _env_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);

    let create = JsonRpcRequest {
        id: 1,
        method: "account/register/hotmailBatch/start".to_string(),
        params: Some(json!({
            "count": 2,
            "concurrency": 1,
            "intervalMin": 1,
            "intervalMax": 2,
            "proxy": "http://127.0.0.1:7890",
        })),
    };
    let read = JsonRpcRequest {
        id: 2,
        method: "account/register/hotmailBatch/read".to_string(),
        params: Some(json!({ "batchId": "batch-1" })),
    };
    let cancel = JsonRpcRequest {
        id: 3,
        method: "account/register/hotmailBatch/cancel".to_string(),
        params: Some(json!({ "batchId": "batch-1" })),
    };
    let continue_batch = JsonRpcRequest {
        id: 4,
        method: "account/register/hotmailBatch/continue".to_string(),
        params: Some(json!({ "batchId": "batch-1" })),
    };
    let abandon = JsonRpcRequest {
        id: 5,
        method: "account/register/hotmailBatch/abandon".to_string(),
        params: Some(json!({ "batchId": "batch-1" })),
    };
    let artifacts = JsonRpcRequest {
        id: 6,
        method: "account/register/hotmailBatch/artifacts".to_string(),
        params: Some(json!({ "batchId": "batch-1" })),
    };

    let responses = vec![create, read, cancel, continue_batch, abandon, artifacts]
        .into_iter()
        .map(|req| send_rpc_request(&req))
        .collect::<Vec<_>>();

    for response in responses {
        assert!(response.get("result").is_some(), "rpc should return result");
    }

    let expected_paths = [
        "/api/hotmail/batches",
        "/api/hotmail/batches/batch-1",
        "/api/hotmail/batches/batch-1/cancel",
        "/api/hotmail/batches/batch-1/continue",
        "/api/hotmail/batches/batch-1/abandon",
        "/api/hotmail/batches/batch-1/artifacts",
    ];

    for expected_path in expected_paths {
        let (path, _) = body_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("receive register body");
        assert_eq!(path, expected_path);
    }

    handle.join().expect("join capture server");
}
