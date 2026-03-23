use bytes::Bytes;
use codexmanager_core::storage::{Account, Storage, Token};
use reqwest::header::CONTENT_TYPE;

use super::super::support::outcome::{decide_upstream_outcome, UpstreamOutcomeDecision};

pub(super) enum OpenAiAttemptResult {
    Upstream(reqwest::blocking::Response),
    Failover,
    Terminal { status_code: u16, message: String },
}

pub(super) struct OpenAiBaseAttemptArgs<'a> {
    pub(super) client: &'a reqwest::blocking::Client,
    pub(super) storage: &'a Storage,
    pub(super) method: &'a reqwest::Method,
    pub(super) path: &'a str,
    pub(super) incoming_headers: &'a super::super::super::IncomingHeaderSnapshot,
    pub(super) body: &'a Bytes,
    pub(super) is_stream: bool,
    pub(super) base: &'a str,
    pub(super) account: &'a Account,
    pub(super) token: &'a mut Token,
    pub(super) upstream_cookie: Option<&'a str>,
    pub(super) strip_session_affinity: bool,
    pub(super) debug: bool,
    pub(super) has_more_candidates: bool,
}

pub(super) fn handle_openai_base_attempt<F>(
    args: OpenAiBaseAttemptArgs<'_>,
    mut log_gateway_result: F,
) -> OpenAiAttemptResult
where
    F: FnMut(Option<&str>, u16, Option<&str>),
{
    let OpenAiBaseAttemptArgs {
        client,
        storage,
        method,
        path,
        incoming_headers,
        body,
        is_stream,
        base,
        account,
        token,
        upstream_cookie,
        strip_session_affinity,
        debug,
        has_more_candidates,
    } = args;
    let (upstream_url, _url_alt) = super::super::super::compute_upstream_url(base, path);
    match super::super::super::try_openai_fallback(super::super::super::TryOpenAiFallbackArgs {
        client,
        storage,
        method,
        request_path: path,
        incoming_headers,
        body,
        is_stream,
        upstream_base: base,
        account,
        token,
        upstream_cookie,
        strip_session_affinity,
        debug,
    }) {
        Ok(Some(resp)) => {
            let status = resp.status();
            let content_type = resp.headers().get(CONTENT_TYPE);
            match decide_upstream_outcome(
                storage,
                &account.id,
                status,
                content_type,
                upstream_url.as_str(),
                has_more_candidates,
                &mut log_gateway_result,
            ) {
                UpstreamOutcomeDecision::Failover => OpenAiAttemptResult::Failover,
                UpstreamOutcomeDecision::RespondUpstream => OpenAiAttemptResult::Upstream(resp),
            }
        }
        Ok(None) => {
            super::super::super::mark_account_cooldown(
                &account.id,
                super::super::super::CooldownReason::Network,
            );
            log_gateway_result(
                Some(upstream_url.as_str()),
                502,
                Some("openai upstream unavailable"),
            );
            // 中文注释：OpenAI 上游不可用时如果还有候选账号就继续 failover，
            // 不这样做会把单账号瞬时抖动放大成整次请求失败。
            if has_more_candidates && super::super::super::retry_policy_allows_status(502) {
                OpenAiAttemptResult::Failover
            } else {
                OpenAiAttemptResult::Terminal {
                    status_code: 502,
                    message: "openai upstream unavailable".to_string(),
                }
            }
        }
        Err(err) => {
            super::super::super::mark_account_cooldown(
                &account.id,
                super::super::super::CooldownReason::Network,
            );
            log_gateway_result(Some(upstream_url.as_str()), 502, Some(err.as_str()));
            // 中文注释：异常分支同样优先切换候选账号，
            // 只有最后一个候选才直接向客户端返回错误，避免过早失败。
            if has_more_candidates && super::super::super::retry_policy_allows_status(502) {
                OpenAiAttemptResult::Failover
            } else {
                OpenAiAttemptResult::Terminal {
                    status_code: 502,
                    message: format!("openai upstream error: {err}"),
                }
            }
        }
    }
}
