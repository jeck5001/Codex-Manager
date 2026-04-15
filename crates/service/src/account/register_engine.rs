use base64::Engine;
use rand::Rng;
use rand::seq::SliceRandom;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE, COOKIE, LOCATION, REFERER, SET_COOKIE, USER_AGENT};
use reqwest::{redirect::Policy, Proxy, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

use crate::account::register_email::{GeneratorEmailProvider, RegisterEmailProvider};
use crate::account::register_http::{
    extract_id_token_claims, generate_register_oauth_start, parse_register_callback,
    submit_register_callback, RegisterOAuthStart,
};
use crate::account::register_runtime::{
    append_register_task_log, set_register_task_result, set_register_task_status,
    LocalRegisterTaskSnapshot,
};

const AUTH_BASE_URL: &str = "https://auth.openai.com";
const AUTHORIZE_CONTINUE_URL: &str = "https://auth.openai.com/api/accounts/authorize/continue";
const USER_REGISTER_URL: &str = "https://auth.openai.com/api/accounts/user/register";
const EMAIL_OTP_SEND_URL: &str = "https://auth.openai.com/api/accounts/email-otp/send";
const EMAIL_OTP_VALIDATE_URL: &str = "https://auth.openai.com/api/accounts/email-otp/validate";
const CREATE_ACCOUNT_URL: &str = "https://auth.openai.com/api/accounts/create_account";
const PASSWORD_VERIFY_URL: &str = "https://auth.openai.com/api/accounts/password/verify";
const WORKSPACE_SELECT_URL: &str = "https://auth.openai.com/api/accounts/workspace/select";
const REGISTER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36";
const EMAIL_OTP_POLL_ATTEMPTS: usize = 10;
const EMAIL_OTP_POLL_INTERVAL_SECS: u64 = 3;

#[derive(Debug, Clone)]
pub(crate) struct RegisterEngineResult {
    pub status: String,
    pub email: Option<String>,
    pub payload: String,
}

pub(crate) fn run_local_register_flow(
    task_uuid: &str,
    input: &LocalRegisterTaskSnapshot,
) -> Result<RegisterEngineResult, String> {
    if let Some(result) = test_mode_result(input) {
        match result {
            Ok(result) => {
                let parsed = serde_json::from_str::<Value>(result.payload.as_str())
                    .unwrap_or(Value::Null);
                set_register_task_result(
                    task_uuid,
                    result.email.clone(),
                    Some(result.payload.clone()),
                    parsed,
                );
                set_register_task_status(task_uuid, "completed", None, None);
                append_register_task_log(task_uuid, "register engine test mode completed");
                return Ok(result);
            }
            Err(err) => {
                return fail(task_uuid, "otp_timeout", err);
            }
        }
    }

    if input.email_service_type.trim() != "generator_email" {
        return fail(
            task_uuid,
            "email_provider_failed",
            format!(
                "unsupported local email provider: {}",
                input.email_service_type.trim()
            ),
        );
    }

    set_task_stage(task_uuid, "preparing_email");
    let provider = GeneratorEmailProvider::new()
        .map_err(|err| fail::<()>(task_uuid, "email_provider_failed", err).unwrap_err())?;
    let mailbox = provider
        .create_mailbox()
        .map_err(|err| fail::<()>(task_uuid, "email_provider_failed", err).unwrap_err())?;
    append_register_task_log(
        task_uuid,
        format!("mailbox allocated: {}", mailbox.email).as_str(),
    );

    let password = generate_password();
    let oauth_reg = generate_register_oauth_start();
    let (signup_client, signup_cookies) = build_client(input.proxy.as_deref())?;
    let auth_url = follow_redirect_chain(&signup_client, &signup_cookies, oauth_reg.auth_url.as_str())?;
    let did = get_cookie(&signup_cookies, "oai-did").unwrap_or_default();
    append_register_task_log(task_uuid, format!("oauth signup started: {auth_url}").as_str());

    set_task_stage(task_uuid, "submitting_signup");
    let (_, signup_status, _) = post_json(
        &signup_client,
        &signup_cookies,
        AUTHORIZE_CONTINUE_URL,
        did.as_str(),
        "https://auth.openai.com/create-account",
        &json!({
            "username": { "value": mailbox.email, "kind": "email" },
            "screen_hint": "signup"
        }),
    )?;
    if signup_status == StatusCode::FORBIDDEN {
        return fail(task_uuid, "signup_blocked", "signup blocked with HTTP 403");
    }
    if !signup_status.is_success() {
        return fail(
            task_uuid,
            "signup_blocked",
            format!("signup email submit failed: HTTP {}", signup_status.as_u16()),
        );
    }
    append_register_task_log(task_uuid, "signup email submitted");

    let (pwd_json, pwd_status, _) = post_json(
        &signup_client,
        &signup_cookies,
        USER_REGISTER_URL,
        did.as_str(),
        "https://auth.openai.com/create-account/password",
        &json!({
            "password": password,
            "username": mailbox.email
        }),
    )?;
    if !pwd_status.is_success() {
        return fail(
            task_uuid,
            "password_submit_failed",
            format!("password submit failed: HTTP {}", pwd_status.as_u16()),
        );
    }
    append_register_task_log(task_uuid, "password submitted");

    if response_requires_email_otp(&pwd_json) {
        request_and_validate_email_otp(
            task_uuid,
            &signup_client,
            &signup_cookies,
            did.as_str(),
            &provider,
            mailbox.credential.as_str(),
            "https://auth.openai.com/create-account/password",
            "https://auth.openai.com/email-verification",
        )?;
    }

    set_task_stage(task_uuid, "creating_account");
    let profile = generate_random_user_info();
    let (create_json, create_status, _) = post_json(
        &signup_client,
        &signup_cookies,
        CREATE_ACCOUNT_URL,
        did.as_str(),
        "https://auth.openai.com/about-you",
        &profile,
    )?;
    if !create_status.is_success() {
        return fail(
            task_uuid,
            "create_account_failed",
            format!("create account failed: HTTP {}", create_status.as_u16()),
        );
    }
    append_register_task_log(task_uuid, "account profile submitted");

    if let Some(result) = try_complete_from_continue_or_workspace(
        task_uuid,
        &signup_client,
        &signup_cookies,
        did.as_str(),
        create_json
            .get("continue_url")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        &oauth_reg,
    )? {
        return Ok(result);
    }

    set_task_stage(task_uuid, "oauth_login");
    run_silent_oauth_login(
        task_uuid,
        &provider,
        mailbox.email.as_str(),
        mailbox.credential.as_str(),
        password.as_str(),
        input.proxy.as_deref(),
    )
}

fn run_silent_oauth_login(
    task_uuid: &str,
    provider: &GeneratorEmailProvider,
    email: &str,
    mailbox_credential: &str,
    password: &str,
    proxy: Option<&str>,
) -> Result<RegisterEngineResult, String> {
    let oauth_login = generate_register_oauth_start();
    let (client, cookies) = build_client(proxy)?;
    let current_url = follow_redirect_chain(&client, &cookies, oauth_login.auth_url.as_str())?;
    if is_callback_url(current_url.as_str()) {
        return finalize_callback(task_uuid, current_url.as_str(), &oauth_login);
    }

    let did = get_cookie(&cookies, "oai-did").unwrap_or_default();
    let (login_start_json, login_start_status, _) = post_json(
        &client,
        &cookies,
        AUTHORIZE_CONTINUE_URL,
        did.as_str(),
        current_url.as_str(),
        &json!({
            "username": { "value": email, "kind": "email" }
        }),
    )?;
    if !login_start_status.is_success() {
        return fail(
            task_uuid,
            "oauth_failed",
            format!("oauth login start failed: HTTP {}", login_start_status.as_u16()),
        );
    }

    let password_url = extract_continue_url(&login_start_json)
        .ok_or_else(|| "oauth_failed: missing password continue url".to_string())?;
    let password_page_url = follow_redirect_chain(&client, &cookies, password_url.as_str())?;
    let (pwd_json, pwd_status, _) = post_json(
        &client,
        &cookies,
        PASSWORD_VERIFY_URL,
        did.as_str(),
        password_page_url.as_str(),
        &json!({ "password": password }),
    )?;
    if !pwd_status.is_success() {
        return fail(
            task_uuid,
            "oauth_failed",
            format!("password verify failed: HTTP {}", pwd_status.as_u16()),
        );
    }

    let next_url = extract_next_url(&pwd_json)
        .ok_or_else(|| "oauth_failed: missing next oauth url".to_string())?;
    let mut current_url = follow_redirect_chain(&client, &cookies, next_url.as_str())?;

    if current_url.ends_with("/email-verification") {
        request_and_validate_email_otp(
            task_uuid,
            &client,
            &cookies,
            did.as_str(),
            provider,
            mailbox_credential,
            current_url.as_str(),
            current_url.as_str(),
        )?;
        let auth_cookie = get_cookie(&cookies, "oai-client-auth-session");
        if let Some(result) = try_select_workspace(
            task_uuid,
            &client,
            &cookies,
            did.as_str(),
            current_url.as_str(),
            auth_cookie.as_deref(),
            &oauth_login,
        )? {
            return Ok(result);
        }
        current_url = current_url.to_string();
    }

    if is_callback_url(current_url.as_str()) {
        return finalize_callback(task_uuid, current_url.as_str(), &oauth_login);
    }

    if current_url.ends_with("/consent") || current_url.ends_with("/workspace") {
        let auth_cookie = get_cookie(&cookies, "oai-client-auth-session");
        if let Some(result) = try_select_workspace(
            task_uuid,
            &client,
            &cookies,
            did.as_str(),
            current_url.as_str(),
            auth_cookie.as_deref(),
            &oauth_login,
        )? {
            return Ok(result);
        }
        return fail(
            task_uuid,
            "workspace_select_failed",
            "workspace page reached without selectable workspace",
        );
    }

    fail(
        task_uuid,
        "oauth_failed",
        format!("oauth flow ended at unexpected page: {current_url}"),
    )
}

fn try_complete_from_continue_or_workspace(
    task_uuid: &str,
    client: &Client,
    cookies: &CookieStateRef,
    did: &str,
    continue_url: Option<String>,
    oauth: &RegisterOAuthStart,
) -> Result<Option<RegisterEngineResult>, String> {
    if let Some(url) = continue_url.and_then(|value| normalize_auth_url(value.as_str())) {
        let current_url = follow_redirect_chain(client, cookies, url.as_str())?;
        if is_callback_url(current_url.as_str()) {
            return finalize_callback(task_uuid, current_url.as_str(), oauth).map(Some);
        }
        let auth_cookie = get_cookie(cookies, "oai-client-auth-session");
        if let Some(result) = try_select_workspace(
            task_uuid,
            client,
            cookies,
            did,
            current_url.as_str(),
            auth_cookie.as_deref(),
            oauth,
        )? {
            return Ok(Some(result));
        }
    }

    let auth_cookie = get_cookie(cookies, "oai-client-auth-session");
    try_select_workspace(
        task_uuid,
        client,
        cookies,
        did,
        "https://auth.openai.com/sign-in-with-chatgpt/codex/consent",
        auth_cookie.as_deref(),
        oauth,
    )
}

fn try_select_workspace(
    task_uuid: &str,
    client: &Client,
    cookies: &CookieStateRef,
    did: &str,
    referer: &str,
    auth_cookie: Option<&str>,
    oauth: &RegisterOAuthStart,
) -> Result<Option<RegisterEngineResult>, String> {
    let Some(workspace_id) = parse_workspace_ids(auth_cookie).into_iter().next() else {
        return Ok(None);
    };

    set_task_stage(task_uuid, "selecting_workspace");
    append_register_task_log(
        task_uuid,
        format!("selecting workspace: {workspace_id}").as_str(),
    );
    let (select_json, select_status, _) = post_json(
        client,
        cookies,
        WORKSPACE_SELECT_URL,
        did,
        referer,
        &json!({ "workspace_id": workspace_id }),
    )?;
    if !select_status.is_success() {
        return fail(
            task_uuid,
            "workspace_select_failed",
            format!("workspace select failed: HTTP {}", select_status.as_u16()),
        );
    }
    let Some(next_url) = extract_continue_url(&select_json) else {
        return Ok(None);
    };
    let final_url = follow_redirect_chain(client, cookies, next_url.as_str())?;
    if is_callback_url(final_url.as_str()) {
        return finalize_callback(task_uuid, final_url.as_str(), oauth).map(Some);
    }
    Ok(None)
}

fn request_and_validate_email_otp(
    task_uuid: &str,
    client: &Client,
    cookies: &CookieStateRef,
    did: &str,
    provider: &GeneratorEmailProvider,
    mailbox_credential: &str,
    send_referer: &str,
    validate_referer: &str,
) -> Result<(), String> {
    set_task_stage(task_uuid, "waiting_email_otp");
    let (_, send_status, _) = post_json(client, cookies, EMAIL_OTP_SEND_URL, did, send_referer, &json!({}))?;
    if !send_status.is_success() {
        append_register_task_log(
            task_uuid,
            format!("email otp send returned HTTP {}", send_status.as_u16()).as_str(),
        );
    } else {
        append_register_task_log(task_uuid, "email OTP requested");
    }

    let code = poll_email_otp(provider, mailbox_credential).ok_or_else(|| {
        fail::<()>(task_uuid, "otp_timeout", "email OTP not received in time").unwrap_err()
    })?;
    append_register_task_log(task_uuid, format!("email OTP received: {code}").as_str());

    set_task_stage(task_uuid, "validating_email_otp");
    let (_, validate_status, _) = post_json(
        client,
        cookies,
        EMAIL_OTP_VALIDATE_URL,
        did,
        validate_referer,
        &json!({ "code": code }),
    )?;
    if !validate_status.is_success() {
        return fail(
            task_uuid,
            "otp_invalid",
            format!("email OTP validate failed: HTTP {}", validate_status.as_u16()),
        );
    }
    append_register_task_log(task_uuid, "email OTP validated");
    Ok(())
}

fn finalize_callback(
    task_uuid: &str,
    callback_url: &str,
    oauth: &RegisterOAuthStart,
) -> Result<RegisterEngineResult, String> {
    let callback = parse_register_callback(callback_url);
    if !callback.error.is_empty() {
        return fail(
            task_uuid,
            "oauth_failed",
            format!("oauth callback returned error: {}", callback.error),
        );
    }
    if callback.code.trim().is_empty() {
        return fail(task_uuid, "token_extract_failed", "callback missing authorization code");
    }
    if callback.state.trim() != oauth.state.trim() {
        return fail(task_uuid, "oauth_failed", "callback state mismatch");
    }

    set_task_stage(task_uuid, "extracting_tokens");
    let payload = submit_register_callback(
        None,
        callback.code.as_str(),
        oauth.redirect_uri.as_str(),
        None,
        oauth.code_verifier.as_str(),
    )
    .map_err(|err| fail::<()>(task_uuid, "token_extract_failed", err).unwrap_err())?;
    let parsed = serde_json::from_str::<Value>(payload.as_str())
        .map_err(|err| fail::<()>(task_uuid, "token_extract_failed", err.to_string()).unwrap_err())?;
    let claims = extract_id_token_claims(
        parsed
            .get("id_token")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );

    append_register_task_log(task_uuid, "register tokens extracted");
    set_register_task_result(
        task_uuid,
        claims.email.clone().or_else(|| {
            parsed
                .get("email")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        Some(payload.clone()),
        parsed.clone(),
    );
    set_register_task_status(task_uuid, "completed", None, None);

    Ok(RegisterEngineResult {
        status: "succeeded".to_string(),
        email: claims.email.or_else(|| {
            parsed
                .get("email")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        }),
        payload,
    })
}

type CookieStateRef = Arc<Mutex<HashMap<String, String>>>;

fn build_client(proxy: Option<&str>) -> Result<(Client, CookieStateRef), String> {
    let cookies = Arc::new(Mutex::new(HashMap::new()));
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(30));
    if let Some(proxy) = proxy.map(str::trim).filter(|value| !value.is_empty()) {
        builder = builder.proxy(
            Proxy::all(proxy).map_err(|err| format!("invalid register proxy: {err}"))?,
        );
    }
    let client = builder
        .build()
        .map_err(|err| format!("build register engine client failed: {err}"))?;
    Ok((client, cookies))
}

fn post_json(
    client: &Client,
    cookies: &CookieStateRef,
    url: &str,
    did: &str,
    referer: &str,
    payload: &Value,
) -> Result<(Value, StatusCode, HeaderMap), String> {
    let mut request = client
        .post(url)
        .header(USER_AGENT, REGISTER_USER_AGENT)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/json")
        .json(payload);
    if let Some(cookie_header) = cookie_header(cookies) {
        request = request.header(COOKIE, cookie_header);
    }
    if !referer.trim().is_empty() {
        request = request.header(REFERER, referer);
    }
    if !did.trim().is_empty() {
        request = request.header("oai-device-id", did);
    }
    let response = request
        .send()
        .map_err(|err| format!("register request failed: {err}"))?;
    store_response_cookies(cookies, response.headers());
    let status = response.status();
    let headers = response.headers().clone();
    let text = response
        .text()
        .map_err(|err| format!("read register response failed: {err}"))?;
    let json = serde_json::from_str(text.as_str()).unwrap_or(Value::Null);
    Ok((json, status, headers))
}

fn follow_redirect_chain(
    client: &Client,
    cookies: &CookieStateRef,
    start_url: &str,
) -> Result<String, String> {
    let mut current_url = normalize_auth_url(start_url)
        .ok_or_else(|| format!("invalid auth url: {start_url}"))?;

    for _ in 0..12 {
        if is_callback_url(current_url.as_str()) {
            return Ok(current_url);
        }
        let mut request = client
            .get(current_url.as_str())
            .header(USER_AGENT, REGISTER_USER_AGENT)
            .header(ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8");
        if let Some(cookie_header) = cookie_header(cookies) {
            request = request.header(COOKIE, cookie_header);
        }
        let response = request
            .send()
            .map_err(|err| format!("follow redirect failed: {err}"))?;
        store_response_cookies(cookies, response.headers());
        if !response.status().is_redirection() {
            return Ok(current_url);
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| "redirect missing Location header".to_string())?;
        current_url = join_url(current_url.as_str(), location)?;
    }

    Err("redirect chain exceeded limit".to_string())
}

fn join_url(base: &str, next: &str) -> Result<String, String> {
    if next.contains("://") {
        return Ok(next.to_string());
    }
    Url::parse(base)
        .and_then(|base_url| base_url.join(next))
        .map(|value| value.to_string())
        .map_err(|err| format!("join redirect url failed: {err}"))
}

fn extract_continue_url(value: &Value) -> Option<String> {
    value
        .get("continue_url")
        .and_then(Value::as_str)
        .and_then(normalize_auth_url)
}

fn extract_next_url(value: &Value) -> Option<String> {
    extract_continue_url(value).or_else(|| {
        let page_type = value
            .get("page")
            .and_then(|page| page.get("type"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        let mapped = match page_type {
            "email_otp_verification" => "https://auth.openai.com/email-verification",
            "sign_in_with_chatgpt_codex_consent" => {
                "https://auth.openai.com/sign-in-with-chatgpt/codex/consent"
            }
            "workspace" => "https://auth.openai.com/workspace",
            "add_phone" | "phone_verification" | "phone_otp_verification"
            | "phone_number_verification" => "https://auth.openai.com/add-phone",
            _ => "",
        };
        if mapped.is_empty() {
            None
        } else {
            Some(mapped.to_string())
        }
    })
}

fn response_requires_email_otp(value: &Value) -> bool {
    extract_continue_url(value)
        .map(|url| url.contains("verify"))
        .unwrap_or(false)
        || value
            .get("page")
            .and_then(|page| page.get("type"))
            .and_then(Value::as_str)
            .map(|kind| kind.contains("otp") || kind.contains("verification"))
            .unwrap_or(false)
}

fn poll_email_otp(
    provider: &GeneratorEmailProvider,
    mailbox_credential: &str,
) -> Option<String> {
    for _ in 0..EMAIL_OTP_POLL_ATTEMPTS {
        if let Ok(Some(code)) = provider.fetch_code(mailbox_credential) {
            if !code.trim().is_empty() {
                return Some(code);
            }
        }
        thread::sleep(Duration::from_secs(EMAIL_OTP_POLL_INTERVAL_SECS));
    }
    None
}

fn get_cookie(cookies: &CookieStateRef, name: &str) -> Option<String> {
    cookies
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(name)
        .cloned()
}

fn parse_workspace_ids(auth_cookie: Option<&str>) -> Vec<String> {
    let Some(auth_cookie) = auth_cookie else {
        return Vec::new();
    };
    for segment in auth_cookie.split('.').take(2) {
        let claims = decode_jwt_segment(segment);
        let Some(items) = claims.get("workspaces").and_then(Value::as_array) else {
            continue;
        };
        let ids = items
            .iter()
            .filter_map(|item| item.get("id").and_then(Value::as_str))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !ids.is_empty() {
            return ids;
        }
    }
    Vec::new()
}

fn decode_jwt_segment(segment: &str) -> Value {
    let raw = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(segment) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };
    serde_json::from_slice(raw.as_slice()).unwrap_or(Value::Null)
}

fn normalize_auth_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        return Some(trimmed.to_string());
    }
    Some(format!(
        "{}/{}",
        AUTH_BASE_URL.trim_end_matches('/'),
        trimmed.trim_start_matches('/')
    ))
}

fn is_callback_url(url: &str) -> bool {
    let parsed = parse_register_callback(url);
    !parsed.code.trim().is_empty() && !parsed.state.trim().is_empty()
}

fn generate_password() -> String {
    const SPECIALS: &[u8] = b"!@#$%&*";
    let mut rng = rand::thread_rng();
    let mut chars = Vec::with_capacity(20);
    for _ in 0..2 {
        chars.push((b'A' + rng.gen_range(0..26)) as char);
        chars.push((b'a' + rng.gen_range(0..26)) as char);
        chars.push((b'0' + rng.gen_range(0..10)) as char);
        chars.push(SPECIALS[rng.gen_range(0..SPECIALS.len())] as char);
    }
    while chars.len() < 20 {
        let bucket = rng.gen_range(0..4);
        let next = match bucket {
            0 => (b'A' + rng.gen_range(0..26)) as char,
            1 => (b'a' + rng.gen_range(0..26)) as char,
            2 => (b'0' + rng.gen_range(0..10)) as char,
            _ => SPECIALS[rng.gen_range(0..SPECIALS.len())] as char,
        };
        chars.push(next);
    }
    chars.shuffle(&mut rng);
    chars.into_iter().collect()
}

fn generate_random_user_info() -> Value {
    const FIRST_NAMES: &[&str] = &[
        "James", "John", "Robert", "Michael", "William", "Emma", "Olivia", "Ava", "Sophia",
    ];
    const LAST_NAMES: &[&str] = &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
    ];
    let mut rng = rand::thread_rng();
    let year = 1980 + rng.gen_range(0..20);
    let month = 1 + rng.gen_range(0..12);
    let day = 1 + rng.gen_range(0..28);
    json!({
        "name": format!(
            "{} {}",
            FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())],
            LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())]
        ),
        "birthdate": format!("{year:04}-{month:02}-{day:02}"),
    })
}

fn set_task_stage(task_uuid: &str, status: &str) {
    set_register_task_status(task_uuid, status, None, None);
}

fn cookie_header(cookies: &CookieStateRef) -> Option<String> {
    let guard = cookies
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_empty() {
        return None;
    }
    Some(
        guard
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn store_response_cookies(cookies: &CookieStateRef, headers: &HeaderMap) {
    let mut guard = cookies
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for value in headers.get_all(SET_COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        let Some((pair, _)) = raw.split_once(';') else {
            continue;
        };
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        guard.insert(name.trim().to_string(), value.trim().to_string());
    }
}

fn fail<T>(task_uuid: &str, code: &str, message: impl Into<String>) -> Result<T, String> {
    let message = message.into();
    append_register_task_log(task_uuid, format!("failure[{code}]: {message}").as_str());
    set_register_task_status(task_uuid, "failed", Some(code), Some(message.as_str()));
    Err(format!("{code}: {message}"))
}

fn test_mode_result(input: &LocalRegisterTaskSnapshot) -> Option<Result<RegisterEngineResult, String>> {
    let mode = std::env::var("CODEXMANAGER_REGISTER_ENGINE_TEST_MODE").ok()?;
    let email = if input.email_service_type == "generator_email" {
        "alpha123@generator.email"
    } else {
        "user@example.com"
    };
    let payload = json!({
        "type": "codex",
        "email": email,
        "account_id": "acc-test",
        "id_token": "header.payload.sig",
        "access_token": "access.test",
        "refresh_token": "refresh.test",
        "last_refresh": now_rfc3339(),
        "expired": now_rfc3339(),
    })
    .to_string();
    Some(match mode.as_str() {
        "success" => Ok(RegisterEngineResult {
            status: "succeeded".to_string(),
            email: Some(email.to_string()),
            payload,
        }),
        "otp_timeout" => Err("otp_timeout: email OTP not received in time".to_string()),
        _ => Err(format!("unsupported test mode: {mode}")),
    })
}

fn now_rfc3339() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct RegisterEngineTestScenario {
    email: String,
    code: Option<String>,
}

#[cfg(test)]
impl RegisterEngineTestScenario {
    pub(crate) fn success() -> Self {
        Self {
            email: "alpha123@generator.email".to_string(),
            code: Some("123456".to_string()),
        }
    }

    pub(crate) fn otp_timeout() -> Self {
        Self {
            email: "alpha123@generator.email".to_string(),
            code: None,
        }
    }
}

#[cfg(test)]
pub(crate) fn run_local_register_flow_for_test(
    scenario: RegisterEngineTestScenario,
) -> Result<RegisterEngineResult, String> {
    let Some(code) = scenario.code else {
        return Err("otp_timeout: email OTP not received in time".to_string());
    };
    Ok(RegisterEngineResult {
        status: "succeeded".to_string(),
        email: Some(scenario.email),
        payload: json!({
            "type": "codex",
            "email": "alpha123@generator.email",
            "account_id": "acc-1",
            "id_token": format!("header.{}.sig", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"email":"alpha123@generator.email","https://api.openai.com/auth":{"chatgpt_account_id":"acc-1"}}"#)),
            "access_token": "access.success",
            "refresh_token": format!("refresh.{code}"),
            "last_refresh": now_rfc3339(),
            "expired": now_rfc3339(),
        })
        .to_string(),
    })
}

#[cfg(test)]
#[path = "tests/register_engine_tests.rs"]
mod tests;
