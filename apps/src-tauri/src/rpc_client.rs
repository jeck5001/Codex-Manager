use codexmanager_core::rpc::types::JsonRpcRequest;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

pub(crate) fn normalize_addr(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("addr is empty".to_string());
    }
    let mut value = trimmed;
    if let Some(rest) = value.strip_prefix("http://") {
        value = rest;
    }
    if let Some(rest) = value.strip_prefix("https://") {
        value = rest;
    }
    let value = value.split('/').next().unwrap_or(value);
    if value.contains(':') {
        Ok(normalize_host(value))
    } else {
        Ok(format!("localhost:{value}"))
    }
}

fn resolve_service_addr(addr: Option<String>) -> Result<String, String> {
    if let Some(addr) = addr {
        return normalize_addr(&addr);
    }
    if let Ok(env_addr) = std::env::var("CODEXMANAGER_SERVICE_ADDR") {
        if let Ok(addr) = normalize_addr(&env_addr) {
            return Ok(addr);
        }
    }
    Ok(codexmanager_service::DEFAULT_ADDR.to_string())
}

fn split_http_response(buf: &str) -> Option<(&str, &str)> {
    if let Some((headers, body)) = buf.split_once("\r\n\r\n") {
        return Some((headers, body));
    }
    if let Some((headers, body)) = buf.split_once("\n\n") {
        return Some((headers, body));
    }
    None
}

fn response_uses_chunked(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
    })
}

fn decode_chunked_body(raw: &str) -> Result<String, String> {
    let bytes = raw.as_bytes();
    let mut cursor = 0usize;
    let mut out = Vec::<u8>::new();

    loop {
        let Some(line_end_rel) = bytes[cursor..].windows(2).position(|w| w == b"\r\n") else {
            return Err("Invalid chunked body: missing chunk size line".to_string());
        };
        let line_end = cursor + line_end_rel;
        let line = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|err| format!("Invalid chunked body: chunk size is not utf8 ({err})"))?;
        let size_hex = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| format!("Invalid chunked body: bad chunk size '{size_hex}'"))?;
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let end = cursor.saturating_add(size);
        if end + 2 > bytes.len() {
            return Err("Invalid chunked body: truncated chunk payload".to_string());
        }
        out.extend_from_slice(&bytes[cursor..end]);
        if &bytes[end..end + 2] != b"\r\n" {
            return Err("Invalid chunked body: missing chunk terminator".to_string());
        }
        cursor = end + 2;
    }

    String::from_utf8(out).map_err(|err| format!("Invalid chunked body utf8 payload: {err}"))
}

fn parse_http_body(buf: &str) -> Result<String, String> {
    let Some((headers, body_raw)) = split_http_response(buf) else {
        // 中文注释：旧实现按原始 socket 读取，理论上总是 HTTP 报文；但在代理/半关闭边界上可能只拿到 body。
        // 这里回退为“整段按 body 处理”，避免把可解析的 JSON 误判成 malformed。
        return Ok(buf.to_string());
    };
    if response_uses_chunked(headers) {
        decode_chunked_body(body_raw)
    } else {
        Ok(body_raw.to_string())
    }
}

fn resolve_socket_addrs(addr: &str) -> Result<Vec<SocketAddr>, String> {
    let addrs = addr
        .to_socket_addrs()
        .map_err(|err| format!("Invalid service address {addr}: {err}"))?;
    let mut out = Vec::new();
    for sock in addrs {
        if !out.iter().any(|item| item == &sock) {
            out.push(sock);
        }
    }
    if out.is_empty() {
        return Err(format!(
            "Invalid service address {addr}: no address resolved"
        ));
    }
    Ok(out)
}

fn rpc_call_on_socket(
    method: &str,
    addr: &str,
    sock: SocketAddr,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut stream =
        TcpStream::connect_timeout(&sock, Duration::from_millis(400)).map_err(|e| {
            let msg = format!("Failed to connect to service at {addr}: {e}");
            log::warn!(
                "rpc connect failed ({} -> {} via {}): {}",
                method,
                addr,
                sock,
                e
            );
            msg
        })?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let req = JsonRpcRequest {
        id: 1,
        method: method.to_string(),
        params,
    };
    let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    let rpc_token = codexmanager_service::rpc_auth_token();
    let http = format!(
        "POST /rpc HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-CodexManager-Rpc-Token: {rpc_token}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );
    stream.write_all(http.as_bytes()).map_err(|e| {
        let msg = e.to_string();
        log::warn!(
            "rpc write failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf).map_err(|e| {
        let msg = e.to_string();
        log::warn!(
            "rpc read failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    let body = parse_http_body(&buf).map_err(|msg| {
        log::warn!(
            "rpc parse failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    if body.trim().is_empty() {
        log::warn!("rpc empty response ({} -> {} via {})", method, addr, sock);
        return Err(
            "Empty response from service (service not ready, exited, or port occupied)".to_string(),
        );
    }

    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        let msg = format!("Unexpected RPC response (non-JSON body): {e}");
        log::warn!(
            "rpc json parse failed ({} -> {} via {}): {}",
            method,
            addr,
            sock,
            msg
        );
        msg
    })?;
    if let Some(err) = v.get("error") {
        log::warn!("rpc error ({} -> {} via {}): {}", method, addr, sock, err);
    }
    Ok(v)
}

pub(crate) fn rpc_call_with_sockets(
    method: &str,
    addr: &str,
    socket_addrs: &[SocketAddr],
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if socket_addrs.is_empty() {
        return Err(format!(
            "Invalid service address {addr}: no address resolved"
        ));
    }
    let mut last_err =
        "Empty response from service (service not ready, exited, or port occupied)".to_string();
    for attempt in 0..=1 {
        for sock in socket_addrs {
            match rpc_call_on_socket(method, addr, *sock, params.clone()) {
                Ok(v) => return Ok(v),
                Err(err) => last_err = err,
            }
        }
        if attempt == 0 {
            std::thread::sleep(Duration::from_millis(120));
        }
    }
    Err(last_err)
}

pub(crate) fn rpc_call(
    method: &str,
    addr: Option<String>,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let addr = resolve_service_addr(addr)?;
    let socket_addrs = resolve_socket_addrs(&addr)?;
    rpc_call_with_sockets(method, &addr, &socket_addrs, params)
}

fn normalize_host(value: &str) -> String {
    if let Some((host, port)) = value.rsplit_once(':') {
        let mapped = match host {
            "127.0.0.1" | "0.0.0.0" | "::1" | "[::1]" => "localhost",
            _ => host,
        };
        format!("{mapped}:{port}")
    } else {
        value.to_string()
    }
}
