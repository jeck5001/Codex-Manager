use codexmanager_core::storage::{Account, Token};

use super::super::super::IncomingHeaderSnapshot;
use crate::apikey_profile::PROTOCOL_ANTHROPIC_NATIVE;

pub(in super::super) struct UpstreamRequestSetup {
    pub(in super::super) upstream_base: String,
    pub(in super::super) upstream_fallback_base: Option<String>,
    pub(in super::super) url: String,
    pub(in super::super) url_alt: Option<String>,
    pub(in super::super) upstream_cookie: Option<String>,
    pub(in super::super) candidate_count: usize,
    pub(in super::super) account_max_inflight: usize,
    pub(in super::super) anthropic_has_prompt_cache_key: bool,
    pub(in super::super) has_sticky_fallback_session: bool,
    pub(in super::super) has_sticky_fallback_conversation: bool,
    pub(in super::super) has_body_encrypted_content: bool,
}

pub(in super::super) struct PrepareRequestSetupInput<'a> {
    pub(in super::super) path: &'a str,
    pub(in super::super) protocol_type: &'a str,
    pub(in super::super) has_prompt_cache_key: bool,
    pub(in super::super) incoming_headers: &'a IncomingHeaderSnapshot,
    pub(in super::super) body: &'a [u8],
    pub(in super::super) key_id: &'a str,
    pub(in super::super) model_for_log: Option<&'a str>,
    pub(in super::super) trace_id: &'a str,
}

pub(in super::super) fn prepare_request_setup(
    input: PrepareRequestSetupInput<'_>,
    candidates: &mut [(Account, Token)],
) -> UpstreamRequestSetup {
    let upstream_base = super::super::super::resolve_upstream_base_url();
    let upstream_fallback_base =
        super::super::super::resolve_upstream_fallback_base_url(upstream_base.as_str());
    let (url, url_alt) = super::super::super::request_rewrite::compute_upstream_url(
        upstream_base.as_str(),
        input.path,
    );
    let upstream_cookie = super::super::super::upstream_cookie();

    let candidate_count = candidates.len();
    let account_max_inflight = super::super::super::account_max_inflight_limit();
    let anthropic_has_prompt_cache_key =
        input.protocol_type == PROTOCOL_ANTHROPIC_NATIVE && input.has_prompt_cache_key;
    super::super::super::apply_route_strategy(candidates, input.key_id, input.model_for_log);
    let candidate_order = candidates
        .iter()
        .map(|(account, _)| format!("{}#sort={}", account.id, account.sort))
        .collect::<Vec<_>>();
    super::super::super::trace_log::log_candidate_pool(
        input.trace_id,
        input.key_id,
        super::super::super::current_route_strategy(),
        candidate_order.as_slice(),
    );

    UpstreamRequestSetup {
        upstream_base,
        upstream_fallback_base,
        url,
        url_alt,
        upstream_cookie,
        candidate_count,
        account_max_inflight,
        anthropic_has_prompt_cache_key,
        has_sticky_fallback_session:
            super::super::header_profile::derive_sticky_session_id_from_headers(
                input.incoming_headers,
            )
            .is_some(),
        has_sticky_fallback_conversation:
            super::super::header_profile::derive_sticky_conversation_id_from_headers(
                input.incoming_headers,
            )
            .is_some(),
        has_body_encrypted_content:
            super::super::support::payload_rewrite::body_has_encrypted_content_hint(input.body),
    }
}
