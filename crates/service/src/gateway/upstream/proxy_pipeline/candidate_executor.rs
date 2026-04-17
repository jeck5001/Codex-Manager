use bytes::Bytes;
use codexmanager_core::storage::{Account, Storage, Token};
use std::time::Instant;
use tiny_http::Request;

use super::super::attempt_flow::candidate_flow::CandidateUpstreamDecision;
use super::super::attempt_flow::transport::UpstreamRequestContext;
use super::super::support::candidates::free_account_model_override;
use super::super::support::deadline;
use super::candidate_attempt::{
    run_candidate_attempt, CandidateAttemptParams, CandidateAttemptTrace,
};
use super::candidate_state::CandidateExecutionState;
use super::execution_context::{FinalResultLogArgs, GatewayUpstreamExecutionContext};
use super::request_setup::UpstreamRequestSetup;
use super::response_finalize::{
    finalize_terminal_candidate, finalize_upstream_response, respond_total_timeout,
    FinalizeUpstreamResponseArgs, TerminalCandidateArgs,
};

fn extract_prompt_cache_key_for_trace(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > 64 * 1024 {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(body).ok()?;
    value
        .get("prompt_cache_key")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(in super::super) enum CandidateExecutionResult {
    Handled,
    Exhausted {
        request: Box<Request>,
        attempted_account_ids: Vec<String>,
        skipped_cooldown: usize,
        skipped_inflight: usize,
        last_attempt_url: Option<String>,
        last_attempt_error: Option<String>,
    },
}

pub(in super::super) struct CandidateExecutorParams<'a> {
    pub(in super::super) storage: &'a Storage,
    pub(in super::super) method: &'a reqwest::Method,
    pub(in super::super) incoming_headers: &'a super::super::super::IncomingHeaderSnapshot,
    pub(in super::super) body: &'a Bytes,
    pub(in super::super) path: &'a str,
    pub(in super::super) key_id: &'a str,
    pub(in super::super) api_key_name: Option<&'a str>,
    pub(in super::super) request_shape: Option<&'a str>,
    pub(in super::super) trace_id: &'a str,
    pub(in super::super) model_for_log: Option<&'a str>,
    pub(in super::super) request_model_override: Option<&'a str>,
    pub(in super::super) response_adapter: super::super::super::ResponseAdapter,
    pub(in super::super) tool_name_restore_map: &'a super::super::super::ToolNameRestoreMap,
    pub(in super::super) context: &'a GatewayUpstreamExecutionContext<'a>,
    pub(in super::super) setup: &'a UpstreamRequestSetup,
    pub(in super::super) request_deadline: Option<Instant>,
    pub(in super::super) started_at: Instant,
    pub(in super::super) client_is_stream: bool,
    pub(in super::super) upstream_is_stream: bool,
    pub(in super::super) actual_model_header: Option<&'a str>,
    pub(in super::super) response_cache_key: Option<&'a str>,
    pub(in super::super) has_more_models: bool,
    pub(in super::super) debug: bool,
    pub(in super::super) allow_openai_fallback: bool,
    pub(in super::super) disable_challenge_stateless_retry: bool,
}

pub(in super::super) fn execute_candidate_sequence(
    request: Request,
    candidates: Vec<(Account, Token)>,
    params: CandidateExecutorParams<'_>,
) -> Result<CandidateExecutionResult, String> {
    let CandidateExecutorParams {
        storage,
        method,
        incoming_headers,
        body,
        path,
        key_id,
        api_key_name,
        request_shape,
        trace_id,
        model_for_log,
        request_model_override,
        response_adapter,
        tool_name_restore_map,
        context,
        setup,
        request_deadline,
        started_at,
        client_is_stream,
        upstream_is_stream,
        actual_model_header,
        response_cache_key,
        has_more_models,
        debug,
        allow_openai_fallback,
        disable_challenge_stateless_retry,
    } = params;
    let mut request = Some(request);
    let mut state = CandidateExecutionState::default();
    let mut attempted_account_ids = Vec::new();
    let mut skipped_cooldown = 0usize;
    let mut skipped_inflight = 0usize;
    let mut last_attempt_url = None;
    let mut last_attempt_error = None;
    let retry_limit = super::super::super::retry_policy_max_retries();
    let mut failover_attempts = 0usize;
    for (idx, (account, mut token)) in candidates.into_iter().enumerate() {
        if deadline::is_expired(request_deadline) {
            let request = request
                .take()
                .expect("request should be available before timeout response");
            respond_total_timeout(
                request,
                context,
                trace_id,
                started_at,
                model_for_log,
                Some(attempted_account_ids.as_slice()),
                skipped_cooldown,
                skipped_inflight,
            )?;
            return Ok(CandidateExecutionResult::Handled);
        }

        let strip_session_affinity =
            state.strip_session_affinity(&account, idx, setup.anthropic_has_prompt_cache_key);
        let attempt_model_override = free_account_model_override(storage, &account, &token);
        let mut effective_model_override = attempt_model_override
            .as_deref()
            .or(request_model_override)
            .map(str::to_string);
        let mut attempt_model_for_log = effective_model_override
            .clone()
            .or_else(|| model_for_log.map(str::to_string));
        let mut body_for_attempt = state.body_for_attempt(
            path,
            body,
            strip_session_affinity,
            setup,
            effective_model_override.as_deref(),
        );
        context.log_candidate_start(&account.id, idx, strip_session_affinity);
        if let Some(skip_reason) = context.should_skip_candidate(&account.id, idx) {
            context.log_candidate_skip(&account.id, idx, skip_reason);
            match skip_reason {
                super::super::support::candidates::CandidateSkipReason::Cooldown => {
                    skipped_cooldown += 1;
                }
                super::super::support::candidates::CandidateSkipReason::Inflight => {
                    skipped_inflight += 1;
                }
            }
            continue;
        }
        attempted_account_ids.push(account.id.clone());

        let request_ref = request
            .as_ref()
            .ok_or_else(|| "request already consumed".to_string())?;
        let request_ctx = UpstreamRequestContext::from_request(request_ref);
        let incoming_session_id = incoming_headers.session_id();
        let incoming_turn_state = incoming_headers.turn_state();
        let incoming_conversation_id = incoming_headers.conversation_id();
        match crate::plugin_runtime::execute_post_route_plugins(
            crate::plugin_runtime::PostRoutePluginInput {
                storage,
                trace_id,
                key_id,
                api_key_name,
                path,
                method: method.as_str(),
                body: &body_for_attempt,
                model_for_log: attempt_model_for_log.as_deref(),
                is_stream: client_is_stream,
                account: &account,
                route_strategy: super::super::super::current_route_strategy(),
            },
        ) {
            crate::plugin_runtime::RequestHookOutcome::Continue(patch) => {
                body_for_attempt = patch.body;
                attempt_model_for_log = patch.model_for_log;
                effective_model_override = attempt_model_for_log.clone();
            }
            crate::plugin_runtime::RequestHookOutcome::Reject(reject) => {
                let request = request
                    .take()
                    .expect("request should be available before plugin reject response");
                log::info!(
                    "event=plugin_request_reject trace_id={} plugin_id={} plugin_name={} hook_point=post_route status_code={} account_id={}",
                    trace_id,
                    reject.plugin_id,
                    reject.plugin_name,
                    reject.status_code,
                    account.id
                );
                context.log_final_result_with_model(FinalResultLogArgs {
                    final_account_id: Some(&account.id),
                    upstream_url: None,
                    model_for_log: attempt_model_for_log.as_deref(),
                    status_code: reject.status_code,
                    usage: super::super::super::request_log::RequestLogUsage::default(),
                    error: Some(reject.message.as_str()),
                    elapsed_ms: started_at.elapsed().as_millis(),
                    attempted_account_ids: Some(attempted_account_ids.as_slice()),
                    skipped_cooldown_count: skipped_cooldown,
                    skipped_inflight_count: skipped_inflight,
                });
                let response = super::super::super::error_response::json_value_response(
                    reject.status_code,
                    &reject.body,
                    Some(trace_id),
                );
                let _ = request.respond(response);
                return Ok(CandidateExecutionResult::Handled);
            }
        }
        let prompt_cache_key_for_trace =
            extract_prompt_cache_key_for_trace(body_for_attempt.as_ref());
        super::super::super::trace_log::log_attempt_profile(
            trace_id,
            &account.id,
            idx,
            setup.candidate_count,
            strip_session_affinity,
            incoming_session_id.is_some() || setup.has_sticky_fallback_session,
            incoming_turn_state.is_some(),
            incoming_conversation_id.is_some() || setup.has_sticky_fallback_conversation,
            prompt_cache_key_for_trace.as_deref(),
            request_shape,
            body_for_attempt.len(),
            attempt_model_for_log.as_deref(),
        );

        let mut inflight_guard = Some(super::super::super::acquire_account_inflight(&account.id));
        let mut attempt_trace = CandidateAttemptTrace::default();
        let has_retry_budget = failover_attempts < retry_limit;
        let has_more_candidates =
            has_retry_budget && (context.has_more_candidates(idx) || has_more_models);
        let decision = run_candidate_attempt(CandidateAttemptParams {
            storage,
            method,
            request_ctx,
            incoming_headers,
            body: &body_for_attempt,
            upstream_is_stream,
            path,
            request_deadline,
            account: &account,
            token: &mut token,
            strip_session_affinity,
            debug,
            allow_openai_fallback,
            disable_challenge_stateless_retry,
            has_more_candidates,
            context,
            setup,
            trace: &mut attempt_trace,
        });

        match decision {
            CandidateUpstreamDecision::Failover => {
                super::super::super::record_gateway_failover_attempt();
                last_attempt_url = attempt_trace.last_attempt_url.take();
                last_attempt_error = attempt_trace.last_attempt_error.take();
                failover_attempts = failover_attempts.saturating_add(1);
                if !super::super::super::sleep_before_retry(
                    failover_attempts.saturating_sub(1),
                    request_deadline,
                ) {
                    let request = request
                        .take()
                        .expect("request should be available before timeout response");
                    respond_total_timeout(
                        request,
                        context,
                        trace_id,
                        started_at,
                        model_for_log,
                        Some(attempted_account_ids.as_slice()),
                        skipped_cooldown,
                        skipped_inflight,
                    )?;
                    return Ok(CandidateExecutionResult::Handled);
                }
                continue;
            }
            CandidateUpstreamDecision::Terminal {
                status_code,
                message,
            } => {
                let request = request
                    .take()
                    .expect("request should be available before terminal response");
                finalize_terminal_candidate(TerminalCandidateArgs {
                    request,
                    context,
                    account_id: &account.id,
                    last_attempt_url: attempt_trace.last_attempt_url.as_deref(),
                    status_code,
                    message,
                    trace_id,
                    started_at,
                    model_for_log: attempt_model_for_log.as_deref(),
                    attempted_account_ids: Some(attempted_account_ids.as_slice()),
                    skipped_cooldown_count: skipped_cooldown,
                    skipped_inflight_count: skipped_inflight,
                })?;
                return Ok(CandidateExecutionResult::Handled);
            }
            CandidateUpstreamDecision::RespondUpstream(mut resp) => {
                if resp.status().as_u16() == 400
                    && !strip_session_affinity
                    && (incoming_turn_state.is_some() || setup.has_body_encrypted_content)
                {
                    let retry_body = state.retry_body(path, &body_for_attempt, setup, None);
                    let retry_decision = run_candidate_attempt(CandidateAttemptParams {
                        storage,
                        method,
                        request_ctx,
                        incoming_headers,
                        body: &retry_body,
                        upstream_is_stream,
                        path,
                        request_deadline,
                        account: &account,
                        token: &mut token,
                        strip_session_affinity: true,
                        debug,
                        allow_openai_fallback,
                        disable_challenge_stateless_retry,
                        has_more_candidates,
                        context,
                        setup,
                        trace: &mut attempt_trace,
                    });

                    match retry_decision {
                        CandidateUpstreamDecision::RespondUpstream(retry_resp) => {
                            resp = retry_resp;
                        }
                        CandidateUpstreamDecision::Failover => {
                            super::super::super::record_gateway_failover_attempt();
                            last_attempt_url = attempt_trace.last_attempt_url.take();
                            last_attempt_error = attempt_trace.last_attempt_error.take();
                            failover_attempts = failover_attempts.saturating_add(1);
                            if !super::super::super::sleep_before_retry(
                                failover_attempts.saturating_sub(1),
                                request_deadline,
                            ) {
                                let request = request
                                    .take()
                                    .expect("request should be available before timeout response");
                                respond_total_timeout(
                                    request,
                                    context,
                                    trace_id,
                                    started_at,
                                    model_for_log,
                                    Some(attempted_account_ids.as_slice()),
                                    skipped_cooldown,
                                    skipped_inflight,
                                )?;
                                return Ok(CandidateExecutionResult::Handled);
                            }
                            continue;
                        }
                        CandidateUpstreamDecision::Terminal {
                            status_code,
                            message,
                        } => {
                            let request = request
                                .take()
                                .expect("request should be available before terminal response");
                            finalize_terminal_candidate(TerminalCandidateArgs {
                                request,
                                context,
                                account_id: &account.id,
                                last_attempt_url: attempt_trace.last_attempt_url.as_deref(),
                                status_code,
                                message,
                                trace_id,
                                started_at,
                                model_for_log: attempt_model_for_log.as_deref(),
                                attempted_account_ids: Some(attempted_account_ids.as_slice()),
                                skipped_cooldown_count: skipped_cooldown,
                                skipped_inflight_count: skipped_inflight,
                            })?;
                            return Ok(CandidateExecutionResult::Handled);
                        }
                    }
                }
                let request = request
                    .take()
                    .expect("request should be available before terminal response");
                let guard = inflight_guard
                    .take()
                    .expect("inflight guard should be available before terminal response");
                crate::plugin_runtime::execute_post_response_plugins(
                    crate::plugin_runtime::PostResponsePluginInput {
                        storage,
                        trace_id,
                        key_id,
                        api_key_name,
                        path,
                        method: method.as_str(),
                        body: &body_for_attempt,
                        model_for_log: attempt_model_for_log.as_deref(),
                        is_stream: client_is_stream,
                        account: &account,
                        route_strategy: super::super::super::current_route_strategy(),
                        status_code: resp.status().as_u16(),
                        response_headers: resp.headers(),
                    },
                );
                finalize_upstream_response(FinalizeUpstreamResponseArgs {
                    request,
                    response: resp,
                    inflight_guard: guard,
                    context,
                    account_id: &account.id,
                    last_attempt_url: attempt_trace.last_attempt_url.as_deref(),
                    last_attempt_error: attempt_trace.last_attempt_error.as_deref(),
                    response_adapter,
                    tool_name_restore_map,
                    client_is_stream,
                    path,
                    trace_id,
                    started_at,
                    actual_model_header,
                    model_for_log: attempt_model_for_log.as_deref(),
                    response_cache_key,
                    attempted_account_ids: Some(attempted_account_ids.as_slice()),
                    skipped_cooldown_count: skipped_cooldown,
                    skipped_inflight_count: skipped_inflight,
                })?;
                return Ok(CandidateExecutionResult::Handled);
            }
        }
    }

    Ok(CandidateExecutionResult::Exhausted {
        request: Box::new(
            request.expect("request should still exist when no candidate handled the response"),
        ),
        attempted_account_ids,
        skipped_cooldown,
        skipped_inflight,
        last_attempt_url,
        last_attempt_error,
    })
}
