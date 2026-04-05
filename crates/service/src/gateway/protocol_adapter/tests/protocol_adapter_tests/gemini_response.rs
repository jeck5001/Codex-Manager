use super::*;

#[test]
fn gemini_json_response_maps_from_openai_responses_shape() {
    let upstream = serde_json::json!({
        "id": "resp_gemini_1",
        "model": "gpt-5.4",
        "status": "completed",
        "output": [
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": "已完成" }
                ]
            },
            {
                "type": "function_call",
                "call_id": "call_ls_1",
                "name": "list_files",
                "arguments": "{\"path\":\".\"}"
            }
        ],
        "usage": {
            "input_tokens": 8,
            "output_tokens": 5,
            "total_tokens": 13
        }
    });
    let upstream = serde_json::to_vec(&upstream).expect("serialize upstream");
    let (body, content_type) = adapt_upstream_response(
        ResponseAdapter::GeminiJson,
        Some("application/json"),
        &upstream,
    )
    .expect("adapt response");
    assert_eq!(content_type, "application/json");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("gemini response");
    assert_eq!(value["candidates"][0]["content"]["role"], "model");
    assert_eq!(value["candidates"][0]["content"]["parts"][0]["text"], "已完成");
    assert_eq!(
        value["candidates"][0]["content"]["parts"][1]["functionCall"]["name"],
        "list_files"
    );
    assert_eq!(
        value["candidates"][0]["content"]["parts"][1]["functionCall"]["args"]["path"],
        "."
    );
    assert_eq!(value["functionCalls"][0]["name"], "list_files");
    assert_eq!(value["functionCalls"][0]["args"]["path"], ".");
    assert_eq!(value["usageMetadata"]["promptTokenCount"], 8);
    assert_eq!(value["usageMetadata"]["candidatesTokenCount"], 5);
    assert_eq!(value["usageMetadata"]["totalTokenCount"], 13);
}

#[test]
fn gemini_json_response_restores_sanitized_mcp_tool_names() {
    let original_tool_name =
        "mcp_browser_server_extremely_long_tool_name_that_gemini_cli_would_truncate...take_snapshot";
    let sanitized_tool_name =
        "mcp_browser_server_extremely_long_tool_name_that_gemini_cli_w";
    let mut restore_map = super::ToolNameRestoreMap::new();
    restore_map.insert(
        sanitized_tool_name.to_string(),
        original_tool_name.to_string(),
    );

    let upstream = serde_json::json!({
        "id": "resp_gemini_mcp_1",
        "model": "gpt-5.4",
        "status": "completed",
        "output": [
            {
                "type": "function_call",
                "call_id": "call_mcp_1",
                "name": sanitized_tool_name,
                "arguments": "{\"uid\":\"87_4\"}"
            }
        ]
    });
    let upstream = serde_json::to_vec(&upstream).expect("serialize upstream");
    let (body, content_type) = adapt_upstream_response_with_tool_name_restore_map(
        ResponseAdapter::GeminiJson,
        Some("application/json"),
        &upstream,
        Some(&restore_map),
    )
    .expect("adapt response");
    assert_eq!(content_type, "application/json");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("gemini response");
    assert_eq!(
        value["candidates"][0]["content"]["parts"][0]["functionCall"]["name"],
        original_tool_name
    );
    assert_eq!(value["functionCalls"][0]["name"], original_tool_name);
}

#[test]
fn gemini_sse_response_maps_openai_responses_event_stream() {
    let upstream = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"response_id\":\"resp_gemini_stream\",\"model\":\"gpt-5.4\",\"delta\":\"你好\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"response_id\":\"resp_gemini_stream\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"call_id\":\"call_ls_1\",\"name\":\"list_files\",\"arguments\":\"{\\\"path\\\":\\\".\\\"}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini_stream\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
        "data: [DONE]\n\n",
    );
    let (body, content_type) = adapt_upstream_response(
        ResponseAdapter::GeminiSse,
        Some("text/event-stream"),
        upstream.as_bytes(),
    )
    .expect("adapt stream");
    assert_eq!(content_type, "text/event-stream");

    let text = String::from_utf8(body).expect("utf8");
    let events = text
        .split("\n\n")
        .filter_map(|frame| frame.strip_prefix("data: "))
        .filter(|frame| !frame.trim().is_empty() && frame.trim() != "[DONE]")
        .map(|frame| serde_json::from_str::<serde_json::Value>(frame).expect("parse sse json"))
        .collect::<Vec<_>>();
    assert_eq!(events[0]["candidates"][0]["content"]["parts"][0]["text"], "你好");
    assert!(events[0]["candidates"][0]["finishReason"].is_null());
    assert_eq!(events[1]["functionCalls"][0]["name"], "list_files");
    assert_eq!(events[1]["functionCalls"][0]["args"]["path"], ".");
    assert_eq!(
        events[1]["candidates"][0]["content"]["parts"][0]["functionCall"]["id"],
        "call_ls_1"
    );
    assert_eq!(
        events[2]["candidates"][0]["finishReason"],
        serde_json::Value::String("STOP".to_string())
    );
    assert_eq!(events[2]["usageMetadata"]["totalTokenCount"], 3);
}
