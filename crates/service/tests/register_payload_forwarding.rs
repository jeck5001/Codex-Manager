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
            let mut request = server.recv().expect("receive register request");
            assert_eq!(request.method().as_str(), "POST");
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .expect("read register body");
            let parsed: Value = serde_json::from_str(&body).expect("parse register body");
            tx.send((request.url().to_string(), parsed))
                .expect("send register body");
            let response = Response::from_string(json!({"ok": true}).to_string())
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
fn rpc_register_forwarding_tracks_auto_create_temp_mail_service_variants() {
    let _lock = REGISTER_ENV_LOCK.lock().unwrap();
    let (register_url, body_rx, handle) = capture_register_requests(2);
    let _env_guard = EnvGuard::set("CODEXMANAGER_REGISTER_SERVICE_URL", &register_url);
    let _engine_guard = EnvGuard::set("CODEXMANAGER_REGISTER_ENGINE_TEST_MODE", "success");

    let camel_case = JsonRpcRequest {
        id: 1,
        method: "account/register/start".to_string(),
        params: Some(json!({
            "emailServiceType": "temp_mail",
            "autoCreateTempMailService": true
        })),
    };
    let snake_case = JsonRpcRequest {
        id: 2,
        method: "account/register/start".to_string(),
        params: Some(json!({
            "emailServiceType": "temp_mail",
            "auto_create_temp_mail_service": true
        })),
    };
    let absent_field = JsonRpcRequest {
        id: 3,
        method: "account/register/start".to_string(),
        params: Some(json!({"emailServiceType": "temp_mail"})),
    };

    let batch_with_flag = JsonRpcRequest {
        id: 4,
        method: "account/register/batch/start".to_string(),
        params: Some(json!({
            "emailServiceType": "temp_mail",
            "count": 1,
            "intervalMin": 1,
            "intervalMax": 1,
            "concurrency": 1,
            "mode": "pipeline",
            "autoCreateTempMailService": true
        })),
    };
    let batch_without_flag = JsonRpcRequest {
        id: 5,
        method: "account/register/batch/start".to_string(),
        params: Some(json!({
            "emailServiceType": "temp_mail",
            "count": 1,
            "intervalMin": 1,
            "intervalMax": 1,
            "concurrency": 1,
            "mode": "pipeline"
        })),
    };

    let responses = vec![
        camel_case,
        snake_case,
        absent_field,
        batch_with_flag,
        batch_without_flag,
    ]
    .into_iter()
    .map(|req| send_rpc_request(&req))
    .collect::<Vec<_>>();

    for response in responses.iter() {
        assert!(response.get("result").is_some());
    }

    let mut batch_requests = Vec::new();
    for _ in 0..2 {
        let (path, body) = body_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("receive register body");
        match path.as_str() {
            "/api/registration/batch" => batch_requests.push(body),
            other => panic!("unexpected path: {}", other),
        }
    }

    assert_eq!(batch_requests.len(), 2);
    for response in responses.iter().take(3) {
        let result = response.get("result").expect("local start result");
        assert_eq!(result.get("status").and_then(Value::as_str), Some("queued"));
        assert!(result
            .get("taskUuid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .starts_with("reg-"));
    }

    assert_eq!(
        batch_requests[0]
            .get("auto_create_temp_mail_service")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(batch_requests[1]
        .get("auto_create_temp_mail_service")
        .is_none());

    handle.join().expect("join capture server");
}

#[test]
fn rpc_register_start_uses_local_engine_for_generator_email() {
    let _lock = REGISTER_ENV_LOCK.lock().unwrap();
    let _engine_guard = EnvGuard::set("CODEXMANAGER_REGISTER_ENGINE_TEST_MODE", "success");

    let request = JsonRpcRequest {
        id: 11,
        method: "account/register/start".to_string(),
        params: Some(json!({
            "emailServiceType": "generator_email",
            "registerMode": "standard"
        })),
    };

    let response = send_rpc_request(&request);
    let result = response.get("result").expect("result payload");
    assert_eq!(
        result.get("emailServiceType").and_then(Value::as_str),
        Some("generator_email")
    );
    assert_eq!(
        result.get("registerMode").and_then(Value::as_str),
        Some("standard")
    );
    assert_eq!(result.get("status").and_then(Value::as_str), Some("queued"));
    assert!(result
        .get("taskUuid")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .starts_with("reg-"));
}
