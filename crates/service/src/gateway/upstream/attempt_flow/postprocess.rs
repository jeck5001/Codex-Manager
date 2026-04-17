use bytes::Bytes;
use codexmanager_core::storage::{Account, Storage, Token};
use std::time::Instant;

use crate::account_status::mark_account_unavailable_for_refresh_token_error;

use super::super::support::outcome::{decide_upstream_outcome, UpstreamOutcomeDecision};
use super::super::support::retry::{retry_with_alternate_path, AltPathRetryResult};
use super::fallback_branch::{handle_openai_fallback_branch, FallbackBranchResult};
use super::stateless_retry::{retry_stateless_then_optional_alt, StatelessRetryResult};
use super::transport::{SendUpstreamRequestArgs, UpstreamRequestContext};

fn try_refresh_chatgpt_access_token(
    storage: &Storage,
    upstream_base: &str,
    account: &Account,
    token: &mut Token,
) -> Result<Option<String>, String> {
    if super::super::super::is_openai_api_base(upstream_base) {
        return Ok(None);
    }
    if !has_chatgpt_recovery_credentials(account, token) {
        return Ok(None);
    }
    let issuer = if account.issuer.trim().is_empty() {
        super::super::super::runtime_config::token_exchange_default_issuer()
    } else {
        account.issuer.clone()
    };
    let client_id = super::super::super::runtime_config::token_exchange_client_id();
    crate::auth_account::refresh_chatgpt_auth_tokens_with_fallback(
        storage,
        account,
        token,
        issuer.as_str(),
        client_id.as_str(),
    )?;
    let refreshed = token.access_token.trim();
    if refreshed.is_empty() {
        return Err("refreshed chatgpt access token is empty".to_string());
    }
    Ok(Some(refreshed.to_string()))
}

fn has_chatgpt_recovery_credentials(account: &Account, token: &Token) -> bool {
    !token.refresh_token.trim().is_empty()
        || crate::account_payment::read_account_cookies(&account.id)
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
}

fn is_session_cookie_refresh_auth_error(err: &str) -> bool {
    err.contains("session cookie refresh failed: usage endpoint failed: status=401")
        || err.contains("session cookie refresh failed: usage endpoint failed: status=403")
}

fn should_failover_for_identity_error(
    storage: &Storage,
    account_id: &str,
    upstream: &reqwest::blocking::Response,
) -> bool {
    let Some(identity_error_code) =
        crate::gateway::extract_identity_error_code_from_headers(upstream.headers())
    else {
        return false;
    };
    crate::account_status::mark_account_unavailable_for_identity_error(
        storage,
        account_id,
        &identity_error_code,
    )
}

pub(super) enum PostRetryFlowDecision {
    Failover,
    Terminal { status_code: u16, message: String },
    RespondUpstream(reqwest::blocking::Response),
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_upstream_post_retry_flow<F>(
    client: &reqwest::blocking::Client,
    storage: &Storage,
    method: &reqwest::Method,
    upstream_base: &str,
    path: &str,
    url: &str,
    url_alt: Option<&str>,
    request_deadline: Option<Instant>,
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    body: &Bytes,
    is_stream: bool,
    upstream_cookie: Option<&str>,
    auth_token: &str,
    account: &Account,
    token: &mut Token,
    upstream_fallback_base: Option<&str>,
    strip_session_affinity: bool,
    debug: bool,
    allow_openai_fallback: bool,
    disable_challenge_stateless_retry: bool,
    has_more_candidates: bool,
    mut upstream: reqwest::blocking::Response,
    mut log_gateway_result: F,
) -> PostRetryFlowDecision
where
    F: FnMut(Option<&str>, u16, Option<&str>),
{
    let mut current_auth_token = auth_token.to_string();
    let mut status = upstream.status();
    // 中文注释：CPA 无 cookie 兼容模式下尽量保持“单跳上游”语义，避免多次 retry 反而触发 challenge。
    let compact_no_cookie_mode =
        super::super::super::cpa_no_cookie_header_mode_enabled() && upstream_cookie.is_none();
    if !status.is_success() {
        log::warn!(
            "gateway upstream non-success: status={}, account_id={}",
            status,
            account.id
        );
    }

    if !compact_no_cookie_mode {
        let has_identity_failover_marker =
            status.as_u16() == 401 && should_failover_for_identity_error(storage, &account.id, &upstream);
        if has_identity_failover_marker && has_more_candidates {
            log_gateway_result(Some(url), 401, Some("identity token invalidated failover"));
            return PostRetryFlowDecision::Failover;
        }
        let allow_unauthorized_stateless_retry = status.as_u16() == 401
            && !has_chatgpt_recovery_credentials(account, token)
            && !has_identity_failover_marker;
        if status.as_u16() == 401 {
            match try_refresh_chatgpt_access_token(storage, upstream_base, account, token) {
                Ok(Some(refreshed_auth_token)) => {
                    current_auth_token = refreshed_auth_token;
                    if debug {
                        log::warn!(
                            "event=gateway_upstream_unauthorized_refresh_retry path={} account_id={}",
                            path,
                            account.id
                        );
                    }
                    match super::transport::send_upstream_request(SendUpstreamRequestArgs {
                        client,
                        method,
                        target_url: url,
                        request_deadline,
                        request_ctx,
                        incoming_headers,
                        body,
                        is_stream,
                        upstream_cookie,
                        auth_token: current_auth_token.as_str(),
                        account,
                        strip_session_affinity,
                    }) {
                        Ok(resp) => {
                            upstream = resp;
                            status = upstream.status();
                        }
                        Err(err) => {
                            log::warn!(
                                "event=gateway_upstream_unauthorized_refresh_retry_error path={} status=502 account_id={} err={}",
                                path,
                                account.id,
                                err
                            );
                        }
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    let refresh_token_invalid = mark_account_unavailable_for_refresh_token_error(
                        storage,
                        &account.id,
                        &err,
                    );
                    let session_cookie_auth_invalid = is_session_cookie_refresh_auth_error(&err);
                    log::warn!(
                        "event=gateway_upstream_unauthorized_refresh_failed path={} account_id={} err={}",
                        path,
                        account.id,
                        err
                    );
                    if (refresh_token_invalid || session_cookie_auth_invalid) && has_more_candidates
                    {
                        let failover_reason = if refresh_token_invalid {
                            "refresh token invalid failover"
                        } else {
                            "session cookie invalid failover"
                        };
                        log_gateway_result(Some(url), 401, Some(failover_reason));
                        return PostRetryFlowDecision::Failover;
                    }
                }
            }
        }
        if let Some(alt_url) = url_alt {
            match retry_with_alternate_path(
                client,
                method,
                Some(alt_url),
                request_deadline,
                request_ctx,
                incoming_headers,
                body,
                is_stream,
                upstream_cookie,
                current_auth_token.as_str(),
                account,
                strip_session_affinity,
                status,
                debug,
                has_more_candidates,
                &mut log_gateway_result,
            ) {
                AltPathRetryResult::NotTriggered => {}
                AltPathRetryResult::Upstream(resp) => {
                    upstream = resp;
                    status = upstream.status();
                }
                AltPathRetryResult::Failover => {
                    return PostRetryFlowDecision::Failover;
                }
                AltPathRetryResult::Terminal {
                    status_code,
                    message,
                } => {
                    return PostRetryFlowDecision::Terminal {
                        status_code,
                        message,
                    };
                }
            }
        }
        let mut attempted_unauthorized_stateless_retry = false;
        match retry_stateless_then_optional_alt(
            client,
            method,
            url,
            url_alt,
            request_deadline,
            request_ctx,
            incoming_headers,
            body,
            is_stream,
            upstream_cookie,
            current_auth_token.as_str(),
            account,
            strip_session_affinity,
            status,
            debug,
            disable_challenge_stateless_retry,
            allow_unauthorized_stateless_retry,
        ) {
            StatelessRetryResult::NotTriggered => {}
            StatelessRetryResult::Upstream(resp) => {
                upstream = resp;
                status = upstream.status();
                attempted_unauthorized_stateless_retry = allow_unauthorized_stateless_retry;
            }
            StatelessRetryResult::Terminal {
                status_code,
                message,
            } => {
                return PostRetryFlowDecision::Terminal {
                    status_code,
                    message,
                };
            }
        }
        if attempted_unauthorized_stateless_retry && status.as_u16() == 401 && has_more_candidates {
            log_gateway_result(
                Some(url),
                401,
                Some("access token only stateless retry failover"),
            );
            return PostRetryFlowDecision::Failover;
        }
    }

    // 中文注释：主流程 fallback 只覆盖首跳响应，这里补齐“重试后仍 challenge/401/403/429”场景。
    match handle_openai_fallback_branch(
        client,
        storage,
        method,
        incoming_headers,
        body,
        is_stream,
        upstream_base,
        path,
        upstream_fallback_base,
        account,
        token,
        upstream_cookie,
        strip_session_affinity,
        debug,
        allow_openai_fallback,
        status,
        upstream.headers().get(reqwest::header::CONTENT_TYPE),
        has_more_candidates,
        &mut log_gateway_result,
    ) {
        FallbackBranchResult::NotTriggered => {}
        FallbackBranchResult::RespondUpstream(resp) => {
            return PostRetryFlowDecision::RespondUpstream(resp);
        }
        FallbackBranchResult::Failover => {
            return PostRetryFlowDecision::Failover;
        }
        FallbackBranchResult::Terminal {
            status_code,
            message,
        } => {
            return PostRetryFlowDecision::Terminal {
                status_code,
                message,
            };
        }
    }

    match decide_upstream_outcome(
        storage,
        &account.id,
        status,
        upstream.headers().get(reqwest::header::CONTENT_TYPE),
        url,
        has_more_candidates,
        &mut log_gateway_result,
    ) {
        UpstreamOutcomeDecision::Failover => PostRetryFlowDecision::Failover,
        UpstreamOutcomeDecision::RespondUpstream => {
            PostRetryFlowDecision::RespondUpstream(upstream)
        }
    }
}
