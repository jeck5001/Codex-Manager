use super::*;

#[test]
fn gemini_generate_content_request_maps_to_responses() {
    let body = serde_json::json!({
        "systemInstruction": {
            "parts": [{ "text": "你是一个代码助手" }]
        },
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "你好" }]
            }
        ],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 256
        }
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        body,
    )
    .expect("adapt request");
    assert_eq!(adapted.path, "/v1/responses");
    assert_eq!(adapted.response_adapter, ResponseAdapter::GeminiJson);

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["model"], "gemini-2.5-pro");
    assert_eq!(value["instructions"], "你是一个代码助手");
    assert_eq!(value["input"][0]["role"], "user");
    assert_eq!(value["input"][0]["content"][0]["text"], "你好");
    assert_eq!(value["temperature"], 0.3);
    assert_eq!(value["max_output_tokens"], 256);
    assert_eq!(value["stream"], false);
}

#[test]
fn gemini_stream_generate_content_request_uses_sse_adapter_and_maps_tools() {
    let body = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "列出文件" }]
            }
        ],
        "tools": [{
            "functionDeclarations": [{
                "name": "list_files",
                "description": "列出文件",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    }
                }
            }]
        }],
        "toolConfig": {
            "functionCallingConfig": {
                "mode": "ANY",
                "allowedFunctionNames": ["list_files"]
            }
        }
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse",
        body,
    )
    .expect("adapt request");
    assert_eq!(adapted.response_adapter, ResponseAdapter::GeminiSse);

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["tools"][0]["type"], "function");
    assert_eq!(value["tools"][0]["name"], "list_files");
    assert_eq!(value["tool_choice"]["type"], "function");
    assert_eq!(value["tool_choice"]["name"], "list_files");
    assert_eq!(value["stream"], true);
}

#[test]
fn gemini_tools_preserve_parameters_json_schema_required_fields() {
    let body = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "在桌面创建文件" }]
            }
        ],
        "tools": [{
            "functionDeclarations": [{
                "name": "run_shell_command",
                "description": "运行命令",
                "parametersJsonSchema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "dir_path": { "type": "string" }
                    },
                    "required": ["command"]
                }
            }, {
                "name": "write_file",
                "description": "写入文件",
                "parametersJsonSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["file_path", "content"]
                }
            }]
        }]
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        body,
    )
    .expect("adapt request");

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["tools"][0]["name"], "run_shell_command");
    assert_eq!(
        value["tools"][0]["parameters"]["required"],
        serde_json::json!(["command"])
    );
    assert_eq!(value["tools"][1]["name"], "write_file");
    assert_eq!(
        value["tools"][1]["parameters"]["required"],
        serde_json::json!(["file_path", "content"])
    );
}

#[test]
fn gemini_mcp_tool_names_are_sanitized_for_openai_and_restored() {
    let original_tool_name =
        "mcp_browser_server_extremely_long_tool_name_that_gemini_cli_would_truncate...take_snapshot";
    let body = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "查看页面元素" }]
            }
        ],
        "tools": [{
            "functionDeclarations": [{
                "name": original_tool_name,
                "description": "浏览器 MCP 工具",
                "parametersJsonSchema": {
                    "type": "object",
                    "properties": {
                        "uid": { "type": "string" }
                    }
                }
            }]
        }],
        "toolConfig": {
            "functionCallingConfig": {
                "mode": "ANY",
                "allowedFunctionNames": [original_tool_name]
            }
        }
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        body,
    )
    .expect("adapt request");

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    let mapped_name = value["tools"][0]["name"]
        .as_str()
        .expect("mapped tool name")
        .to_string();
    assert_ne!(mapped_name, original_tool_name);
    assert!(mapped_name.len() <= 64);
    assert!(!mapped_name.contains('.'));
    assert_eq!(
        adapted.tool_name_restore_map.get(&mapped_name),
        Some(&original_tool_name.to_string())
    );
    assert_eq!(value["tool_choice"]["name"], mapped_name);
}
