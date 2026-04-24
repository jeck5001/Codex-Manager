use super::{AdaptedGatewayRequest, ResponseAdapter, ToolNameRestoreMap};

/// 函数 `adapt_request_for_protocol`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn adapt_request_for_protocol(
    _protocol_type: &str,
    path: &str,
    body: Vec<u8>,
) -> Result<AdaptedGatewayRequest, String> {
    Ok(AdaptedGatewayRequest {
        path: path.to_string(),
        body,
        response_adapter: ResponseAdapter::Passthrough,
        gemini_stream_output_mode: None,
        tool_name_restore_map: ToolNameRestoreMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::adapt_request_for_protocol;
    use crate::apikey_profile::{PROTOCOL_ANTHROPIC_NATIVE, PROTOCOL_GEMINI_NATIVE};
    use crate::gateway::ResponseAdapter;

    #[test]
    fn anthropic_messages_now_passthrough_without_responses_rewrite() {
        let body = br#"{"model":"claude-3-7-sonnet","messages":[]}"#.to_vec();

        let adapted =
            adapt_request_for_protocol(PROTOCOL_ANTHROPIC_NATIVE, "/v1/messages", body.clone())
                .expect("adapt anthropic request");

        assert_eq!(adapted.path, "/v1/messages");
        assert_eq!(adapted.body, body);
        assert_eq!(adapted.response_adapter, ResponseAdapter::Passthrough);
    }

    #[test]
    fn gemini_generate_content_now_passthrough_without_responses_rewrite() {
        let body = br#"{"contents":[]}"#.to_vec();

        let adapted = adapt_request_for_protocol(
            PROTOCOL_GEMINI_NATIVE,
            "/v1beta/models/gemini-2.5-pro:generateContent",
            body.clone(),
        )
        .expect("adapt gemini request");

        assert_eq!(
            adapted.path,
            "/v1beta/models/gemini-2.5-pro:generateContent"
        );
        assert_eq!(adapted.body, body);
        assert_eq!(adapted.response_adapter, ResponseAdapter::Passthrough);
    }
}
