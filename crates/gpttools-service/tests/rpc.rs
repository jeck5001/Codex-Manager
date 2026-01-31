use gpttools_core::rpc::types::JsonRpcRequest;
use std::io::{Read, Write};
use std::net::TcpStream;

fn post_rpc(addr: &str, body: &str) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).expect("connect server");
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).expect("write");
    stream.shutdown(std::net::Shutdown::Write).ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read");
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("");
    serde_json::from_str(body).expect("parse response")
}

#[test]
fn rpc_initialize_roundtrip() {
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 1,
        method: "initialize".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    assert_eq!(result.get("server_name").unwrap(), "gpttools-service");
}

#[test]
fn rpc_account_list_empty() {
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 2,
        method: "account/list".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    let items = result.get("items").expect("items").as_array().unwrap();
    assert!(items.is_empty());
}

#[test]
fn rpc_login_start_returns_url() {
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 3,
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
fn rpc_usage_read_empty() {
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 4,
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
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 5,
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
    let server = gpttools_service::start_one_shot_server().expect("start server");

    let req = JsonRpcRequest {
        id: 6,
        method: "account/usage/list".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let v = post_rpc(&server.addr, &json);
    let result = v.get("result").expect("result");
    let items = result.get("items").expect("items").as_array().unwrap();
    assert!(items.is_empty());
}
