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
    assert_eq!(value["instructions"], "");
    let input_items = value["input"].as_array().expect("input array");
    assert_eq!(input_items[0]["role"], "developer");
    assert_eq!(input_items[0]["content"][0]["type"], "input_text");
    assert_eq!(input_items[0]["content"][0]["text"], "你是一个代码助手");
    assert_eq!(input_items[1]["role"], "user");
    assert_eq!(input_items[1]["content"][0]["type"], "input_text");
    assert_eq!(input_items[1]["content"][0]["text"], "你好");
    assert_eq!(value["reasoning"]["effort"], "medium");
    assert_eq!(value["reasoning"]["summary"], "auto");
    assert_eq!(
        value["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(value["parallel_tool_calls"], true);
    assert_eq!(value["stream"], true);
    assert_eq!(value["store"], false);
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
    assert_eq!(
        adapted.gemini_stream_output_mode,
        Some(GeminiStreamOutputMode::Sse)
    );

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["tools"][0]["type"], "function");
    assert_eq!(value["tools"][0]["name"], "list_files");
    assert_eq!(value["tool_choice"], "auto");
    assert_eq!(value["stream"], true);
}

#[test]
fn gemini_cli_internal_generate_content_request_maps_to_responses() {
    let body = serde_json::json!({
        "project": "",
        "model": "gemini-2.5-pro",
        "request": {
            "systemInstruction": {
                "parts": [{ "text": "你是内部 Gemini CLI 助手" }]
            },
            "contents": [{
                "parts": [{ "text": "你好" }]
            }]
        }
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol("gemini_native", "/v1internal:generateContent", body)
        .expect("adapt request");
    assert_eq!(adapted.path, "/v1/responses");
    assert_eq!(adapted.response_adapter, ResponseAdapter::GeminiCliJson);

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["model"], "gemini-2.5-pro");
    assert_eq!(value["instructions"], "");
    let input_items = value["input"].as_array().expect("input array");
    assert_eq!(input_items[0]["role"], "developer");
    assert_eq!(
        input_items[0]["content"][0]["text"],
        "你是内部 Gemini CLI 助手"
    );
    assert_eq!(input_items[1]["role"], "");
    assert_eq!(input_items[1]["content"][0]["text"], "你好");
    assert_eq!(value["stream"], true);
}

#[test]
fn gemini_cli_internal_stream_request_uses_cli_sse_adapter() {
    let body = serde_json::json!({
        "project": "",
        "model": "gemini-2.5-pro",
        "request": {
            "contents": [{
                "parts": [{ "text": "列出文件" }]
            }]
        }
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1internal:streamGenerateContent?alt=sse",
        body,
    )
    .expect("adapt request");
    assert_eq!(adapted.response_adapter, ResponseAdapter::GeminiCliSse);
    assert_eq!(
        adapted.gemini_stream_output_mode,
        Some(GeminiStreamOutputMode::Sse)
    );

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    assert_eq!(value["model"], "gemini-2.5-pro");
    assert_eq!(value["stream"], true);
}

#[test]
fn gemini_raw_stream_request_uses_raw_output_mode() {
    let body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": "raw stream" }]
        }]
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=json",
        body,
    )
    .expect("adapt request");

    assert_eq!(adapted.response_adapter, ResponseAdapter::GeminiSse);
    assert_eq!(
        adapted.gemini_stream_output_mode,
        Some(GeminiStreamOutputMode::Raw)
    );
}

#[test]
fn gemini_auto_tool_config_filters_to_allowed_function_names() {
    let body = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "只允许 browser tool" }]
            }
        ],
        "tools": [{
            "functionDeclarations": [{
                "name": "list_files",
                "parameters": { "type": "object", "properties": {} }
            }, {
                "name": "mcp__browser__take_screenshot",
                "parameters": { "type": "object", "properties": {} }
            }]
        }],
        "toolConfig": {
            "functionCallingConfig": {
                "mode": "AUTO",
                "allowedFunctionNames": ["mcp__browser__take_screenshot"]
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
    let tools = value["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["name"], "list_files");
    assert_eq!(tools[1]["name"], "mcp__browser__take_screenshot");
    assert_eq!(
        value.get("tool_choice").and_then(serde_json::Value::as_str),
        Some("auto")
    );
}

#[test]
fn gemini_function_response_with_inline_data_maps_to_function_call_output_items() {
    let body = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": "帮我看截图" }]
            },
            {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "mcp__browser__take_screenshot",
                        "id": "call_browser_1",
                        "args": { "uid": "node-1" }
                    }
                }]
            },
            {
                "role": "user",
                "parts": [
                    {
                        "functionResponse": {
                            "name": "mcp__browser__take_screenshot",
                            "id": "call_browser_1",
                            "response": { "output": "截图已生成" }
                        }
                    },
                    {
                        "inlineData": {
                            "mimeType": "image/png",
                            "data": "ZmFrZS1pbWFnZQ=="
                        }
                    }
                ]
            }
        ]
    });
    let body = serde_json::to_vec(&body).expect("serialize request");

    let adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        body,
    )
    .expect("adapt request");

    let value: serde_json::Value = serde_json::from_slice(&adapted.body).expect("adapted json");
    let input_items = value["input"].as_array().expect("input array");
    let tool_call = input_items
        .iter()
        .find(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("function_call"))
        .expect("function_call item");
    let tool_output = input_items
        .iter()
        .find(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        })
        .expect("function_call_output item");
    let call_id = tool_call["call_id"].as_str().expect("call id");
    assert!(call_id.starts_with("call_"));
    assert_eq!(call_id.len(), 29);
    assert_eq!(tool_output["call_id"], call_id);
    assert!(tool_output["output"].is_string());
    assert_eq!(tool_output["output"], "{\"output\":\"截图已生成\"}");
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
    assert_eq!(
        adapted.tool_name_restore_map.get(&mapped_name),
        Some(&original_tool_name.to_string())
    );
    assert_eq!(value["tool_choice"], "auto");
}

#[test]
fn gemini_request_maps_thinking_config_and_tool_names() {
    let body = serde_json::json!({
        "system_instruction": {
            "parts": [{ "text": "你是一个谨慎的代码助手" }]
        },
        "contents": [{
            "role": "user",
            "parts": [{
                "text": "分析这个截图"
            }, {
                "inline_data": {
                    "mime_type": "image/png",
                    "data": "ZmFrZS1pbWFnZQ=="
                }
            }]
        }],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingBudget": 9000
            }
        },
        "tools": [{
            "functionDeclarations": [{
                "name": "tool.a/b",
                "description": "读取文件",
                "parametersJsonSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
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
    assert_eq!(value["instructions"], "");
    assert_eq!(value["reasoning"]["effort"], "high");
    assert_eq!(value["reasoning"]["summary"], "auto");
    assert_eq!(
        value["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(value["parallel_tool_calls"], true);
    assert_eq!(value["stream"], true);
    assert_eq!(value["store"], false);

    let input_items = value["input"].as_array().expect("input array");
    assert_eq!(input_items[0]["role"], "developer");
    assert_eq!(
        input_items[0]["content"][0]["text"],
        "你是一个谨慎的代码助手"
    );
    assert_eq!(input_items[1]["role"], "user");
    let content_items = input_items[1]["content"].as_array().expect("content array");
    assert_eq!(content_items.len(), 1);
    assert_eq!(content_items[0]["type"], "input_text");
    assert_eq!(content_items[0]["text"], "分析这个截图");

    let mapped_name = value["tools"][0]["name"]
        .as_str()
        .expect("mapped tool name");
    assert_eq!(mapped_name, "tool.a/b");
    assert_eq!(
        value["tools"][0]["parameters"]["required"],
        serde_json::json!(["path"])
    );
    assert_eq!(value["tool_choice"], "auto");
}

#[test]
fn gemini_request_unwraps_cli_shape_without_role_normalization() {
    let body = serde_json::json!({
        "project": "",
        "model": "gemini-2.5-pro",
        "request": {
            "contents": [
                {
                    "parts": [{ "text": "先列一下目录" }]
                },
                {
                    "role": "bad-role",
                    "parts": []
                },
                {
                    "parts": [{
                        "functionCall": {
                            "name": "list_files",
                            "args": { "path": "." }
                        }
                    }]
                },
                {
                    "role": "user",
                    "parts": [{
                        "functionResponse": {
                            "response": { "output": "Desktop\nDocuments" }
                        }
                    }]
                }
            ]
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
    let input_items = value["input"].as_array().expect("input array");
    assert_eq!(input_items[0]["role"], "");
    assert_eq!(input_items[0]["content"][0]["text"], "先列一下目录");
    let tool_call = input_items
        .iter()
        .find(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("function_call"))
        .expect("function_call item");
    let tool_output = input_items
        .iter()
        .find(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        })
        .expect("function_call_output item");
    let call_id = tool_call["call_id"].as_str().expect("call id");
    assert!(call_id.starts_with("call_"));
    assert_eq!(tool_output["call_id"], call_id);
    assert!(tool_output["output"].is_string());
    assert_eq!(
        tool_output["output"],
        "{\"output\":\"Desktop\\nDocuments\"}"
    );
}

#[test]
fn gemini_request_maps_minimal_none_auto_and_include_thoughts() {
    let none_body = serde_json::json!({
        "contents": [{ "role": "user", "parts": [{ "text": "hi" }] }],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingLevel": "none"
            }
        }
    });
    let none_body = serde_json::to_vec(&none_body).expect("serialize none request");
    let none_adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        none_body,
    )
    .expect("adapt none request");
    let none_value: serde_json::Value =
        serde_json::from_slice(&none_adapted.body).expect("adapted none json");
    assert_eq!(none_value["reasoning"]["effort"], "none");
    assert_eq!(none_value["reasoning"]["summary"], "auto");
    assert_eq!(
        none_value["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );

    let auto_body = serde_json::json!({
        "contents": [{ "role": "user", "parts": [{ "text": "hi" }] }],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingBudget": -1
            }
        }
    });
    let auto_body = serde_json::to_vec(&auto_body).expect("serialize auto request");
    let auto_adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        auto_body,
    )
    .expect("adapt auto request");
    let auto_value: serde_json::Value =
        serde_json::from_slice(&auto_adapted.body).expect("adapted auto json");
    assert_eq!(auto_value["reasoning"]["effort"], "auto");
    assert_eq!(auto_value["reasoning"]["summary"], "auto");
    assert_eq!(
        auto_value["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );

    let minimal_body = serde_json::json!({
        "contents": [{ "role": "user", "parts": [{ "text": "hi" }] }],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingLevel": "minimal",
                "includeThoughts": false
            }
        }
    });
    let minimal_body = serde_json::to_vec(&minimal_body).expect("serialize minimal request");
    let minimal_adapted = adapt_request_for_protocol(
        "gemini_native",
        "/v1beta/models/gemini-2.5-pro:generateContent",
        minimal_body,
    )
    .expect("adapt minimal request");
    let minimal_value: serde_json::Value =
        serde_json::from_slice(&minimal_adapted.body).expect("adapted minimal json");
    assert_eq!(minimal_value["reasoning"]["effort"], "minimal");
    assert_eq!(minimal_value["reasoning"]["summary"], "auto");
    assert_eq!(
        minimal_value["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
}
