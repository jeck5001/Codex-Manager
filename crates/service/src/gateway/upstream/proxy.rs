use crate::apikey_profile::{PROTOCOL_ANTHROPIC_NATIVE, PROTOCOL_AZURE_OPENAI};
use crate::gateway::request_log::RequestLogUsage;
use std::time::Instant;
use tiny_http::Request;

use super::super::local_validation::LocalValidationResult;
use super::proxy_pipeline::candidate_executor::{
    execute_candidate_sequence, CandidateExecutionResult, CandidateExecutorParams,
};
use super::proxy_pipeline::execution_context::{
    FinalResultLogArgs, GatewayUpstreamExecutionContext,
};
use super::proxy_pipeline::request_gate::acquire_request_gate;
use super::proxy_pipeline::request_setup::{prepare_request_setup, PrepareRequestSetupInput};
use super::proxy_pipeline::response_finalize::respond_terminal;
use super::support::precheck::{prepare_candidates_for_proxy, CandidatePrecheckResult};

fn normalize_model_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn build_model_attempt_chain(
    requested_model: Option<&str>,
    configured_chain: &[String],
) -> Vec<String> {
    let mut chain = Vec::new();
    if let Some(requested_model) = requested_model.and_then(normalize_model_name) {
        chain.push(requested_model);
    }
    for model in configured_chain {
        let Some(model) = normalize_model_name(model) else {
            continue;
        };
        if chain.iter().any(|item| item == &model) {
            continue;
        }
        chain.push(model);
    }
    chain
}

fn filter_model_attempt_chain_by_allowed_models(
    model_attempt_chain: Vec<String>,
    allowed_models: &[String],
    primary_model: Option<&str>,
    requested_model: Option<&str>,
) -> Vec<String> {
    if allowed_models.is_empty() {
        return model_attempt_chain;
    }

    let requested_model_allowed = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|model| allowed_models.iter().any(|allowed| allowed == model));
    let primary_model = primary_model
        .map(str::trim)
        .filter(|value| !value.is_empty());

    model_attempt_chain
        .into_iter()
        .filter(|model| {
            allowed_models.iter().any(|allowed| allowed == model)
                || requested_model_allowed && primary_model == Some(model.as_str())
        })
        .collect()
}

fn load_model_attempt_chain(
    storage: &codexmanager_core::storage::Storage,
    key_id: &str,
    primary_model: Option<&str>,
    requested_model: Option<&str>,
) -> Vec<String> {
    let allowed_models = storage
        .find_api_key_allowed_models_by_id(key_id)
        .ok()
        .flatten()
        .map(|raw| crate::apikey_allowed_models::parse_allowed_models(raw.as_str()))
        .unwrap_or_default();
    let configured_chain = storage
        .find_api_key_model_fallback_by_id(key_id)
        .ok()
        .flatten()
        .map(|config| crate::apikey_model_fallback::parse_model_chain(&config.model_chain_json))
        .unwrap_or_default();
    filter_model_attempt_chain_by_allowed_models(
        build_model_attempt_chain(primary_model, configured_chain.as_slice()),
        allowed_models.as_slice(),
        primary_model,
        requested_model,
    )
}

fn actual_model_header_value<'a>(
    requested_model: Option<&str>,
    actual_model: Option<&'a str>,
) -> Option<&'a str> {
    let actual_model = actual_model?;
    let normalized_actual = normalize_model_name(actual_model)?;
    let normalized_requested = requested_model.and_then(normalize_model_name)?;
    if Some(normalized_requested.as_str()) == Some(normalized_actual.as_str()) {
        None
    } else {
        Some(actual_model)
    }
}

fn build_api_key_response_cache_key(
    storage: &codexmanager_core::storage::Storage,
    key_id: &str,
    original_path: &str,
    body: &[u8],
    client_is_stream: bool,
) -> Option<String> {
    if client_is_stream {
        return None;
    }

    let enabled = match storage.find_api_key_response_cache_config_by_id(key_id) {
        Ok(Some(config)) => config.enabled,
        Ok(None) => false,
        Err(err) => {
            log::warn!(
                "event=gateway_response_cache_key_config_read_failed key_id={} error={}",
                key_id,
                err
            );
            false
        }
    };
    if !enabled {
        return None;
    }

    super::super::build_response_cache_key(original_path, body)
}

fn append_attempted_account_ids(target: &mut Vec<String>, source: &[String]) {
    for account_id in source {
        target.push(account_id.clone());
    }
}

fn exhausted_gateway_error_for_log(
    attempted_account_ids: &[String],
    skipped_cooldown: usize,
    skipped_inflight: usize,
    last_attempt_error: Option<&str>,
) -> String {
    let kind = if !attempted_account_ids.is_empty() {
        "no_available_account_exhausted"
    } else if skipped_cooldown > 0 && skipped_inflight > 0 {
        "no_available_account_skipped"
    } else if skipped_cooldown > 0 {
        "no_available_account_cooldown"
    } else if skipped_inflight > 0 {
        "no_available_account_inflight"
    } else {
        "no_available_account"
    };
    let mut parts = vec!["no available account".to_string(), format!("kind={kind}")];
    if !attempted_account_ids.is_empty() {
        parts.push(format!("attempted={}", attempted_account_ids.join(",")));
    }
    if skipped_cooldown > 0 || skipped_inflight > 0 {
        parts.push(format!(
            "skipped(cooldown={}, inflight={})",
            skipped_cooldown, skipped_inflight
        ));
    }
    if let Some(last_attempt_error) = last_attempt_error
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("last_attempt={last_attempt_error}"));
    }
    parts.join("; ")
}

pub(in super::super) fn proxy_validated_request(
    request: Request,
    validated: LocalValidationResult,
    debug: bool,
) -> Result<(), String> {
    let LocalValidationResult {
        trace_id,
        incoming_headers,
        storage,
        original_path,
        path,
        body,
        is_stream,
        has_prompt_cache_key,
        request_shape,
        protocol_type,
        upstream_base_url,
        static_headers_json,
        response_adapter,
        tool_name_restore_map,
        request_method,
        key_id,
        api_key_name,
        requested_model_for_log,
        model_for_log,
        reasoning_for_log,
        method,
    } = validated;
    let started_at = Instant::now();
    let client_is_stream = is_stream;
    let requested_model = requested_model_for_log.clone();
    let is_compact_path =
        path == "/v1/responses/compact" || path.starts_with("/v1/responses/compact?");
    // 中文注释：对齐 CPA：/v1/responses 上游固定走 SSE。
    // 下游是否流式仍由客户端 `stream` 参数决定（在 response bridge 层聚合/透传）。
    let upstream_is_stream =
        client_is_stream || (path.starts_with("/v1/responses") && !is_compact_path);
    let request_deadline = super::support::deadline::request_deadline(started_at, client_is_stream);
    let response_cache_key = build_api_key_response_cache_key(
        &storage,
        key_id.as_str(),
        original_path.as_str(),
        body.as_ref(),
        client_is_stream,
    );

    super::super::trace_log::log_request_start(super::super::trace_log::RequestStartLog {
        trace_id: trace_id.as_str(),
        key_id: key_id.as_str(),
        method: request_method.as_str(),
        path: path.as_str(),
        model: model_for_log.as_deref(),
        reasoning: reasoning_for_log.as_deref(),
        is_stream: client_is_stream,
        protocol_type: protocol_type.as_str(),
    });
    super::super::trace_log::log_request_body_preview(trace_id.as_str(), body.as_ref());

    if let Some(cache_key) = response_cache_key.as_deref() {
        if let Some(cached) = super::super::response_cache::lookup_response_cache(cache_key) {
            let context = GatewayUpstreamExecutionContext::new(
                &trace_id,
                &storage,
                &key_id,
                &original_path,
                &path,
                &request_method,
                response_adapter,
                protocol_type.as_str(),
                cached.actual_model.as_deref().or(model_for_log.as_deref()),
                requested_model.as_deref(),
                reasoning_for_log.as_deref(),
                None,
                0,
                super::super::account_max_inflight_limit(),
            );
            context.log_final_result_with_model(FinalResultLogArgs {
                final_account_id: None,
                upstream_url: Some("cache://response-cache"),
                model_for_log: cached.actual_model.as_deref().or(model_for_log.as_deref()),
                status_code: cached.status_code,
                usage: cached.usage,
                error: None,
                elapsed_ms: started_at.elapsed().as_millis(),
                attempted_account_ids: None,
                skipped_cooldown_count: 0,
                skipped_inflight_count: 0,
            });
            super::super::response_cache::respond_with_cached_response(
                request,
                trace_id.as_str(),
                &cached,
            )?;
            return Ok(());
        }
    }

    if protocol_type == PROTOCOL_AZURE_OPENAI {
        return super::protocol::azure_openai::proxy_azure_request(
            request,
            &storage,
            trace_id.as_str(),
            key_id.as_str(),
            original_path.as_str(),
            path.as_str(),
            request_method.as_str(),
            &method,
            &body,
            upstream_is_stream,
            response_adapter,
            &tool_name_restore_map,
            model_for_log.as_deref(),
            reasoning_for_log.as_deref(),
            upstream_base_url.as_deref(),
            static_headers_json.as_deref(),
            requested_model.as_deref(),
            actual_model_header_value(requested_model.as_deref(), model_for_log.as_deref()),
            request_deadline,
            started_at,
        );
    }

    let (request, candidates) = match prepare_candidates_for_proxy(
        request,
        &storage,
        trace_id.as_str(),
        &key_id,
        &original_path,
        &path,
        response_adapter,
        &request_method,
        model_for_log.as_deref(),
        reasoning_for_log.as_deref(),
    ) {
        CandidatePrecheckResult::Ready {
            request,
            candidates,
        } => (request, candidates),
        CandidatePrecheckResult::Responded => return Ok(()),
    };
    let base = super::config::resolve_upstream_base_url();
    let base_candidates = candidates;
    let primary_model = model_for_log.as_deref();
    let model_attempt_chain =
        load_model_attempt_chain(&storage, &key_id, primary_model, requested_model.as_deref());
    let model_attempt_count = model_attempt_chain.len().max(1);
    let allow_openai_fallback = false;
    let disable_challenge_stateless_retry = !(path.starts_with("/v1/responses")
        || protocol_type == PROTOCOL_ANTHROPIC_NATIVE && body.len() <= 2 * 1024);
    let _request_gate_guard = acquire_request_gate(
        trace_id.as_str(),
        key_id.as_str(),
        path.as_str(),
        model_for_log.as_deref(),
        request_deadline,
    );
    let mut request = request;
    let mut attempted_account_ids_all = Vec::new();
    let mut skipped_cooldown_total = 0usize;
    let mut skipped_inflight_total = 0usize;
    let mut last_attempt_url = None;
    let mut last_attempt_error = None;
    for model_idx in 0..model_attempt_count {
        let current_model_for_log = model_attempt_chain
            .get(model_idx)
            .map(String::as_str)
            .or(primary_model)
            .or(requested_model.as_deref());
        let model_fallback_path = (model_attempt_chain.len() > 1
            && model_idx < model_attempt_chain.len())
        .then_some(&model_attempt_chain[..=model_idx]);
        let mut candidates = base_candidates.clone();
        let setup = prepare_request_setup(
            PrepareRequestSetupInput {
                path: path.as_str(),
                protocol_type: protocol_type.as_str(),
                has_prompt_cache_key,
                incoming_headers: &incoming_headers,
                body: body.as_ref(),
                key_id: key_id.as_str(),
                model_for_log: current_model_for_log,
                trace_id: trace_id.as_str(),
            },
            candidates.as_mut_slice(),
        );
        let context = GatewayUpstreamExecutionContext::new(
            &trace_id,
            &storage,
            &key_id,
            &original_path,
            &path,
            &request_method,
            response_adapter,
            protocol_type.as_str(),
            current_model_for_log,
            requested_model.as_deref(),
            reasoning_for_log.as_deref(),
            model_fallback_path.map(|items| items as &[String]),
            setup.candidate_count,
            setup.account_max_inflight,
        );
        let has_more_models = model_idx + 1 < model_attempt_chain.len();
        match execute_candidate_sequence(
            request,
            candidates,
            CandidateExecutorParams {
                storage: &storage,
                method: &method,
                incoming_headers: &incoming_headers,
                body: &body,
                path: path.as_str(),
                key_id: key_id.as_str(),
                api_key_name: api_key_name.as_deref(),
                request_shape: request_shape.as_deref(),
                trace_id: trace_id.as_str(),
                model_for_log: current_model_for_log,
                request_model_override: current_model_for_log,
                response_adapter,
                tool_name_restore_map: &tool_name_restore_map,
                context: &context,
                setup: &setup,
                request_deadline,
                started_at,
                client_is_stream,
                upstream_is_stream,
                actual_model_header: actual_model_header_value(
                    requested_model.as_deref(),
                    current_model_for_log,
                ),
                response_cache_key: response_cache_key.as_deref(),
                has_more_models,
                debug,
                allow_openai_fallback,
                disable_challenge_stateless_retry,
            },
        )? {
            CandidateExecutionResult::Handled => return Ok(()),
            CandidateExecutionResult::Exhausted {
                request: returned_request,
                attempted_account_ids,
                skipped_cooldown,
                skipped_inflight,
                last_attempt_url: current_last_attempt_url,
                last_attempt_error: current_last_attempt_error,
            } => {
                request = *returned_request;
                append_attempted_account_ids(
                    &mut attempted_account_ids_all,
                    attempted_account_ids.as_slice(),
                );
                skipped_cooldown_total += skipped_cooldown;
                skipped_inflight_total += skipped_inflight;
                last_attempt_url = current_last_attempt_url;
                last_attempt_error = current_last_attempt_error;

                if has_more_models {
                    log::warn!(
                        "event=gateway_model_fallback trace_id={} key_id={} requested_model={} next_model={}",
                        trace_id,
                        key_id,
                        current_model_for_log.unwrap_or("-"),
                        model_attempt_chain
                            .get(model_idx + 1)
                            .map(String::as_str)
                            .unwrap_or("-"),
                    );
                }
            }
        }
    }
    let final_error = exhausted_gateway_error_for_log(
        attempted_account_ids_all.as_slice(),
        skipped_cooldown_total,
        skipped_inflight_total,
        last_attempt_error.as_deref(),
    );

    let final_model_for_log = model_attempt_chain
        .last()
        .map(String::as_str)
        .or(primary_model)
        .or(requested_model.as_deref());
    let final_context = GatewayUpstreamExecutionContext::new(
        &trace_id,
        &storage,
        &key_id,
        &original_path,
        &path,
        &request_method,
        response_adapter,
        protocol_type.as_str(),
        final_model_for_log,
        requested_model.as_deref(),
        reasoning_for_log.as_deref(),
        (model_attempt_chain.len() > 1).then_some(model_attempt_chain.as_slice()),
        base_candidates.len(),
        crate::gateway::runtime_config::account_max_inflight_limit(),
    );

    final_context.log_final_result(FinalResultLogArgs {
        final_account_id: None,
        upstream_url: last_attempt_url.as_deref().or(Some(base.as_str())),
        model_for_log: None,
        status_code: 503,
        usage: RequestLogUsage::default(),
        error: Some(final_error.as_str()),
        elapsed_ms: started_at.elapsed().as_millis(),
        attempted_account_ids: (!attempted_account_ids_all.is_empty())
            .then_some(attempted_account_ids_all.as_slice()),
        skipped_cooldown_count: skipped_cooldown_total,
        skipped_inflight_count: skipped_inflight_total,
    });
    respond_terminal(
        request,
        503,
        "no available account".to_string(),
        Some(trace_id.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        actual_model_header_value, build_model_attempt_chain, exhausted_gateway_error_for_log,
        filter_model_attempt_chain_by_allowed_models,
    };

    #[test]
    fn exhausted_gateway_error_includes_attempts_skips_and_last_error() {
        let message = exhausted_gateway_error_for_log(
            &["acc-a".to_string(), "acc-b".to_string()],
            2,
            1,
            Some("upstream challenge blocked"),
        );

        assert!(message.contains("no available account"));
        assert!(message.contains("kind=no_available_account_exhausted"));
        assert!(message.contains("attempted=acc-a,acc-b"));
        assert!(message.contains("skipped(cooldown=2, inflight=1)"));
        assert!(message.contains("last_attempt=upstream challenge blocked"));
    }

    #[test]
    fn exhausted_gateway_error_marks_cooldown_only_skip_kind() {
        let message = exhausted_gateway_error_for_log(&[], 2, 0, None);

        assert!(message.contains("kind=no_available_account_cooldown"));
    }

    #[test]
    fn build_model_attempt_chain_keeps_requested_model_first_and_dedupes() {
        let chain = build_model_attempt_chain(
            Some("o3"),
            &[
                "o3".to_string(),
                "o4-mini".to_string(),
                "gpt-4o".to_string(),
                "o4-mini".to_string(),
            ],
        );

        assert_eq!(chain, vec!["o3", "o4-mini", "gpt-4o"]);
    }

    #[test]
    fn filter_model_attempt_chain_by_allowed_models_removes_disallowed_fallbacks() {
        let filtered = filter_model_attempt_chain_by_allowed_models(
            vec![
                "o3".to_string(),
                "o4-mini".to_string(),
                "gpt-4o".to_string(),
            ],
            &["o3".to_string(), "gpt-4o".to_string()],
            Some("o3"),
            Some("o3"),
        );

        assert_eq!(filtered, vec!["o3", "gpt-4o"]);
    }

    #[test]
    fn filter_model_attempt_chain_keeps_alias_selected_primary_model() {
        let filtered = filter_model_attempt_chain_by_allowed_models(
            vec!["o3".to_string(), "o4-mini".to_string()],
            &["o3-auto".to_string()],
            Some("o3"),
            Some("o3-auto"),
        );

        assert_eq!(filtered, vec!["o3"]);
    }

    #[test]
    fn actual_model_header_value_only_returns_when_model_changes() {
        assert_eq!(
            actual_model_header_value(Some("o3-auto"), Some("o3")),
            Some("o3")
        );
        assert_eq!(actual_model_header_value(Some("o3"), Some("o3")), None);
        assert_eq!(actual_model_header_value(None, Some("o3")), None);
    }
}
