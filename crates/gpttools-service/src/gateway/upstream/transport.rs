use gpttools_core::storage::Account;
use tiny_http::Request;

pub(super) fn send_upstream_request(
    client: &reqwest::blocking::Client,
    method: &reqwest::Method,
    target_url: &str,
    request: &Request,
    body: &[u8],
    is_stream: bool,
    upstream_cookie: Option<&str>,
    auth_token: &str,
    account: &Account,
    strip_session_affinity: bool,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let mut builder = client.request(method.clone(), target_url);
    let account_id = account
        .chatgpt_account_id
        .as_deref()
        .or_else(|| account.workspace_id.as_deref());
    let header_input = super::header_profile::CodexUpstreamHeaderInput {
        auth_token,
        account_id,
        upstream_cookie,
        incoming_session_id: super::header_profile::find_incoming_header(request, "session_id"),
        incoming_turn_state: super::header_profile::find_incoming_header(request, "x-codex-turn-state"),
        incoming_conversation_id: super::header_profile::find_incoming_header(request, "conversation_id"),
        strip_session_affinity,
        is_stream,
        has_body: !body.is_empty(),
    };
    for (name, value) in super::header_profile::build_codex_upstream_headers(header_input) {
        builder = builder.header(name, value);
    }
    if !body.is_empty() {
        builder = builder.body(body.to_vec());
    }
    builder.send()
}


