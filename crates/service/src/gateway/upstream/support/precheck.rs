use codexmanager_core::storage::{Account, Storage, Token};
use tiny_http::Request;

pub(in super::super) enum CandidatePrecheckResult {
    Ready {
        request: Request,
        candidates: Vec<(Account, Token)>,
    },
    Responded,
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn prepare_candidates_for_proxy(
    request: Request,
    storage: &Storage,
    trace_id: &str,
    key_id: &str,
    original_path: &str,
    path: &str,
    response_adapter: super::super::super::ResponseAdapter,
    request_method: &str,
    model_for_log: Option<&str>,
    reasoning_for_log: Option<&str>,
) -> CandidatePrecheckResult {
    let candidates: Vec<(Account, Token)> =
        match super::candidates::prepare_gateway_candidates(storage, model_for_log) {
            Ok(v) => v,
            Err(err) => {
                let err_text = format!("candidate resolve failed: {err}");
                super::super::super::write_request_log(
                    storage,
                    super::super::super::request_log::RequestLogTraceContext {
                        trace_id: Some(trace_id),
                        original_path: Some(original_path),
                        adapted_path: Some(path),
                        response_adapter: Some(response_adapter),
                    },
                    super::super::super::request_log::RequestLogEntry {
                        key_id: Some(key_id),
                        account_id: None,
                        request_path: path,
                        method: request_method,
                        model: model_for_log,
                        reasoning_effort: reasoning_for_log,
                        upstream_url: None,
                        status_code: Some(500),
                        usage: super::super::super::request_log::RequestLogUsage::default(),
                        error: Some(err_text.as_str()),
                        duration_ms: None,
                    },
                );
                let response = super::super::super::error_response::terminal_text_response(
                    500,
                    err_text.clone(),
                    Some(trace_id),
                );
                let _ = request.respond(response);
                super::super::super::trace_log::log_request_final(
                    trace_id,
                    500,
                    None,
                    None,
                    Some(err_text.as_str()),
                    0,
                );
                return CandidatePrecheckResult::Responded;
            }
        };

    if candidates.is_empty() {
        super::super::super::write_request_log(
            storage,
            super::super::super::request_log::RequestLogTraceContext {
                trace_id: Some(trace_id),
                original_path: Some(original_path),
                adapted_path: Some(path),
                response_adapter: Some(response_adapter),
            },
            super::super::super::request_log::RequestLogEntry {
                key_id: Some(key_id),
                account_id: None,
                request_path: path,
                method: request_method,
                model: model_for_log,
                reasoning_effort: reasoning_for_log,
                upstream_url: None,
                status_code: Some(503),
                usage: super::super::super::request_log::RequestLogUsage::default(),
                error: Some("no available account"),
                duration_ms: None,
            },
        );
        let response = super::super::super::error_response::terminal_text_response(
            503,
            "no available account",
            Some(trace_id),
        );
        let _ = request.respond(response);
        super::super::super::trace_log::log_request_final(
            trace_id,
            503,
            None,
            None,
            Some("no available account"),
            0,
        );
        return CandidatePrecheckResult::Responded;
    }

    CandidatePrecheckResult::Ready {
        request,
        candidates,
    }
}
