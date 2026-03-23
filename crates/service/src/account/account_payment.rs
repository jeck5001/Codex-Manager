use codexmanager_core::{
    auth::{
        extract_token_exp, extract_workspace_name, parse_id_token_claims, DEFAULT_CLIENT_ID,
        DEFAULT_ISSUER,
    },
    storage::{Account, Token},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Duration};

use crate::{
    app_settings::{
        get_persisted_app_setting, save_persisted_app_setting,
        APP_SETTING_ACCOUNT_PAYMENT_STATE_KEY, APP_SETTING_ACCOUNT_SESSION_STATE_KEY,
        APP_SETTING_TEAM_MANAGER_API_KEY_KEY, APP_SETTING_TEAM_MANAGER_API_URL_KEY,
        APP_SETTING_TEAM_MANAGER_ENABLED_KEY,
    },
    storage_helpers::open_storage,
    usage_token_refresh::refresh_and_persist_access_token,
};

const PAYMENT_CHECKOUT_URL: &str = "https://chatgpt.com/backend-api/payments/checkout";
const TEAM_CHECKOUT_BASE_URL: &str = "https://chatgpt.com/checkout/openai_llc/";
const TEAM_MANAGER_IMPORT_PATH: &str = "/api/accounts/import";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AccountPaymentState {
    pub subscription_plan_type: Option<String>,
    pub subscription_updated_at: Option<i64>,
    pub team_manager_uploaded_at: Option<i64>,
    pub official_promo_link: Option<String>,
    pub official_promo_link_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AccountSessionState {
    pub cookies: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TeamManagerSettings {
    enabled: bool,
    api_url: String,
    api_key: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TeamManagerTestPayload {
    api_url: Option<String>,
    api_key: Option<String>,
}

struct CheckoutLinkArgs<'a> {
    access_token: &'a str,
    cookies: Option<&'a str>,
    plan_type: &'a str,
    workspace_name: &'a str,
    price_interval: &'a str,
    seat_quantity: i64,
    country: &'a str,
    proxy: Option<&'a str>,
}

fn payment_http_client(proxy: Option<&str>) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(Duration::from_secs(30));
    if let Some(proxy_url) = proxy.map(str::trim).filter(|value| !value.is_empty()) {
        let proxy =
            reqwest::Proxy::all(proxy_url).map_err(|err| format!("invalid proxy url: {err}"))?;
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .map_err(|err| format!("build payment client failed: {err}"))
}

fn team_manager_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build team manager client failed: {err}"))
}

fn resolve_account_with_token(account_id: &str) -> Result<(Account, Token), String> {
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("accountId is required".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let account = storage
        .find_account_by_id(normalized_account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "account not found".to_string())?;
    let mut token = storage
        .find_token_by_account_id(normalized_account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "account token not found".to_string())?;

    let now = codexmanager_core::storage::now_ts();
    let access_exp = extract_token_exp(&token.access_token);
    let should_refresh = !token.refresh_token.trim().is_empty()
        && access_exp.map(|value| value <= now + 60).unwrap_or(false);
    if should_refresh {
        let issuer =
            std::env::var("CODEXMANAGER_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.to_string());
        let client_id = std::env::var("CODEXMANAGER_CLIENT_ID")
            .unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
        refresh_and_persist_access_token(&storage, &mut token, &issuer, &client_id)?;
    }

    if token.access_token.trim().is_empty() {
        return Err("account access_token is empty".to_string());
    }

    Ok((account, token))
}

fn country_currency(country: &str) -> &'static str {
    match country.trim().to_ascii_uppercase().as_str() {
        "SG" => "SGD",
        "US" => "USD",
        "TR" => "TRY",
        "JP" => "JPY",
        "HK" => "HKD",
        "GB" => "GBP",
        "EU" => "EUR",
        "AU" => "AUD",
        "CA" => "CAD",
        "IN" => "INR",
        "BR" => "BRL",
        "MX" => "MXN",
        _ => "USD",
    }
}

fn extract_oai_did(cookies: &str) -> Option<String> {
    cookies.split(';').find_map(|part| {
        let trimmed = part.trim();
        trimmed
            .strip_prefix("oai-did=")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

fn payment_headers(access_token: &str, cookies: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![
        ("Authorization", format!("Bearer {access_token}")),
        ("Content-Type", "application/json".to_string()),
        ("oai-language", "zh-CN".to_string()),
    ];
    if let Some(cookie_value) = cookies.map(str::trim).filter(|value| !value.is_empty()) {
        headers.push(("cookie", cookie_value.to_string()));
        if let Some(device_id) = extract_oai_did(cookie_value) {
            headers.push(("oai-device-id", device_id));
        }
    }
    headers
}

fn normalize_subscription_plan_type(plan_type: &str) -> Option<String> {
    match plan_type.trim().to_ascii_lowercase().as_str() {
        "free" => Some("free".to_string()),
        "plus" | "pro" => Some("plus".to_string()),
        "team" | "business" | "enterprise" => Some("team".to_string()),
        _ => None,
    }
}

fn normalize_official_promo_link(link: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = link.map(str::trim) else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Ok(None);
    }
    if !raw.starts_with("https://chatgpt.com/checkout/openai_llc/cs_") {
        return Err("official promo link 必须为 chatgpt 官方 checkout 链接".to_string());
    }
    Ok(Some(raw.to_string()))
}

pub(crate) fn read_payment_state_map() -> BTreeMap<String, AccountPaymentState> {
    let Some(raw) = get_persisted_app_setting(APP_SETTING_ACCOUNT_PAYMENT_STATE_KEY) else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BTreeMap<String, AccountPaymentState>>(&raw).unwrap_or_default()
}

fn read_session_state_map() -> BTreeMap<String, AccountSessionState> {
    let Some(raw) = get_persisted_app_setting(APP_SETTING_ACCOUNT_SESSION_STATE_KEY) else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BTreeMap<String, AccountSessionState>>(&raw).unwrap_or_default()
}

fn save_payment_state_map(map: &BTreeMap<String, AccountPaymentState>) -> Result<(), String> {
    let raw = serde_json::to_string(map)
        .map_err(|err| format!("serialize payment state failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_ACCOUNT_PAYMENT_STATE_KEY, Some(&raw))
}

fn save_session_state_map(map: &BTreeMap<String, AccountSessionState>) -> Result<(), String> {
    let raw = serde_json::to_string(map)
        .map_err(|err| format!("serialize session state failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_ACCOUNT_SESSION_STATE_KEY, Some(&raw))
}

fn update_payment_state<F>(account_id: &str, mutate: F) -> Result<AccountPaymentState, String>
where
    F: FnOnce(&mut AccountPaymentState),
{
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("accountId is required".to_string());
    }

    let mut map = read_payment_state_map();
    let entry = map.entry(normalized_account_id.to_string()).or_default();
    mutate(entry);
    let next = entry.clone();
    save_payment_state_map(&map)?;
    Ok(next)
}

fn update_session_state<F>(account_id: &str, mutate: F) -> Result<AccountSessionState, String>
where
    F: FnOnce(&mut AccountSessionState),
{
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("accountId is required".to_string());
    }

    let mut map = read_session_state_map();
    let entry = map.entry(normalized_account_id.to_string()).or_default();
    mutate(entry);
    let next = entry.clone();
    save_session_state_map(&map)?;
    Ok(next)
}

pub(crate) fn store_account_cookies(
    account_id: &str,
    cookies: Option<&str>,
) -> Result<AccountSessionState, String> {
    let Some(normalized_cookies) = cookies.map(str::trim) else {
        let state = read_session_state_map();
        return Ok(state.get(account_id.trim()).cloned().unwrap_or_default());
    };
    update_session_state(account_id, |entry| {
        entry.cookies = if normalized_cookies.is_empty() {
            None
        } else {
            Some(normalized_cookies.to_string())
        };
    })
}

fn account_cookies(account_id: &str) -> Option<String> {
    read_session_state_map()
        .get(account_id.trim())
        .and_then(|entry| entry.cookies.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_team_manager_settings() -> TeamManagerSettings {
    let enabled = get_persisted_app_setting(APP_SETTING_TEAM_MANAGER_ENABLED_KEY)
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);

    TeamManagerSettings {
        enabled,
        api_url: get_persisted_app_setting(APP_SETTING_TEAM_MANAGER_API_URL_KEY)
            .unwrap_or_default(),
        api_key: get_persisted_app_setting(APP_SETTING_TEAM_MANAGER_API_KEY_KEY)
            .unwrap_or_default(),
    }
}

fn infer_subscription_plan_type(payload: &Value) -> String {
    let direct_plan = payload
        .get("plan_type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if direct_plan.contains("team")
        || direct_plan.contains("enterprise")
        || direct_plan.contains("business")
    {
        return "team".to_string();
    }
    if direct_plan.contains("plus") || direct_plan.contains("pro") {
        return "plus".to_string();
    }

    if let Some(orgs) = payload
        .get("orgs")
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
    {
        for org in orgs {
            let workspace_plan_type = org
                .get("settings")
                .and_then(|value| value.get("workspace_plan_type"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if matches!(
                workspace_plan_type.as_str(),
                "team" | "enterprise" | "business"
            ) {
                return "team".to_string();
            }
        }
    }

    "free".to_string()
}

fn generate_checkout_link(args: CheckoutLinkArgs<'_>) -> Result<String, String> {
    let client = payment_http_client(args.proxy)?;
    let currency = country_currency(args.country);

    let payload = if args.plan_type.eq_ignore_ascii_case("team") {
        json!({
            "plan_name": "chatgptteamplan",
            "team_plan_data": {
                "workspace_name": args.workspace_name,
                "price_interval": args.price_interval,
                "seat_quantity": args.seat_quantity.max(1),
            },
            "billing_details": {
                "country": args.country.trim().to_ascii_uppercase(),
                "currency": currency,
            },
            "promo_campaign": {
                "promo_campaign_id": "team-1-month-free",
                "is_coupon_from_query_param": true,
            },
            "cancel_url": "https://chatgpt.com/#pricing",
            "checkout_ui_mode": "custom",
        })
    } else {
        json!({
            "plan_name": "chatgptplusplan",
            "billing_details": {
                "country": args.country.trim().to_ascii_uppercase(),
                "currency": currency,
            },
            "promo_campaign": {
                "promo_campaign_id": "plus-1-month-free",
                "is_coupon_from_query_param": false,
            },
            "checkout_ui_mode": "custom",
        })
    };

    let mut request = client.post(PAYMENT_CHECKOUT_URL);
    for (name, value) in payment_headers(args.access_token, args.cookies) {
        request = request.header(name, value);
    }
    let response = request
        .json(&payload)
        .send()
        .map_err(|err| format!("request payment checkout failed: {err}"))?;

    let status = response.status();
    let payload = response
        .json::<Value>()
        .map_err(|err| format!("parse payment checkout response failed: {err}"))?;
    if !status.is_success() {
        let detail = payload
            .get("detail")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("payment checkout failed");
        return Err(format!(
            "payment checkout http {}: {}",
            status.as_u16(),
            detail
        ));
    }

    let checkout_session_id = payload
        .get("checkout_session_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "payment checkout missing checkout_session_id".to_string())?;
    Ok(format!("{TEAM_CHECKOUT_BASE_URL}{checkout_session_id}"))
}

fn resolve_account_email(token: &Token, account: &Account) -> String {
    parse_id_token_claims(&token.id_token)
        .ok()
        .and_then(|claims| claims.email)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            parse_id_token_claims(&token.access_token)
                .ok()
                .and_then(|claims| claims.email)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| account.label.clone())
}

fn team_manager_import_url(api_url: &str) -> Result<String, String> {
    let normalized = api_url.trim().trim_end_matches('/');
    if normalized.is_empty() {
        return Err("Team Manager API URL 未配置".to_string());
    }
    Ok(format!("{normalized}{TEAM_MANAGER_IMPORT_PATH}"))
}

fn upload_token_to_team_manager(
    account: &Account,
    token: &Token,
    settings: &TeamManagerSettings,
) -> Result<String, String> {
    if !settings.enabled {
        return Err("Team Manager 上传未启用".to_string());
    }
    if settings.api_url.trim().is_empty() {
        return Err("Team Manager API URL 未配置".to_string());
    }
    if settings.api_key.trim().is_empty() {
        return Err("Team Manager API Key 未配置".to_string());
    }
    if token.access_token.trim().is_empty() {
        return Err("账号缺少 access_token".to_string());
    }

    let url = team_manager_import_url(&settings.api_url)?;
    let client = team_manager_http_client()?;
    let workspace_name = extract_workspace_name(&token.id_token)
        .or_else(|| extract_workspace_name(&token.access_token))
        .unwrap_or_else(|| "MyTeam".to_string());
    let payload = json!({
        "import_type": "single",
        "email": resolve_account_email(token, account),
        "account_id": account.id.clone(),
        "account_label": account.label.clone(),
        "workspace_id": account.workspace_id.clone(),
        "workspace_name": workspace_name,
        "access_token": token.access_token.clone(),
        "refresh_token": token.refresh_token.clone(),
        "id_token": token.id_token.clone(),
        "client_id": std::env::var("CODEXMANAGER_CLIENT_ID")
            .unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string()),
    });

    let response = client
        .post(&url)
        .header("X-API-Key", settings.api_key.trim())
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|err| format!("上传异常: {err}"))?;
    let status = response.status();
    let body = response.text().unwrap_or_default();
    if status.is_success() {
        return Ok("上传成功".to_string());
    }

    let detail = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| value.get("detail").and_then(Value::as_str))
                .map(ToString::to_string)
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                format!("上传失败: HTTP {}", status.as_u16())
            } else {
                format!(
                    "上传失败: HTTP {} - {}",
                    status.as_u16(),
                    trimmed.chars().take(200).collect::<String>()
                )
            }
        });
    Err(detail)
}

pub(crate) fn generate_payment_link(
    account_id: &str,
    plan_type: &str,
    workspace_name: Option<&str>,
    price_interval: Option<&str>,
    seat_quantity: Option<i64>,
    country: Option<&str>,
    proxy: Option<&str>,
) -> Result<Value, String> {
    let (account, token) = resolve_account_with_token(account_id)?;
    let normalized_plan = match plan_type.trim().to_ascii_lowercase().as_str() {
        "plus" => "plus",
        "team" => "team",
        _ => return Err("planType must be plus or team".to_string()),
    };
    let link = generate_checkout_link(CheckoutLinkArgs {
        access_token: &token.access_token,
        cookies: account_cookies(&account.id).as_deref(),
        plan_type: normalized_plan,
        workspace_name: workspace_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("MyTeam"),
        price_interval: price_interval
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("month"),
        seat_quantity: seat_quantity.unwrap_or(5).max(1),
        country: country
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("SG"),
        proxy,
    })?;

    Ok(json!({
        "accountId": account.id,
        "accountName": account.label,
        "planType": normalized_plan,
        "link": link,
    }))
}

pub(crate) fn check_account_subscription(
    account_id: &str,
    proxy: Option<&str>,
) -> Result<Value, String> {
    let (account, token) = resolve_account_with_token(account_id)?;
    let client = payment_http_client(proxy)?;
    let response = client
        .get("https://chatgpt.com/backend-api/me")
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("Content-Type", "application/json")
        .send()
        .map_err(|err| format!("request subscription status failed: {err}"))?;

    let status = response.status();
    let payload = response
        .json::<Value>()
        .map_err(|err| format!("parse subscription response failed: {err}"))?;
    if !status.is_success() {
        let detail = payload
            .get("detail")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("subscription status request failed");
        return Err(format!(
            "subscription status http {}: {}",
            status.as_u16(),
            detail
        ));
    }

    let detected_plan_type = infer_subscription_plan_type(&payload);
    let state = update_payment_state(&account.id, |entry| {
        entry.subscription_plan_type = Some(detected_plan_type.clone());
        entry.subscription_updated_at = Some(codexmanager_core::storage::now_ts());
    })?;
    Ok(json!({
        "accountId": account.id,
        "accountName": account.label,
        "success": true,
        "planType": detected_plan_type,
        "subscriptionUpdatedAt": state.subscription_updated_at,
        "rawPlanType": payload.get("plan_type").cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn check_many_accounts_subscription(
    account_ids: Vec<String>,
    proxy: Option<&str>,
) -> Result<Value, String> {
    if account_ids.is_empty() {
        return Err("accountIds is required".to_string());
    }

    let mut success_count = 0_i64;
    let mut failed_count = 0_i64;
    let mut details = Vec::with_capacity(account_ids.len());

    for account_id in account_ids {
        match check_account_subscription(&account_id, proxy) {
            Ok(result) => {
                success_count += 1;
                details.push(result);
            }
            Err(err) => {
                failed_count += 1;
                details.push(json!({
                    "accountId": account_id,
                    "success": false,
                    "error": err,
                }));
            }
        }
    }

    Ok(json!({
        "successCount": success_count,
        "failedCount": failed_count,
        "details": details,
    }))
}

pub(crate) fn mark_account_subscription(
    account_id: &str,
    plan_type: &str,
) -> Result<Value, String> {
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("accountId is required".to_string());
    }

    let normalized_plan = normalize_subscription_plan_type(plan_type)
        .ok_or_else(|| "planType must be free / plus / team".to_string())?;
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let account = storage
        .find_account_by_id(normalized_account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "account not found".to_string())?;
    let state = update_payment_state(normalized_account_id, |entry| {
        entry.subscription_plan_type = Some(normalized_plan.clone());
        entry.subscription_updated_at = Some(codexmanager_core::storage::now_ts());
    })?;

    Ok(json!({
        "accountId": account.id,
        "accountName": account.label,
        "success": true,
        "planType": normalized_plan,
        "subscriptionUpdatedAt": state.subscription_updated_at,
    }))
}

pub(crate) fn set_account_official_promo_link(
    account_id: &str,
    link: Option<&str>,
) -> Result<Value, String> {
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("accountId is required".to_string());
    }
    let (account, _) = resolve_account_with_token(normalized_account_id)?;
    let normalized_link = normalize_official_promo_link(link)?;
    let state = update_payment_state(normalized_account_id, |entry| {
        entry.official_promo_link = normalized_link.clone();
        entry.official_promo_link_updated_at = normalized_link
            .as_ref()
            .map(|_| codexmanager_core::storage::now_ts());
    })?;

    Ok(json!({
        "accountId": account.id,
        "accountName": account.label,
        "success": true,
        "officialPromoLink": state.official_promo_link,
        "officialPromoLinkUpdatedAt": state.official_promo_link_updated_at,
    }))
}

pub(crate) fn upload_account_to_team_manager(account_id: &str) -> Result<Value, String> {
    let (account, token) = resolve_account_with_token(account_id)?;
    let settings = read_team_manager_settings();
    let message = upload_token_to_team_manager(&account, &token, &settings)?;
    let state = update_payment_state(&account.id, |entry| {
        entry.team_manager_uploaded_at = Some(codexmanager_core::storage::now_ts());
    })?;

    Ok(json!({
        "accountId": account.id,
        "accountName": account.label,
        "success": true,
        "message": message,
        "uploadedAt": state.team_manager_uploaded_at,
    }))
}

pub(crate) fn upload_many_accounts_to_team_manager(
    account_ids: Vec<String>,
) -> Result<Value, String> {
    if account_ids.is_empty() {
        return Err("accountIds is required".to_string());
    }

    let mut success_count = 0_i64;
    let mut failed_count = 0_i64;
    let mut skipped_count = 0_i64;
    let mut details = Vec::with_capacity(account_ids.len());

    for account_id in account_ids {
        match upload_account_to_team_manager(&account_id) {
            Ok(result) => {
                success_count += 1;
                details.push(result);
            }
            Err(err) => {
                if err.contains("access_token") {
                    skipped_count += 1;
                } else {
                    failed_count += 1;
                }
                details.push(json!({
                    "accountId": account_id,
                    "success": false,
                    "error": err,
                }));
            }
        }
    }

    Ok(json!({
        "successCount": success_count,
        "failedCount": failed_count,
        "skippedCount": skipped_count,
        "details": details,
    }))
}

pub(crate) fn test_team_manager_connection(params: Option<&Value>) -> Result<Value, String> {
    let payload = params
        .cloned()
        .map(serde_json::from_value::<TeamManagerTestPayload>)
        .transpose()
        .map_err(|err| format!("invalid team manager test payload: {err}"))?
        .unwrap_or_default();
    let saved = read_team_manager_settings();
    let api_url = payload.api_url.unwrap_or(saved.api_url).trim().to_string();
    let api_key = payload
        .api_key
        .filter(|value| !value.trim().is_empty() && value != "use_saved_key")
        .unwrap_or(saved.api_key)
        .trim()
        .to_string();

    if api_url.is_empty() {
        return Ok(json!({ "success": false, "message": "API URL 不能为空" }));
    }
    if api_key.is_empty() {
        return Ok(json!({ "success": false, "message": "未配置 API Key" }));
    }

    let url = team_manager_import_url(&api_url)?;
    let client = team_manager_http_client()?;
    let response = client
        .request(reqwest::Method::OPTIONS, &url)
        .header("X-API-Key", api_key)
        .send();

    match response {
        Ok(response) => {
            let status = response.status().as_u16();
            if matches!(status, 200 | 204 | 401 | 403 | 405) {
                if status == 401 {
                    return Ok(json!({
                        "success": false,
                        "message": "连接成功，但 API Key 无效",
                    }));
                }
                return Ok(json!({
                    "success": true,
                    "message": "Team Manager 连接测试成功",
                }));
            }
            Ok(json!({
                "success": false,
                "message": format!("服务器返回异常状态码: {status}"),
            }))
        }
        Err(err) if err.is_timeout() => Ok(json!({
            "success": false,
            "message": "连接超时，请检查网络配置",
        })),
        Err(err) => Ok(json!({
            "success": false,
            "message": format!("连接测试失败: {err}"),
        })),
    }
}
