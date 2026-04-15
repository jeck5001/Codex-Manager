use base64::Engine;
use codexmanager_core::auth::{
    build_authorize_url, generate_pkce, generate_state, parse_id_token_claims,
    token_exchange_body_authorization_code, DEFAULT_CLIENT_ID, DEFAULT_ISSUER,
    DEFAULT_ORIGINATOR,
};
use serde_json::Value;
use url::Url;

const DEFAULT_REGISTER_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";

#[derive(Debug, Clone)]
pub(crate) struct RegisterOAuthStart {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RegisterCallbackParams {
    pub code: String,
    pub state: String,
    pub error: String,
    pub error_description: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RegisterIdTokenClaims {
    pub email: Option<String>,
    pub account_id: Option<String>,
}

pub(crate) fn generate_register_oauth_start() -> RegisterOAuthStart {
    let pkce = generate_pkce();
    let state = generate_state();
    let redirect_uri = DEFAULT_REGISTER_REDIRECT_URI.to_string();
    let auth_url = build_authorize_url(
        DEFAULT_ISSUER,
        DEFAULT_CLIENT_ID,
        redirect_uri.as_str(),
        pkce.code_challenge.as_str(),
        state.as_str(),
        DEFAULT_ORIGINATOR,
        None,
    );

    RegisterOAuthStart {
        auth_url,
        state,
        code_verifier: pkce.code_verifier,
        redirect_uri,
    }
}

pub(crate) fn parse_register_callback(callback_url: &str) -> RegisterCallbackParams {
    let mut candidate = callback_url.trim().to_string();
    if candidate.is_empty() {
        return RegisterCallbackParams::default();
    }

    if !candidate.contains("://") {
        if candidate.starts_with('?') {
            candidate = format!("http://localhost{candidate}");
        } else if candidate.contains('=') {
            candidate = format!("http://localhost/?{candidate}");
        } else if candidate.contains('/') || candidate.contains('#') || candidate.contains(':') {
            candidate = format!("http://{candidate}");
        }
    }

    let parsed = match Url::parse(candidate.as_str()) {
        Ok(value) => value,
        Err(_) => return RegisterCallbackParams::default(),
    };

    let mut query = parsed
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<std::collections::HashMap<_, _>>();
    let fragment = url::form_urlencoded::parse(parsed.fragment().unwrap_or_default().as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<std::collections::HashMap<_, _>>();

    for (key, value) in fragment {
        let use_fragment = query
            .get(key.as_str())
            .map(|current| current.trim().is_empty())
            .unwrap_or(true);
        if use_fragment {
            query.insert(key, value);
        }
    }

    let mut params = RegisterCallbackParams {
        code: query.get("code").cloned().unwrap_or_default(),
        state: query.get("state").cloned().unwrap_or_default(),
        error: query.get("error").cloned().unwrap_or_default(),
        error_description: query
            .get("error_description")
            .cloned()
            .unwrap_or_default(),
    };

    if !params.code.is_empty() && params.state.is_empty() && params.code.contains('#') {
        let code_with_fragment = params.code.clone();
        let mut parts = code_with_fragment.splitn(2, '#');
        params.code = parts.next().unwrap_or_default().to_string();
        params.state = parts.next().unwrap_or_default().to_string();
    }

    if params.error.is_empty() && !params.error_description.is_empty() {
        params.error = params.error_description.clone();
        params.error_description.clear();
    }

    params
}

pub(crate) fn extract_id_token_claims(id_token: &str) -> RegisterIdTokenClaims {
    if let Ok(claims) = parse_id_token_claims(id_token) {
        return RegisterIdTokenClaims {
            email: claims.email,
            account_id: claims.auth.and_then(|auth| auth.chatgpt_account_id),
        };
    }

    let payload = decode_jwt_payload(id_token);
    RegisterIdTokenClaims {
        email: payload
            .get("email")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        account_id: payload
            .get("https://api.openai.com/auth")
            .and_then(|value| value.get("chatgpt_account_id"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| {
                payload
                    .get("chatgpt_account_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            }),
    }
}

pub(crate) fn submit_register_callback(
    token_url: Option<&str>,
    code: &str,
    redirect_uri: &str,
    client_id: Option<&str>,
    code_verifier: &str,
) -> Result<String, String> {
    let endpoint = token_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{}/oauth/token", DEFAULT_ISSUER.trim_end_matches('/')));
    let effective_client_id = client_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_CLIENT_ID);
    let body = token_exchange_body_authorization_code(
        code,
        redirect_uri,
        effective_client_id,
        code_verifier,
    );
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build register callback client failed: {err}"))?
        .post(endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .map_err(|err| format!("submit register callback failed: {err}"))?;
    let response = response
        .error_for_status()
        .map_err(|err| format!("submit register callback failed: {err}"))?;
    let payload = response
        .json::<Value>()
        .map_err(|err| format!("parse register callback response failed: {err}"))?;
    serde_json::to_string(&payload)
        .map_err(|err| format!("encode register callback response failed: {err}"))
}

#[cfg(test)]
pub(crate) fn parse_register_callback_for_test(callback_url: &str) -> RegisterCallbackParams {
    parse_register_callback(callback_url)
}

#[cfg(test)]
pub(crate) fn generate_register_oauth_start_for_test() -> RegisterOAuthStart {
    generate_register_oauth_start()
}

#[cfg(test)]
pub(crate) fn extract_id_token_claims_for_test(id_token: &str) -> RegisterIdTokenClaims {
    extract_id_token_claims(id_token)
}

fn decode_jwt_payload(id_token: &str) -> Value {
    let payload = match id_token.split('.').nth(1) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return Value::Null,
    };
    let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };
    serde_json::from_slice(decoded.as_slice()).unwrap_or(Value::Null)
}

#[cfg(test)]
#[path = "tests/register_http_tests.rs"]
mod tests;
