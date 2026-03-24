use tiny_http::{Request, Response};

pub(crate) fn handle_gateway_request(mut request: Request) -> Result<(), String> {
    // 处理代理请求（鉴权后转发到上游）
    let debug = super::DEFAULT_GATEWAY_DEBUG;
    if request.method().as_str() == "OPTIONS" {
        let response = Response::empty(204);
        let _ = request.respond(response);
        return Ok(());
    }

    if request.url() == "/health" {
        let response = Response::from_string("ok");
        let _ = request.respond(response);
        return Ok(());
    }

    let _request_guard = super::begin_gateway_request();
    let trace_id = super::trace_log::next_trace_id();
    let request_path_for_log = super::normalize_models_path(request.url());
    let request_method_for_log = request.method().as_str().to_string();
    let mut validated =
        match super::local_validation::prepare_local_request(&mut request, trace_id.clone(), debug)
        {
            Ok(v) => v,
            Err(err) => {
                super::trace_log::log_request_start(super::trace_log::RequestStartLog {
                    trace_id: trace_id.as_str(),
                    key_id: "-",
                    method: request_method_for_log.as_str(),
                    path: request_path_for_log.as_str(),
                    model: None,
                    reasoning: None,
                    is_stream: false,
                    protocol_type: "-",
                });
                super::trace_log::log_request_final(
                    trace_id.as_str(),
                    err.status_code,
                    None,
                    None,
                    Some(err.message.as_str()),
                    0,
                );
                super::record_gateway_request_outcome(
                    request_path_for_log.as_str(),
                    err.status_code,
                    None,
                );
                if let Some(storage) = super::open_storage() {
                    super::write_request_log(
                        &storage,
                        super::request_log::RequestLogTraceContext {
                            trace_id: Some(trace_id.as_str()),
                            original_path: Some(request_path_for_log.as_str()),
                            adapted_path: Some(request_path_for_log.as_str()),
                            response_adapter: None,
                        },
                        super::request_log::RequestLogEntry {
                            key_id: None,
                            account_id: None,
                            request_path: &request_path_for_log,
                            method: &request_method_for_log,
                            model: None,
                            reasoning_effort: None,
                            upstream_url: None,
                            status_code: Some(err.status_code),
                            usage: super::request_log::RequestLogUsage::default(),
                            error: Some(err.message.as_str()),
                            duration_ms: None,
                        },
                    );
                }
                let response = super::error_response::with_retry_after_header(
                    super::error_response::terminal_text_response(
                        err.status_code,
                        err.message,
                        Some(trace_id.as_str()),
                    ),
                    err.retry_after_secs,
                );
                let _ = request.respond(response);
                return Ok(());
            }
        };

    let request = match super::maybe_respond_local_models(
        request,
        super::local_models::LocalModelsRequestContext {
            trace_id: validated.trace_id.as_str(),
            key_id: validated.key_id.as_str(),
            protocol_type: validated.protocol_type.as_str(),
            original_path: validated.original_path.as_str(),
            path: validated.path.as_str(),
            response_adapter: validated.response_adapter,
            request_method: validated.request_method.as_str(),
            model_for_log: validated.model_for_log.as_deref(),
            reasoning_for_log: validated.reasoning_for_log.as_deref(),
            storage: &validated.storage,
        },
    )? {
        Some(request) => request,
        None => return Ok(()),
    };

    let request = match super::maybe_respond_local_count_tokens(
        request,
        super::local_count_tokens::LocalCountTokensRequestContext {
            trace_id: validated.trace_id.as_str(),
            key_id: validated.key_id.as_str(),
            protocol_type: validated.protocol_type.as_str(),
            original_path: validated.original_path.as_str(),
            path: validated.path.as_str(),
            response_adapter: validated.response_adapter,
            request_method: validated.request_method.as_str(),
            body: validated.body.as_ref(),
            model_for_log: validated.model_for_log.as_deref(),
            reasoning_for_log: validated.reasoning_for_log.as_deref(),
            storage: &validated.storage,
        },
    )? {
        Some(request) => request,
        None => return Ok(()),
    };

    match crate::plugin_runtime::execute_pre_route_plugins(
        crate::plugin_runtime::PreRoutePluginInput {
            storage: &validated.storage,
            trace_id: validated.trace_id.as_str(),
            key_id: validated.key_id.as_str(),
            api_key_name: validated.api_key_name.as_deref(),
            path: validated.path.as_str(),
            method: validated.request_method.as_str(),
            body: &validated.body,
            model_for_log: validated.model_for_log.as_deref(),
            is_stream: validated.is_stream,
        },
    ) {
        crate::plugin_runtime::RequestHookOutcome::Continue(patch) => {
            validated.body = patch.body;
            validated.model_for_log = patch.model_for_log;
        }
        crate::plugin_runtime::RequestHookOutcome::Reject(reject) => {
            log::info!(
                "event=plugin_request_reject trace_id={} plugin_id={} plugin_name={} hook_point=pre_route status_code={}",
                validated.trace_id,
                reject.plugin_id,
                reject.plugin_name,
                reject.status_code
            );
            super::trace_log::log_request_final(
                validated.trace_id.as_str(),
                reject.status_code,
                None,
                None,
                Some(reject.message.as_str()),
                0,
            );
            super::record_gateway_request_outcome(
                validated.path.as_str(),
                reject.status_code,
                Some(validated.protocol_type.as_str()),
            );
            super::write_request_log(
                &validated.storage,
                super::request_log::RequestLogTraceContext {
                    trace_id: Some(validated.trace_id.as_str()),
                    original_path: Some(validated.original_path.as_str()),
                    adapted_path: Some(validated.path.as_str()),
                    response_adapter: Some(validated.response_adapter),
                },
                super::request_log::RequestLogEntry {
                    key_id: Some(validated.key_id.as_str()),
                    account_id: None,
                    request_path: validated.path.as_str(),
                    method: validated.request_method.as_str(),
                    model: validated.model_for_log.as_deref(),
                    reasoning_effort: validated.reasoning_for_log.as_deref(),
                    upstream_url: None,
                    status_code: Some(reject.status_code),
                    usage: super::request_log::RequestLogUsage::default(),
                    error: Some(reject.message.as_str()),
                    duration_ms: None,
                },
            );
            let response = super::error_response::json_value_response(
                reject.status_code,
                &reject.body,
                Some(validated.trace_id.as_str()),
            );
            let _ = request.respond(response);
            return Ok(());
        }
    }

    super::proxy_validated_request(request, validated, debug)
}
