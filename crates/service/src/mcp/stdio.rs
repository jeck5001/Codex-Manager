use std::io::{self, BufRead, BufReader, Write};

const CONTENT_TYPE_HEADER: &str = "Content-Type: application/json\r\n";

pub fn run_stdio_server() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    serve_stdio(&mut reader, &mut writer).map_err(|err| format!("mcp stdio server failed: {err}"))
}

pub(crate) fn serve_stdio<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<()> {
    loop {
        let Some(payload) = read_framed_message(reader)? else {
            return Ok(());
        };
        if let Some(response) = crate::mcp::session::handle_jsonrpc_message(&payload) {
            write_framed_message(writer, response.to_string().as_bytes())?;
        }
    }
}

fn read_framed_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut saw_header = false;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            if saw_header {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected EOF while reading MCP headers",
                ));
            }
            return Ok(None);
        }

        saw_header = true;
        if line == "\r\n" || line == "\n" {
            break;
        }

        let header = line.trim_end_matches(['\r', '\n']);
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(value.trim().parse::<usize>().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length header: {err}"),
                )
            })?);
        }
    }

    let Some(length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header",
        ));
    };
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload)?;
    Ok(Some(payload))
}

fn write_framed_message<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n", payload.len())?;
    writer.write_all(CONTENT_TYPE_HEADER.as_bytes())?;
    writer.write_all(b"\r\n")?;
    writer.write_all(payload)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{read_framed_message, serve_stdio};
    use crate::mcp::session::install_chat_completion_override;
    use codexmanager_core::rpc::types::ModelOption;
    use codexmanager_core::storage::{now_ts, Account, Storage, UsageSnapshotRecord};
    use serde_json::{json, Value};
    use std::fs;
    use std::io::Cursor;
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

    fn frame(payload: Value) -> Vec<u8> {
        let body = payload.to_string();
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    fn decode_frames(bytes: &[u8]) -> Vec<Value> {
        let mut cursor = Cursor::new(bytes.to_vec());
        let mut frames = Vec::new();
        while let Some(payload) = read_framed_message(&mut cursor).expect("read output frame") {
            frames.push(serde_json::from_slice(&payload).expect("decode output json"));
        }
        frames
    }

    #[test]
    fn stdio_server_handles_initialize_and_tools_list() {
        let (_db_scope, _storage) = setup_test_db("mcp-stdio-init-tools");
        crate::set_mcp_enabled(true).expect("enable mcp");

        let mut input = Vec::new();
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26"
            }
        })));
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })));

        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["result"]["protocolVersion"], "2025-03-26");
        let tools = frames[1]["result"]["tools"]
            .as_array()
            .expect("tools array");
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0]["name"], "chat_completion");
        assert_eq!(tools[1]["name"], "list_models");
        assert_eq!(tools[2]["name"], "list_accounts");
        assert_eq!(tools[3]["name"], "get_usage");
    }

    #[test]
    fn stdio_server_rejects_initialize_when_mcp_is_disabled() {
        let (_db_scope, _storage) = setup_test_db("mcp-stdio-disabled");
        crate::set_mcp_enabled(false).expect("disable mcp");

        let mut reader = Cursor::new(frame(json!({
            "jsonrpc": "2.0",
            "id": "init-disabled",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26"
            }
        })));
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["error"]["code"], -32001);
        assert!(frames[0]["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("设置中禁用"));
    }

    #[test]
    fn stdio_server_rejects_tool_requests_when_mcp_is_disabled() {
        let (_db_scope, _storage) = setup_test_db("mcp-stdio-tools-disabled");
        crate::set_mcp_enabled(false).expect("disable mcp");

        let mut input = Vec::new();
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": "tools-list-disabled",
            "method": "tools/list",
            "params": {}
        })));
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": "tools-call-disabled",
            "method": "tools/call",
            "params": {
                "name": "list_models",
                "arguments": {}
            }
        })));

        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["error"]["code"], -32001);
        assert_eq!(frames[1]["error"]["code"], -32001);
        assert!(frames.iter().all(|frame| {
            frame["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("设置中禁用"))
        }));
    }

    #[test]
    fn stdio_server_reports_unknown_method() {
        let mut reader = Cursor::new(frame(json!({
            "jsonrpc": "2.0",
            "id": "req-404",
            "method": "unknown/method"
        })));
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["error"]["code"], -32601);
        assert!(frames[0]["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("unknown/method"));
    }

    #[test]
    fn stdio_server_executes_read_only_tools() {
        let (_db_scope, storage) = setup_test_db("mcp-stdio-read-tools");
        crate::set_mcp_enabled(true).expect("enable mcp");
        storage
            .upsert_model_options_cache(
                "default",
                &serde_json::to_string(&vec![ModelOption {
                    slug: "gpt-5".to_string(),
                    display_name: "GPT-5".to_string(),
                }])
                .expect("serialize cached models"),
                now_ts(),
            )
            .expect("seed cached models");
        storage
            .insert_account(&Account {
                id: "acc-mcp-1".to_string(),
                label: "MCP Account".to_string(),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: Some("default".to_string()),
                sort: 0,
                status: "active".to_string(),
                created_at: now_ts(),
                updated_at: now_ts(),
            })
            .expect("insert account");
        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: "acc-mcp-1".to_string(),
                used_percent: Some(20.0),
                window_minutes: Some(300),
                resets_at: None,
                secondary_used_percent: None,
                secondary_window_minutes: None,
                secondary_resets_at: None,
                credits_json: None,
                captured_at: now_ts(),
            })
            .expect("insert usage snapshot");

        let mut input = Vec::new();
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "list_models",
                "arguments": {}
            }
        })));
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "list_accounts",
                "arguments": {}
            }
        })));
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "get_usage",
                "arguments": {}
            }
        })));

        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 3);

        assert_eq!(frames[0]["result"]["isError"], false);
        assert_eq!(
            frames[0]["result"]["structuredContent"]["items"][0]["slug"],
            "gpt-5"
        );

        assert_eq!(frames[1]["result"]["isError"], false);
        assert_eq!(
            frames[1]["result"]["structuredContent"]["items"][0]["id"],
            "acc-mcp-1"
        );
        assert_eq!(
            frames[1]["result"]["structuredContent"]["items"][0]["status"],
            "active"
        );

        assert_eq!(frames[2]["result"]["isError"], false);
        assert_eq!(
            frames[2]["result"]["structuredContent"]["primaryBucketCount"],
            1
        );
        assert_eq!(
            frames[2]["result"]["structuredContent"]["primaryRemainPercent"],
            80
        );
    }

    #[test]
    fn stdio_server_returns_tool_errors_for_unimplemented_or_unknown_tools() {
        let (_db_scope, _storage) = setup_test_db("mcp-stdio-tool-errors");
        crate::set_mcp_enabled(true).expect("enable mcp");

        let mut input = Vec::new();
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "chat_completion",
                "arguments": {
                    "model": "gpt-5",
                    "messages": []
                }
            }
        })));
        input.extend(frame(json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool",
                "arguments": {}
            }
        })));

        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["result"]["isError"], true);
        assert!(frames[0]["result"]["content"][0]["text"]
            .as_str()
            .expect("chat_completion error")
            .contains("missing api key"));
        assert_eq!(frames[1]["result"]["isError"], true);
        assert!(frames[1]["result"]["content"][0]["text"]
            .as_str()
            .expect("unknown tool error")
            .contains("unknown tool"));
    }

    #[test]
    fn stdio_server_executes_chat_completion_tool() {
        let (_db_scope, _storage) = setup_test_db("mcp-stdio-chat-completion");
        crate::set_mcp_enabled(true).expect("enable mcp");

        let _override = install_chat_completion_override(|payload, api_key| {
            assert_eq!(api_key, "cm-platform-key");
            assert_eq!(payload["model"], "gpt-5");
            assert_eq!(payload["stream"], false);
            assert!(payload.get("apiKey").is_none());
            assert_eq!(payload["messages"][0]["role"], "user");

            Ok(json!({
                "response": {
                    "id": "cmpl-mcp",
                    "object": "chat.completion",
                    "model": "gpt-5",
                    "choices": [
                        {
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": "hello from mcp"
                            },
                            "finish_reason": "stop"
                        }
                    ]
                },
                "gateway": {
                    "status": 200,
                    "actualModel": "gpt-5"
                }
            }))
        });

        let mut reader = Cursor::new(frame(json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "chat_completion",
                "arguments": {
                    "apiKey": "cm-platform-key",
                    "model": "gpt-5",
                    "messages": [
                        {
                            "role": "user",
                            "content": "hello"
                        }
                    ]
                }
            }
        })));
        let mut writer = Vec::new();
        serve_stdio(&mut reader, &mut writer).expect("serve stdio");

        let frames = decode_frames(&writer);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["result"]["isError"], false);
        assert_eq!(
            frames[0]["result"]["structuredContent"]["response"]["id"],
            "cmpl-mcp"
        );
        assert_eq!(
            frames[0]["result"]["structuredContent"]["response"]["choices"][0]["message"]
                ["content"],
            "hello from mcp"
        );
        assert_eq!(
            frames[0]["result"]["structuredContent"]["gateway"]["actualModel"],
            "gpt-5"
        );
    }
}
