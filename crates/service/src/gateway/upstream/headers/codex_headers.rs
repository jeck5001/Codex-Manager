use super::sticky_ids::random_session_id;

pub(crate) const CODEX_CLIENT_VERSION: &str = "0.101.0";
const X_CODEX_WINDOW_ID_HEADER_NAME: &str = "x-codex-window-id";
const X_CODEX_PARENT_THREAD_ID_HEADER_NAME: &str = "x-codex-parent-thread-id";

fn normalize_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn looks_like_codex_identity(value: &str) -> bool {
    value.to_ascii_lowercase().contains("codex")
}

fn resolve_originator_header(
    incoming_originator: Option<&str>,
    preserve_client_identity: bool,
) -> String {
    normalize_non_empty(incoming_originator)
        .filter(|value| preserve_client_identity || looks_like_codex_identity(value))
        .map(str::to_string)
        .unwrap_or_else(crate::gateway::current_originator)
}

fn resolve_user_agent_header(
    incoming_user_agent: Option<&str>,
    preserve_client_identity: bool,
) -> String {
    normalize_non_empty(incoming_user_agent)
        .filter(|value| preserve_client_identity || looks_like_codex_identity(value))
        .map(str::to_string)
        .unwrap_or_else(crate::gateway::current_codex_user_agent)
}

pub(crate) struct CodexUpstreamHeaderInput<'a> {
    pub(crate) auth_token: &'a str,
    pub(crate) account_id: Option<&'a str>,
    pub(crate) include_account_id: bool,
    pub(crate) upstream_cookie: Option<&'a str>,
    pub(crate) incoming_user_agent: Option<&'a str>,
    pub(crate) incoming_originator: Option<&'a str>,
    pub(crate) preserve_client_identity: bool,
    pub(crate) incoming_session_id: Option<&'a str>,
    pub(crate) incoming_window_id: Option<&'a str>,
    pub(crate) incoming_client_request_id: Option<&'a str>,
    pub(crate) incoming_subagent: Option<&'a str>,
    pub(crate) incoming_beta_features: Option<&'a str>,
    pub(crate) incoming_turn_metadata: Option<&'a str>,
    pub(crate) incoming_parent_thread_id: Option<&'a str>,
    pub(crate) passthrough_codex_headers: &'a [(String, String)],
    pub(crate) fallback_session_id: Option<&'a str>,
    pub(crate) incoming_turn_state: Option<&'a str>,
    pub(crate) include_turn_state: bool,
    pub(crate) strip_session_affinity: bool,
    pub(crate) is_stream: bool,
    pub(crate) has_body: bool,
}

pub(crate) struct CodexCompactUpstreamHeaderInput<'a> {
    pub(crate) auth_token: &'a str,
    pub(crate) account_id: Option<&'a str>,
    pub(crate) include_account_id: bool,
    pub(crate) upstream_cookie: Option<&'a str>,
    pub(crate) incoming_user_agent: Option<&'a str>,
    pub(crate) incoming_originator: Option<&'a str>,
    pub(crate) preserve_client_identity: bool,
    pub(crate) incoming_session_id: Option<&'a str>,
    pub(crate) incoming_window_id: Option<&'a str>,
    pub(crate) incoming_subagent: Option<&'a str>,
    pub(crate) incoming_parent_thread_id: Option<&'a str>,
    pub(crate) passthrough_codex_headers: &'a [(String, String)],
    pub(crate) fallback_session_id: Option<&'a str>,
    pub(crate) strip_session_affinity: bool,
    pub(crate) has_body: bool,
}

pub(crate) fn build_codex_upstream_headers(
    input: CodexUpstreamHeaderInput<'_>,
) -> Vec<(String, String)> {
    let user_agent =
        resolve_user_agent_header(input.incoming_user_agent, input.preserve_client_identity);
    let originator =
        resolve_originator_header(input.incoming_originator, input.preserve_client_identity);
    let mut headers = Vec::with_capacity(16);
    let resolved_session_id = resolve_optional_session_id(
        input.incoming_session_id,
        input.fallback_session_id,
        input.strip_session_affinity,
    );
    headers.push((
        "Authorization".to_string(),
        format!("Bearer {}", input.auth_token),
    ));
    if input.has_body {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }
    headers.push((
        "Accept".to_string(),
        if input.is_stream {
            "text/event-stream"
        } else {
            "application/json"
        }
        .to_string(),
    ));
    headers.push(("User-Agent".to_string(), user_agent));
    headers.push(("originator".to_string(), originator));
    if let Some(residency_requirement) = crate::gateway::current_residency_requirement() {
        headers.push((
            crate::gateway::runtime_config::RESIDENCY_HEADER_NAME.to_string(),
            residency_requirement,
        ));
    }
    if let Some(client_request_id) = resolve_client_request_id(input.incoming_client_request_id) {
        headers.push(("x-client-request-id".to_string(), client_request_id));
    }
    if let Some(subagent) = input
        .incoming_subagent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("x-openai-subagent".to_string(), subagent.to_string()));
    }
    if let Some(beta_features) = input
        .incoming_beta_features
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            "x-codex-beta-features".to_string(),
            beta_features.to_string(),
        ));
    }
    if let Some(turn_metadata) = input
        .incoming_turn_metadata
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            "x-codex-turn-metadata".to_string(),
            turn_metadata.to_string(),
        ));
    }
    if let Some(parent_thread_id) = input
        .incoming_parent_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_PARENT_THREAD_ID_HEADER_NAME.to_string(),
            parent_thread_id.to_string(),
        ));
    }
    if let Some(session_id) = resolved_session_id.as_deref() {
        headers.push(("session_id".to_string(), session_id.to_string()));
    }
    if let Some(window_id) = resolve_window_id(
        input.incoming_window_id,
        resolved_session_id.as_deref(),
        input.strip_session_affinity,
    ) {
        headers.push((X_CODEX_WINDOW_ID_HEADER_NAME.to_string(), window_id));
    }
    append_passthrough_codex_headers(
        &mut headers,
        input.passthrough_codex_headers,
        !input.strip_session_affinity,
    );

    if !input.strip_session_affinity && input.include_turn_state {
        if let Some(turn_state) = input.incoming_turn_state {
            headers.push(("x-codex-turn-state".to_string(), turn_state.to_string()));
        }
    }

    if input.include_account_id {
        if let Some(account_id) = input.account_id {
            headers.push(("ChatGPT-Account-ID".to_string(), account_id.to_string()));
        }
    }
    if should_forward_upstream_cookie() {
        if let Some(cookie) = input
            .upstream_cookie
            .filter(|value| !value.trim().is_empty())
        {
            headers.push(("Cookie".to_string(), cookie.to_string()));
        }
    }
    headers
}

pub(crate) fn build_codex_compact_upstream_headers(
    input: CodexCompactUpstreamHeaderInput<'_>,
) -> Vec<(String, String)> {
    let user_agent =
        resolve_user_agent_header(input.incoming_user_agent, input.preserve_client_identity);
    let originator =
        resolve_originator_header(input.incoming_originator, input.preserve_client_identity);
    let mut headers = Vec::with_capacity(12);
    headers.push((
        "Authorization".to_string(),
        format!("Bearer {}", input.auth_token),
    ));
    if input.has_body {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }
    headers.push(("Accept".to_string(), "application/json".to_string()));
    headers.push(("User-Agent".to_string(), user_agent));
    headers.push(("originator".to_string(), originator));
    if let Some(residency_requirement) = crate::gateway::current_residency_requirement() {
        headers.push((
            crate::gateway::runtime_config::RESIDENCY_HEADER_NAME.to_string(),
            residency_requirement,
        ));
    }
    if let Some(subagent) = input
        .incoming_subagent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push(("x-openai-subagent".to_string(), subagent.to_string()));
    }
    if let Some(parent_thread_id) = input
        .incoming_parent_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((
            X_CODEX_PARENT_THREAD_ID_HEADER_NAME.to_string(),
            parent_thread_id.to_string(),
        ));
    }
    let resolved_session_id = resolve_optional_session_id(
        input.incoming_session_id,
        input.fallback_session_id,
        input.strip_session_affinity,
    );
    if let Some(session_id) = resolved_session_id.clone() {
        headers.push(("session_id".to_string(), session_id));
    }
    if let Some(window_id) = resolve_window_id(
        input.incoming_window_id,
        resolved_session_id.as_deref(),
        input.strip_session_affinity,
    ) {
        headers.push((X_CODEX_WINDOW_ID_HEADER_NAME.to_string(), window_id));
    }
    append_passthrough_codex_headers(
        &mut headers,
        input.passthrough_codex_headers,
        !input.strip_session_affinity,
    );
    if input.include_account_id {
        if let Some(account_id) = input.account_id {
            headers.push(("ChatGPT-Account-ID".to_string(), account_id.to_string()));
        }
    }
    let _ = input.upstream_cookie;
    headers
}

fn should_forward_upstream_cookie() -> bool {
    !crate::gateway::cpa_no_cookie_header_mode_enabled()
}

fn resolve_optional_session_id(
    incoming: Option<&str>,
    fallback_session_id: Option<&str>,
    strip_session_affinity: bool,
) -> Option<String> {
    if strip_session_affinity {
        return fallback_session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
    }
    if let Some(value) = incoming {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    fallback_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| Some(random_session_id()))
}

fn resolve_window_id(
    incoming_window_id: Option<&str>,
    resolved_session_id: Option<&str>,
    strip_session_affinity: bool,
) -> Option<String> {
    let normalized_session_id = resolved_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if !strip_session_affinity {
        if let Some(window_id) = incoming_window_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let matches_session = match normalized_session_id {
                Some(session_id) => {
                    window_id == session_id
                        || window_id.starts_with(format!("{session_id}:").as_str())
                }
                None => true,
            };
            if matches_session {
                return Some(window_id.to_string());
            }
        }
    }
    normalized_session_id.map(|session_id| format!("{session_id}:0"))
}

fn append_passthrough_codex_headers(
    headers: &mut Vec<(String, String)>,
    passthrough_headers: &[(String, String)],
    enabled: bool,
) {
    let _ = headers;
    let _ = passthrough_headers;
    let _ = enabled;
}

fn resolve_client_request_id(incoming_client_request_id: Option<&str>) -> Option<String> {
    if let Some(value) = incoming_client_request_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_string());
    }
    None
}
