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
    assert_eq!(
        value["candidates"][0]["content"]["parts"][0]["text"],
        "已完成"
    );
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
    let sanitized_tool_name = "mcp_browser_server_extremely_long_tool_name_that_gemini_cli_w";
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
fn gemini_cli_json_response_wraps_gemini_payload_in_response_field() {
    let upstream = serde_json::json!({
        "id": "resp_gemini_cli_1",
        "model": "gpt-5.4",
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "已完成" }]
        }],
        "usage": {
            "input_tokens": 3,
            "output_tokens": 2,
            "total_tokens": 5
        }
    });
    let upstream = serde_json::to_vec(&upstream).expect("serialize upstream");
    let (body, content_type) = adapt_upstream_response(
        ResponseAdapter::GeminiCliJson,
        Some("application/json"),
        &upstream,
    )
    .expect("adapt response");
    assert_eq!(content_type, "application/json");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("gemini cli response");
    assert_eq!(
        value["response"]["candidates"][0]["content"]["role"],
        "model"
    );
    assert_eq!(
        value["response"]["candidates"][0]["content"]["parts"][0]["text"],
        "已完成"
    );
    assert_eq!(value["response"]["usageMetadata"]["totalTokenCount"], 5);
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
    assert_eq!(
        events[0]["candidates"][0]["content"]["parts"][0]["text"],
        "你好"
    );
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

#[test]
fn gemini_cli_sse_response_wraps_each_chunk_in_response_field() {
    let upstream = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"response_id\":\"resp_gemini_cli_stream\",\"model\":\"gpt-5.4\",\"delta\":\"你好\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini_cli_stream\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
        "data: [DONE]\n\n",
    );
    let (body, content_type) = adapt_upstream_response(
        ResponseAdapter::GeminiCliSse,
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
    assert_eq!(
        events[0]["response"]["candidates"][0]["content"]["parts"][0]["text"],
        "你好"
    );
    assert_eq!(
        events[1]["response"]["candidates"][0]["finishReason"],
        serde_json::Value::String("STOP".to_string())
    );
    assert_eq!(events[1]["response"]["usageMetadata"]["totalTokenCount"], 3);
}

#[test]
fn gemini_sse_response_ignores_output_item_id_for_tool_call_identity() {
    let upstream = concat!(
        "data: {\"type\":\"response.output_item.added\",\"response_id\":\"resp_gemini_tool_id\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_linux_do_1\",\"name\":\"chrome_devtools_new_page\"}}\n\n",
        "data: {\"type\":\"response.function_call_arguments.done\",\"response_id\":\"resp_gemini_tool_id\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item_id\":\"fc_linux_do_1\",\"arguments\":\"{\\\"url\\\":\\\"https://linux.do\\\"}\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"response_id\":\"resp_gemini_tool_id\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_linux_do_1\",\"call_id\":\"call_linux_do_1\",\"name\":\"chrome_devtools_new_page\",\"arguments\":\"{\\\"url\\\":\\\"https://linux.do\\\"}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini_tool_id\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":3,\"output_tokens\":4,\"total_tokens\":7}}}\n\n",
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

    let tool_events = events
        .iter()
        .filter(|event| event["functionCalls"].is_array())
        .collect::<Vec<_>>();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(
        tool_events[0]["functionCalls"][0]["name"],
        "chrome_devtools_new_page"
    );
    assert_eq!(
        tool_events[0]["functionCalls"][0]["id"],
        serde_json::Value::String("call_linux_do_1".to_string())
    );
    assert_eq!(
        tool_events[0]["functionCalls"][0]["args"]["url"],
        "https://linux.do"
    );
}

#[test]
fn gemini_sse_response_waits_for_completed_arguments_before_emitting_tool_call() {
    let upstream = concat!(
        "data: {\"type\":\"response.output_item.added\",\"response_id\":\"resp_gemini_wait_args\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_write_file_1\",\"name\":\"write_file\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"response_id\":\"resp_gemini_wait_args\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_write_file_1\",\"call_id\":\"call_write_file_1\",\"name\":\"write_file\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini_wait_args\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_write_file_1\",\"call_id\":\"call_write_file_1\",\"name\":\"write_file\",\"arguments\":\"{\\\"file_path\\\":\\\"Desktop\\\\\\\\gemini.txt\\\",\\\"content\\\":\\\"偏松没地方开始骂开发gemini\\\"}\"}],\"usage\":{\"input_tokens\":6,\"output_tokens\":5,\"total_tokens\":11}}}\n\n",
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

    let tool_events = events
        .iter()
        .filter(|event| event["functionCalls"].is_array())
        .collect::<Vec<_>>();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(tool_events[0]["functionCalls"][0]["name"], "write_file");
    assert_eq!(
        tool_events[0]["functionCalls"][0]["args"]["file_path"],
        "Desktop\\gemini.txt"
    );
    assert_eq!(
        tool_events[0]["functionCalls"][0]["args"]["content"],
        "偏松没地方开始骂开发gemini"
    );
}

#[test]
fn gemini_sse_response_skips_placeholder_arguments_until_completed_tool_call() {
    let upstream = concat!(
        "data: {\"type\":\"response.output_item.added\",\"response_id\":\"resp_gemini_placeholder_args\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_linux_do_1\",\"name\":\"chrome_devtools_new_page\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"response_id\":\"resp_gemini_placeholder_args\",\"model\":\"gpt-5.4\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_linux_do_1\",\"call_id\":\"call_linux_do_1\",\"name\":\"chrome_devtools_new_page\",\"arguments\":\"{}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_gemini_placeholder_args\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_linux_do_1\",\"call_id\":\"call_linux_do_1\",\"name\":\"chrome_devtools_new_page\",\"arguments\":\"{\\\"url\\\":\\\"https://linux.do\\\"}\"}],\"usage\":{\"input_tokens\":4,\"output_tokens\":5,\"total_tokens\":9}}}\n\n",
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

    let tool_events = events
        .iter()
        .filter(|event| event["functionCalls"].is_array())
        .collect::<Vec<_>>();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(
        tool_events[0]["functionCalls"][0]["name"],
        "chrome_devtools_new_page"
    );
    assert_eq!(
        tool_events[0]["functionCalls"][0]["args"]["url"],
        "https://linux.do"
    );
}
