use gpttools_core::storage::{Account, Storage, Token};
use reqwest::blocking::Client;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use tiny_http::Request;

pub(super) fn try_openai_fallback(
    client: &Client,
    storage: &Storage,
    method: &Method,
    request: &Request,
    body: &[u8],
    upstream_base: &str,
    account: &Account,
    token: &mut Token,
    upstream_cookie: Option<&str>,
    strip_session_affinity: bool,
    debug: bool,
) -> Result<Option<reqwest::blocking::Response>, String> {
    let path = super::normalize_models_path(request.url());
    let (url, _url_alt) = super::compute_upstream_url(upstream_base, &path);
    let bearer = super::resolve_openai_bearer_token(storage, account, token)?;

    let mut builder = client.request(method.clone(), &url);
    let mut has_user_agent = false;
    for header in request.headers() {
        let name = header.field.as_str().as_str();
        if if strip_session_affinity {
            super::should_drop_incoming_header_for_failover(name)
        } else {
            super::should_drop_incoming_header(name)
        } {
            continue;
        }
        if header.field.equiv("User-Agent") {
            has_user_agent = true;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(header.field.as_str().as_bytes()),
            HeaderValue::from_str(header.value.as_str()),
        ) {
            builder = builder.header(name, value);
        }
    }
    if !has_user_agent {
        builder = builder.header("User-Agent", "codex-cli");
    }
    if let Some(cookie) = upstream_cookie {
        if !cookie.trim().is_empty() {
            builder = builder.header("Cookie", cookie);
        }
    }
    if debug {
        eprintln!(
            "gateway upstream: base={}, token_source=api_key_access_token",
            upstream_base
        );
    }
    builder = builder.header("Authorization", format!("Bearer {}", bearer));
    if !body.is_empty() {
        builder = builder.body(body.to_vec());
    }
    let resp = builder.send().map_err(|e| e.to_string())?;
    Ok(Some(resp))
}
