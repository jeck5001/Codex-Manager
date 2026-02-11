use tiny_http::{Request, Response};

use super::local_validation::LocalValidationResult;
use super::upstream_candidate_flow::{process_candidate_upstream_flow, CandidateUpstreamDecision};
use super::upstream_precheck::{prepare_candidates_for_proxy, CandidatePrecheckResult};

fn respond_terminal(request: Request, status_code: u16, message: String) -> Result<(), String> {
    let response = Response::from_string(message).with_status_code(status_code);
    let _ = request.respond(response);
    Ok(())
}

pub(super) fn proxy_validated_request(
    request: Request,
    validated: LocalValidationResult,
    debug: bool,
) -> Result<(), String> {
    let LocalValidationResult {
        storage,
        path,
        body,
        request_method,
        key_id,
        model_for_log,
        reasoning_for_log,
        method,
    } = validated;

    let (request, candidates) = match prepare_candidates_for_proxy(
        request,
        &storage,
        &key_id,
        &path,
        &request_method,
        model_for_log.as_deref(),
        reasoning_for_log.as_deref(),
    ) {
        CandidatePrecheckResult::Ready { request, candidates } => (request, candidates),
        CandidatePrecheckResult::Responded => return Ok(()),
    };
    let upstream_base = super::resolve_upstream_base_url();
    let base = upstream_base.as_str();
    let upstream_fallback_base = super::resolve_upstream_fallback_base_url(base);
    let (url, url_alt) = super::request_rewrite::compute_upstream_url(base, &path);

    let client = super::upstream_client();
    let upstream_cookie = std::env::var("GPTTOOLS_UPSTREAM_COOKIE").ok();

    let candidate_count = candidates.len();
    let account_max_inflight = super::account_max_inflight_limit();
    let has_more_candidates = |idx: usize| idx + 1 < candidate_count;
    let mut log_gateway_result =
        |upstream_url: Option<&str>, status_code: u16, error: Option<&str>| {
            super::write_request_log(
                &storage,
                Some(&key_id),
                &path,
                &request_method,
                model_for_log.as_deref(),
                reasoning_for_log.as_deref(),
                upstream_url,
                Some(status_code),
                error,
            );
        };
    for (idx, (account, mut token)) in candidates.into_iter().enumerate() {
        let strip_session_affinity = idx > 0;
        if super::upstream_candidates::should_skip_candidate_for_proxy(
            &account.id,
            idx,
            candidate_count,
            account_max_inflight,
        ) {
            continue;
        }
        // 中文注释：把 inflight 计数覆盖到整个响应生命周期，确保下一批请求能看到真实负载。
        let inflight_guard = super::acquire_account_inflight(&account.id);

        match process_candidate_upstream_flow(
            &client,
            &storage,
            &method,
            &request,
            &body,
            base,
            &path,
            url.as_str(),
            url_alt.as_deref(),
            upstream_fallback_base.as_deref(),
            &account,
            &mut token,
            upstream_cookie.as_deref(),
            strip_session_affinity,
            debug,
            has_more_candidates(idx),
            &mut log_gateway_result,
        ) {
            CandidateUpstreamDecision::RespondUpstream(resp) => {
                return super::respond_with_upstream(request, resp, inflight_guard);
            }
            CandidateUpstreamDecision::Failover => {
                super::record_gateway_failover_attempt();
                continue;
            }
            CandidateUpstreamDecision::Terminal {
                status_code,
                message,
            } => {
                return respond_terminal(request, status_code, message);
            }
        }
    }

    log_gateway_result(Some(base), 503, Some("no available account"));
    respond_terminal(request, 503, "no available account".to_string())
}
