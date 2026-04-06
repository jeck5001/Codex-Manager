use codexmanager_core::storage::{Account, Storage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::thread;
use std::time::Duration;

use crate::account_identity::pick_existing_account_id_by_identity;

const ENV_REGISTER_SERVICE_URL: &str = "CODEXMANAGER_REGISTER_SERVICE_URL";
const DEFAULT_REGISTER_SERVICE_URL: &str = "http://127.0.0.1:9000";
const REGISTER_BATCH_AUTO_IMPORT_POLL_INTERVAL_SECS: u64 = 3;
const REGISTER_BATCH_AUTO_IMPORT_TIMEOUT_SECS: u64 = 30 * 60;
const REGISTER_RECOVERY_AUTO_LOGIN_POLL_INTERVAL_SECS: u64 = 3;
const REGISTER_RECOVERY_AUTO_LOGIN_TIMEOUT_SECS: u64 = 20 * 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterTaskReadResponse {
    task_uuid: String,
    status: String,
    email_service_id: Option<i64>,
    proxy: Option<String>,
    created_at: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
    failure_code: Option<String>,
    failure_label: Option<String>,
    email: Option<String>,
    can_import: bool,
    imported_account_id: Option<String>,
    is_imported: bool,
    requires_manual_import: bool,
    result: Value,
    logs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterProxyItem {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub enabled: bool,
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
    pub priority: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct StartRegisterBatchInput<'a> {
    pub email_service_type: &'a str,
    pub email_service_id: Option<i64>,
    pub email_service_config: Option<Value>,
    pub register_mode: Option<&'a str>,
    pub browserbase_config_id: Option<i64>,
    pub proxy: Option<String>,
    pub count: i64,
    pub interval_min: i64,
    pub interval_max: i64,
    pub concurrency: i64,
    pub mode: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) struct CreateRegisterProxyInput<'a> {
    pub name: &'a str,
    pub proxy_type: &'a str,
    pub host: &'a str,
    pub port: u16,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub enabled: bool,
    pub priority: i64,
}

#[derive(Debug, Clone)]
struct AutoRegisterRecoveryPlan {
    service_type: String,
    email_service_id: Option<i64>,
}

fn build_auto_register_batch_payload(plan: &AutoRegisterRecoveryPlan) -> Value {
    json!({
        "email_service_type": plan.service_type,
        "email_service_id": plan.email_service_id,
        "register_mode": "any_auto",
        "proxy": Value::Null,
        "count": 1,
        "interval_min": 0,
        "interval_max": 0,
        "concurrency": 1,
        "mode": "parallel",
    })
}

impl RegisterTaskReadResponse {
    pub(crate) fn status(&self) -> &str {
        self.status.as_str()
    }

    pub(crate) fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub(crate) fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub(crate) fn can_import(&self) -> bool {
        self.can_import
    }
}

fn normalized_register_service_url(raw: Option<&str>) -> String {
    let base = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_REGISTER_SERVICE_URL);
    base.trim_end_matches('/').to_string()
}

fn current_register_service_url() -> String {
    normalized_register_service_url(std::env::var(ENV_REGISTER_SERVICE_URL).ok().as_deref())
}

fn register_service_url(path: &str) -> String {
    format!("{}{}", current_register_service_url(), path)
}

fn register_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build register client failed: {err}"))
}

fn read_json_response(response: reqwest::blocking::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .unwrap_or_else(|_| String::from("<unreadable body>"));
        let snippet = body.chars().take(300).collect::<String>();
        return Err(format!(
            "register service http {}: {}",
            status.as_u16(),
            snippet
        ));
    }
    response
        .json::<Value>()
        .map_err(|err| format!("parse register service response failed: {err}"))
}

fn register_get_json(path: &str) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .get(register_service_url(path))
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn register_get_json_with_query(path: &str, query: &[(String, String)]) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .get(register_service_url(path))
        .query(query)
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn register_post_json(path: &str, payload: &Value) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .post(register_service_url(path))
        .json(payload)
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn register_patch_json(path: &str, payload: &Value) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .patch(register_service_url(path))
        .json(payload)
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn register_delete_json(path: &str) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .delete(register_service_url(path))
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn register_delete_json_with_body(path: &str, payload: &Value) -> Result<Value, String> {
    let client = register_http_client()?;
    let response = client
        .delete(register_service_url(path))
        .json(payload)
        .send()
        .map_err(|err| format!("request register service failed: {err}"))?;
    read_json_response(response)
}

fn task_status(task: &Value) -> String {
    task.get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn task_result_email(task: &Value) -> Option<String> {
    task.get("result")
        .and_then(|value| value.get("email"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn task_string_field(task: &Value, key: &str) -> Option<String> {
    task.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn task_i64_field(task: &Value, key: &str) -> Option<i64> {
    task.get(key).and_then(Value::as_i64)
}

fn task_logs(logs_payload: &Value) -> Vec<String> {
    logs_payload
        .get("logs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn task_result_string_field(task_result: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        task_result
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

fn resolve_existing_imported_account_id_from_accounts(
    accounts: &[Account],
    email: Option<&str>,
    task_result: &Value,
) -> Option<String> {
    let chatgpt_account_id = task_result_string_field(
        task_result,
        &["account_id", "accountId", "chatgpt_account_id"],
    );
    let workspace_id = task_result_string_field(task_result, &["workspace_id", "workspaceId"]);
    if let Some(account_id) = pick_existing_account_id_by_identity(
        accounts.iter(),
        chatgpt_account_id.as_deref(),
        workspace_id.as_deref(),
        None,
        None,
    ) {
        return Some(account_id);
    }

    let normalized_email = email
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())?;
    let mut matches = accounts.iter().filter(|account| {
        account
            .label
            .trim()
            .eq_ignore_ascii_case(normalized_email.as_str())
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(first.id.clone())
}

fn resolve_existing_imported_account_id(
    storage: &Storage,
    email: Option<&str>,
    task_result: &Value,
) -> Option<String> {
    let accounts = storage.list_accounts().ok()?;
    resolve_existing_imported_account_id_from_accounts(accounts.as_slice(), email, task_result)
}

fn normalize_failure_text(error_message: Option<&str>, logs: &str) -> String {
    [error_message.unwrap_or_default(), logs]
        .into_iter()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase()
}

pub(crate) fn classify_register_failure_reason(
    error_message: Option<&str>,
    logs: &str,
) -> Option<(&'static str, &'static str)> {
    let normalized = normalize_failure_text(error_message, logs);
    if normalized.trim().is_empty() {
        return None;
    }

    if normalized.contains("add_phone")
        || normalized.contains("手机号验证")
        || normalized.contains("phone verification")
        || normalized.contains("blocked_step") && normalized.contains("phone")
    {
        return Some(("register_phone_required", "注册触发手机号验证"));
    }

    if normalized.contains("wrong_email_otp_code")
        || normalized.contains("wrong code. please check it and try again")
        || normalized.contains("登录验证码不是最新一封或已失效")
        || normalized.contains("验证码错误")
    {
        return Some(("register_email_otp_invalid", "邮箱验证码错误或已过期"));
    }

    if normalized.contains("等待验证码超时")
        || normalized.contains("验证码超时")
        || normalized.contains("email otp timeout")
    {
        return Some(("register_email_otp_timeout", "邮箱验证码超时"));
    }

    if (normalized.contains("proxy") || normalized.contains("代理"))
        && (normalized.contains("error")
            || normalized.contains("异常")
            || normalized.contains("timeout")
            || normalized.contains("timed out")
            || normalized.contains("connect")
            || normalized.contains("refused")
            || normalized.contains("auth required")
            || normalized.contains("authentication required")
            || normalized.contains("authentication failed")
            || normalized.contains("认证"))
    {
        return Some(("register_proxy_error", "注册代理异常"));
    }

    None
}

fn enrich_register_task_value(task: &mut Value) {
    let Some(object) = task.as_object_mut() else {
        return;
    };
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !matches!(status.as_str(), "failed" | "cancelled") {
        return;
    }

    let error_message = object
        .get("error_message")
        .or_else(|| object.get("errorMessage"))
        .and_then(Value::as_str);
    let logs = object
        .get("logs")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if let Some((code, label)) = classify_register_failure_reason(error_message, logs.as_str()) {
        object.insert("failureCode".to_string(), Value::String(code.to_string()));
        object.insert("failureLabel".to_string(), Value::String(label.to_string()));
    }
}

fn enrich_register_task_import_state(task: &mut Value, accounts: Option<&[Account]>) {
    let email = task_result_email(task);
    let task_result = task.get("result").cloned().unwrap_or(Value::Null);
    let Some(object) = task.as_object_mut() else {
        return;
    };
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let can_import = status == "completed" && email.is_some();
    let imported_account_id = accounts.and_then(|items| {
        resolve_existing_imported_account_id_from_accounts(items, email.as_deref(), &task_result)
    });
    let is_imported = imported_account_id.is_some();

    object.insert("canImport".to_string(), Value::Bool(can_import));
    object.insert("isImported".to_string(), Value::Bool(is_imported));
    object.insert(
        "requiresManualImport".to_string(),
        Value::Bool(can_import && !is_imported),
    );
    object.insert(
        "importedAccountId".to_string(),
        imported_account_id
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
}

fn task_uuid_array_from_items(items: &[Value]) -> Vec<Value> {
    items
        .iter()
        .filter_map(|item| {
            item.get("task_uuid")
                .or_else(|| item.get("taskUuid"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
        })
        .collect()
}

fn task_uuid_strings_from_payload(payload: &Value) -> Vec<String> {
    payload
        .get("tasks")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("task_uuid")
                        .or_else(|| item.get("taskUuid"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_register_task_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "completed" | "failed" | "cancelled"
    )
}

fn spawn_register_batch_auto_import(task_uuids: Vec<String>) {
    if task_uuids.is_empty() {
        return;
    }

    let _ = thread::Builder::new()
        .name("register-batch-auto-import".to_string())
        .spawn(move || {
            let started_at = std::time::Instant::now();
            let deadline =
                started_at + Duration::from_secs(REGISTER_BATCH_AUTO_IMPORT_TIMEOUT_SECS);
            let mut pending = task_uuids;

            while !pending.is_empty() && std::time::Instant::now() < deadline {
                let current = pending.clone();
                pending.clear();

                for task_uuid in current {
                    let snapshot = match read_register_task(task_uuid.as_str()) {
                        Ok(snapshot) => snapshot,
                        Err(err) => {
                            log::warn!(
                                "register batch auto import read task failed: task_uuid={} err={}",
                                task_uuid,
                                err
                            );
                            pending.push(task_uuid);
                            continue;
                        }
                    };

                    if !is_register_task_terminal(snapshot.status()) {
                        pending.push(task_uuid);
                        continue;
                    }

                    if !snapshot.can_import() {
                        log::info!(
                            "register batch auto import skipped: task_uuid={} status={}",
                            task_uuid,
                            snapshot.status()
                        );
                        continue;
                    }

                    match import_register_task(task_uuid.as_str()) {
                        Ok(imported) => {
                            let account_id = imported
                                .get("accountId")
                                .or_else(|| imported.get("account_id"))
                                .and_then(Value::as_str)
                                .unwrap_or("--");
                            log::info!(
                                "register batch auto import succeeded: task_uuid={} email={} account_id={}",
                                task_uuid,
                                snapshot.email().unwrap_or("--"),
                                account_id
                            );
                        }
                        Err(err) => {
                            log::warn!(
                                "register batch auto import failed: task_uuid={} email={} err={}",
                                task_uuid,
                                snapshot.email().unwrap_or("--"),
                                err
                            );
                        }
                    }
                }

                if !pending.is_empty() {
                    thread::sleep(Duration::from_secs(
                        REGISTER_BATCH_AUTO_IMPORT_POLL_INTERVAL_SECS,
                    ));
                }
            }

            if !pending.is_empty() {
                log::warn!(
                    "register batch auto import timed out: pending={} elapsed_ms={}",
                    pending.len(),
                    started_at.elapsed().as_millis()
                );
            }
        });
}

fn pick_remote_account_by_email<'a>(items: &'a [Value], email: &str) -> Option<&'a Value> {
    items.iter().find(|item| {
        item.get("email")
            .and_then(Value::as_str)
            .map(str::trim)
            .map(|candidate| candidate.eq_ignore_ascii_case(email.trim()))
            .unwrap_or(false)
    })
}

fn remote_account_string_field(account: &Value, key: &str) -> Option<String> {
    account
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn merge_session_token_into_cookies(
    cookies: Option<String>,
    session_token: Option<String>,
) -> Option<String> {
    let normalized_cookies = cookies
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let normalized_session_token = session_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    match (normalized_cookies, normalized_session_token) {
        (Some(existing), Some(_session_token))
            if existing.contains("__Secure-next-auth.session-token=")
                || existing.contains("next-auth.session-token=") =>
        {
            Some(existing)
        }
        (Some(existing), Some(session_token)) => {
            Some(format!("{existing}; __Secure-next-auth.session-token={session_token}"))
        }
        (Some(existing), None) => Some(existing),
        (None, Some(session_token)) => {
            Some(format!("__Secure-next-auth.session-token={session_token}"))
        }
        (None, None) => None,
    }
}

fn resolve_remote_account_for_email(email: &str) -> Result<Value, String> {
    let payload = register_get_json_with_query(
        "/api/accounts",
        &[
            ("page".to_string(), "1".to_string()),
            ("page_size".to_string(), "20".to_string()),
            ("search".to_string(), email.to_string()),
        ],
    )?;
    let items = payload
        .get("accounts")
        .and_then(Value::as_array)
        .ok_or_else(|| "register service accounts response missing accounts".to_string())?;
    pick_remote_account_by_email(items, email)
        .cloned()
        .ok_or_else(|| format!("register service account not found for email: {email}"))
}

fn find_remote_account_by_field<'a>(
    items: &'a [Value],
    field: &str,
    expected: &str,
) -> Option<&'a Value> {
    let normalized = expected.trim();
    if normalized.is_empty() {
        return None;
    }
    items.iter().find(|item| {
        item.get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .map(|value| value.eq_ignore_ascii_case(normalized))
            .unwrap_or(false)
    })
}

fn search_remote_accounts(search: &str) -> Result<Vec<Value>, String> {
    let payload = register_get_json_with_query(
        "/api/accounts",
        &[
            ("page".to_string(), "1".to_string()),
            ("page_size".to_string(), "20".to_string()),
            ("search".to_string(), search.to_string()),
        ],
    )?;
    payload
        .get("accounts")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "register service accounts response missing accounts".to_string())
}

fn resolve_remote_account(
    email_hint: Option<&str>,
    chatgpt_account_id_hint: Option<&str>,
    workspace_id_hint: Option<&str>,
) -> Result<Value, String> {
    if let Some(email) = email_hint.map(str::trim).filter(|value| !value.is_empty()) {
        if let Ok(account) = resolve_remote_account_for_email(email) {
            return Ok(account);
        }
    }

    if let Some(account_id) = chatgpt_account_id_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let items = search_remote_accounts(account_id)?;
        if let Some(account) = find_remote_account_by_field(&items, "account_id", account_id) {
            return Ok(account.clone());
        }
    }

    if let Some(workspace_id) = workspace_id_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let items = search_remote_accounts(workspace_id)?;
        if let Some(account) = find_remote_account_by_field(&items, "workspace_id", workspace_id) {
            return Ok(account.clone());
        }
    }

    let mut reasons = Vec::new();
    if let Some(email) = email_hint.map(str::trim).filter(|value| !value.is_empty()) {
        reasons.push(format!("email: {email}"));
    }
    if let Some(account_id) = chatgpt_account_id_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        reasons.push(format!("account_id: {account_id}"));
    }
    if let Some(workspace_id) = workspace_id_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        reasons.push(format!("workspace_id: {workspace_id}"));
    }
    Err(format!(
        "register service account not found for identity: {}",
        reasons.join(", ")
    ))
}

fn import_remote_account(
    remote_account: &Value,
    email: &str,
    chatgpt_account_id_hint: Option<String>,
    workspace_id_hint: Option<String>,
) -> Result<Value, String> {
    let normalized_email = remote_account_string_field(remote_account, "email")
        .or_else(|| {
            let normalized = email.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        })
        .ok_or_else(|| "email is required".to_string())?;

    let remote_account_id = remote_account
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| "register service account missing id".to_string())?;
    let remote_tokens = register_get_json(&format!("/api/accounts/{remote_account_id}/tokens"))?;

    let access_token = remote_tokens
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "register service account missing access_token".to_string())?
        .to_string();
    let refresh_token = remote_tokens
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let id_token = remote_tokens
        .get("id_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let session_token = remote_tokens
        .get("session_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let cookies = remote_account
        .get("cookies")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let cookies = merge_session_token_into_cookies(cookies, session_token);
    let chatgpt_account_id =
        remote_account_string_field(&remote_account, "account_id").or(chatgpt_account_id_hint);
    let workspace_id =
        remote_account_string_field(&remote_account, "workspace_id").or(workspace_id_hint);

    let imported = crate::auth_account::login_with_chatgpt_auth_tokens(
        crate::auth_account::ChatgptAuthTokensLoginInput {
            access_token,
            refresh_token,
            id_token,
            cookies,
            email_hint: Some(normalized_email.to_string()),
            chatgpt_account_id,
            workspace_id,
            chatgpt_plan_type: None,
        },
    )?;

    Ok(json!({
        "email": normalized_email,
        "remoteAccountId": remote_account_id,
        "accountId": imported.account_id,
        "chatgptAccountId": imported.chatgpt_account_id,
        "workspaceId": imported.workspace_id,
        "type": imported.kind,
    }))
}

fn import_remote_account_for_email(
    email: &str,
    chatgpt_account_id_hint: Option<String>,
    workspace_id_hint: Option<String>,
) -> Result<Value, String> {
    let remote_account = resolve_remote_account(
        Some(email),
        chatgpt_account_id_hint.as_deref(),
        workspace_id_hint.as_deref(),
    )?;
    import_remote_account(
        &remote_account,
        email,
        chatgpt_account_id_hint,
        workspace_id_hint,
    )
}

pub(crate) fn refresh_and_import_register_account_by_email(
    email: &str,
    chatgpt_account_id_hint: Option<String>,
    workspace_id_hint: Option<String>,
) -> Result<Value, String> {
    let remote_account = resolve_remote_account(
        Some(email),
        chatgpt_account_id_hint.as_deref(),
        workspace_id_hint.as_deref(),
    )?;
    let remote_account_id = remote_account
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| "register service account missing id".to_string())?;
    let refresh_payload = register_post_json(
        &format!("/api/accounts/{remote_account_id}/refresh"),
        &json!({}),
    )?;
    let refresh_success = refresh_payload
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !refresh_success {
        let error_message = refresh_payload
            .get("error")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                refresh_payload
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or("register service refresh failed");
        return Err(error_message.to_string());
    }

    import_remote_account(
        &remote_account,
        email,
        chatgpt_account_id_hint,
        workspace_id_hint,
    )
}

pub(crate) fn auto_register_and_import_account() -> Result<Value, String> {
    let plan = resolve_auto_register_recovery_plan()?;
    let started =
        register_post_json("/api/registration/batch", &build_auto_register_batch_payload(&plan))?;
    let task_uuid = task_uuid_strings_from_payload(&started)
        .into_iter()
        .next()
        .ok_or_else(|| "register auto login returned no task uuid".to_string())?;
    let deadline =
        std::time::Instant::now() + Duration::from_secs(REGISTER_RECOVERY_AUTO_LOGIN_TIMEOUT_SECS);

    while std::time::Instant::now() < deadline {
        let snapshot = read_register_task(task_uuid.as_str())?;
        if is_register_task_terminal(snapshot.status()) {
            if snapshot.can_import() {
                let mut imported = import_register_task(task_uuid.as_str())?;
                if let Some(object) = imported.as_object_mut() {
                    object.insert(
                        "recoveryMode".to_string(),
                        Value::String("registerAutoLogin".to_string()),
                    );
                }
                return Ok(imported);
            }
            return Err(format!(
                "register auto login task {} ended with status {}: {}",
                task_uuid,
                snapshot.status(),
                snapshot.error_message().unwrap_or("not importable")
            ));
        }
        thread::sleep(Duration::from_secs(
            REGISTER_RECOVERY_AUTO_LOGIN_POLL_INTERVAL_SECS,
        ));
    }

    Err(format!("register auto login task timed out: {}", task_uuid))
}

pub(crate) fn available_register_services() -> Result<Value, String> {
    let mut payload = register_get_json("/api/registration/available-services")?;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "serviceUrl".to_string(),
            Value::String(current_register_service_url()),
        );
    }
    Ok(payload)
}

pub(crate) fn start_register_task(
    email_service_type: &str,
    email_service_id: Option<i64>,
    email_service_config: Option<Value>,
    register_mode: Option<&str>,
    browserbase_config_id: Option<i64>,
    proxy: Option<String>,
    auto_create_temp_mail_service: Option<bool>,
) -> Result<Value, String> {
    let service_type = email_service_type.trim();
    let register_mode = register_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("standard");
    if service_type.is_empty() && register_mode != "browserbase_ddg" {
        return Err("emailServiceType is required".to_string());
    }
    let mut payload = json!({
        "email_service_type": service_type,
        "email_service_id": email_service_id,
        "register_mode": register_mode,
        "browserbase_config_id": browserbase_config_id,
        "proxy": proxy,
    });
    if let Some(config) = email_service_config {
        if config.is_object() {
            if let Some(object) = payload.as_object_mut() {
                object.insert("email_service_config".to_string(), config);
            }
        }
    }
    if let Some(flag) = auto_create_temp_mail_service {
        if let Some(object) = payload.as_object_mut() {
            object.insert("auto_create_temp_mail_service".to_string(), json!(flag));
        }
    }
    register_post_json("/api/registration/start", &payload)
}

pub(crate) fn start_register_batch(
    input: StartRegisterBatchInput<'_>,
    auto_create_temp_mail_service: Option<bool>,
) -> Result<Value, String> {
    let StartRegisterBatchInput {
        email_service_type,
        email_service_id,
        email_service_config,
        register_mode,
        browserbase_config_id,
        proxy,
        count,
        interval_min,
        interval_max,
        concurrency,
        mode,
    } = input;
    let service_type = email_service_type.trim();
    let register_mode = register_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("standard");
    if service_type.is_empty() && register_mode != "browserbase_ddg" {
        return Err("emailServiceType is required".to_string());
    }
    if count < 1 {
        return Err("count must be greater than 0".to_string());
    }
    if interval_min < 0 || interval_max < interval_min {
        return Err("invalid interval range".to_string());
    }
    if concurrency < 1 {
        return Err("concurrency must be greater than 0".to_string());
    }
    let normalized_mode = mode.trim().to_ascii_lowercase();
    if !matches!(normalized_mode.as_str(), "pipeline" | "parallel") {
        return Err("mode must be pipeline or parallel".to_string());
    }

    let mut payload = json!({
        "email_service_type": service_type,
        "email_service_id": email_service_id,
        "register_mode": register_mode,
        "browserbase_config_id": browserbase_config_id,
        "proxy": proxy,
        "count": count,
        "interval_min": interval_min,
        "interval_max": interval_max,
        "concurrency": concurrency,
        "mode": normalized_mode,
    });
    if let Some(config) = email_service_config {
        if config.is_object() {
            if let Some(object) = payload.as_object_mut() {
                object.insert("email_service_config".to_string(), config);
            }
        }
    }
    if let Some(flag) = auto_create_temp_mail_service {
        if let Some(object) = payload.as_object_mut() {
            object.insert("auto_create_temp_mail_service".to_string(), json!(flag));
        }
    }
    let mut payload = register_post_json("/api/registration/batch", &payload)?;
    let task_uuids = payload
        .get("tasks")
        .and_then(Value::as_array)
        .map(|items| task_uuid_array_from_items(items))
        .unwrap_or_default();
    let auto_import_task_uuids = task_uuid_strings_from_payload(&payload);
    if let Some(object) = payload.as_object_mut() {
        object.insert("taskUuids".to_string(), Value::Array(task_uuids));
    }
    spawn_register_batch_auto_import(auto_import_task_uuids);
    Ok(payload)
}

pub(crate) fn read_register_batch(batch_id: &str) -> Result<Value, String> {
    let batch_id = batch_id.trim();
    if batch_id.is_empty() {
        return Err("batchId is required".to_string());
    }
    register_get_json(&format!("/api/registration/batch/{batch_id}"))
}

pub(crate) fn cancel_register_batch(batch_id: &str) -> Result<Value, String> {
    let batch_id = batch_id.trim();
    if batch_id.is_empty() {
        return Err("batchId is required".to_string());
    }
    register_post_json(
        &format!("/api/registration/batch/{batch_id}/cancel"),
        &json!({}),
    )
}

pub(crate) fn list_register_tasks(
    page: i64,
    page_size: i64,
    status: Option<&str>,
) -> Result<Value, String> {
    let mut query = vec![
        ("page".to_string(), page.max(1).to_string()),
        ("page_size".to_string(), page_size.clamp(1, 100).to_string()),
    ];
    if let Some(status) = status.map(str::trim).filter(|value| !value.is_empty()) {
        query.push(("status".to_string(), status.to_string()));
    }
    let mut payload = register_get_json_with_query("/api/registration/tasks", &query)?;
    let accounts =
        crate::storage_helpers::open_storage().and_then(|storage| storage.list_accounts().ok());
    if let Some(items) = payload.get_mut("tasks").and_then(Value::as_array_mut) {
        for item in items {
            enrich_register_task_value(item);
            enrich_register_task_import_state(item, accounts.as_deref());
        }
    }
    Ok(payload)
}

pub(crate) fn register_stats() -> Result<Value, String> {
    register_get_json("/api/registration/stats")
}

pub(crate) fn cancel_register_task(task_uuid: &str) -> Result<Value, String> {
    let task_uuid = task_uuid.trim();
    if task_uuid.is_empty() {
        return Err("taskUuid is required".to_string());
    }
    register_post_json(
        &format!("/api/registration/tasks/{task_uuid}/cancel"),
        &json!({}),
    )
}

pub(crate) fn retry_register_task(
    task_uuid: &str,
    strategy: Option<&str>,
) -> Result<Value, String> {
    let task_uuid = task_uuid.trim();
    if task_uuid.is_empty() {
        return Err("taskUuid is required".to_string());
    }
    let strategy = strategy
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let result = register_post_json(
        &format!("/api/registration/tasks/{task_uuid}/retry"),
        &json!({
            "strategy": strategy,
        }),
    )?;
    crate::operation_audit::record_operation_audit(
        "register_task_retry",
        "重试注册任务",
        format!(
            "任务 {}，策略 {}",
            task_uuid,
            strategy.as_deref().unwrap_or("same")
        ),
    );
    Ok(result)
}

pub(crate) fn delete_register_task(task_uuid: &str) -> Result<Value, String> {
    let task_uuid = task_uuid.trim();
    if task_uuid.is_empty() {
        return Err("taskUuid is required".to_string());
    }
    register_delete_json(&format!("/api/registration/tasks/{task_uuid}"))
}

pub(crate) fn delete_register_tasks(task_uuids: Vec<String>) -> Result<Value, String> {
    let task_uuids = task_uuids
        .into_iter()
        .map(|task_uuid| task_uuid.trim().to_string())
        .filter(|task_uuid| !task_uuid.is_empty())
        .collect::<Vec<_>>();
    if task_uuids.is_empty() {
        return Err("taskUuids is required".to_string());
    }
    register_post_json(
        "/api/registration/tasks/batch-delete",
        &json!({
            "task_uuids": task_uuids,
        }),
    )
}

pub(crate) fn list_register_outlook_accounts() -> Result<Value, String> {
    register_get_json("/api/registration/outlook-accounts")
}

pub(crate) fn start_register_outlook_batch(
    service_ids: Vec<i64>,
    skip_registered: bool,
    proxy: Option<String>,
    interval_min: i64,
    interval_max: i64,
    concurrency: i64,
    mode: &str,
) -> Result<Value, String> {
    let ids = service_ids
        .into_iter()
        .filter(|service_id| *service_id > 0)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("serviceIds is required".to_string());
    }
    if interval_min < 0 || interval_max < interval_min {
        return Err("invalid interval range".to_string());
    }
    if concurrency < 1 {
        return Err("concurrency must be greater than 0".to_string());
    }
    let normalized_mode = mode.trim().to_ascii_lowercase();
    if !matches!(normalized_mode.as_str(), "pipeline" | "parallel") {
        return Err("mode must be pipeline or parallel".to_string());
    }

    register_post_json(
        "/api/registration/outlook-batch",
        &json!({
            "service_ids": ids,
            "skip_registered": skip_registered,
            "proxy": proxy,
            "interval_min": interval_min,
            "interval_max": interval_max,
            "concurrency": concurrency,
            "mode": normalized_mode,
        }),
    )
}

pub(crate) fn read_register_outlook_batch(batch_id: &str) -> Result<Value, String> {
    let batch_id = batch_id.trim();
    if batch_id.is_empty() {
        return Err("batchId is required".to_string());
    }
    register_get_json(&format!("/api/registration/outlook-batch/{batch_id}"))
}

pub(crate) fn cancel_register_outlook_batch(batch_id: &str) -> Result<Value, String> {
    let batch_id = batch_id.trim();
    if batch_id.is_empty() {
        return Err("batchId is required".to_string());
    }
    register_post_json(
        &format!("/api/registration/outlook-batch/{batch_id}/cancel"),
        &json!({}),
    )
}

pub(crate) fn register_email_service_types() -> Result<Value, String> {
    register_get_json("/api/email-services/types")
}

pub(crate) fn list_register_proxies(
    enabled: Option<bool>,
) -> Result<Vec<RegisterProxyItem>, String> {
    let mut query = Vec::new();
    if let Some(value) = enabled {
        query.push((
            "enabled".to_string(),
            if value { "true" } else { "false" }.to_string(),
        ));
    }
    let payload = if query.is_empty() {
        register_get_json("/api/settings/proxies")?
    } else {
        register_get_json_with_query("/api/settings/proxies", &query)?
    };
    payload
        .get("proxies")
        .and_then(Value::as_array)
        .ok_or_else(|| "register service proxies response missing proxies".to_string())?
        .iter()
        .map(|item| {
            serde_json::from_value::<RegisterProxyItem>(item.clone())
                .map_err(|err| format!("parse register proxy failed: {err}"))
        })
        .collect::<Result<Vec<_>, _>>()
}

pub(crate) fn create_register_proxy(input: CreateRegisterProxyInput<'_>) -> Result<Value, String> {
    let CreateRegisterProxyInput {
        name,
        proxy_type,
        host,
        port,
        username,
        password,
        enabled,
        priority,
    } = input;
    register_post_json(
        "/api/settings/proxies",
        &json!({
            "name": name,
            "type": proxy_type,
            "host": host,
            "port": port,
            "username": username,
            "password": password,
            "enabled": enabled,
            "priority": priority,
        }),
    )
}

pub(crate) fn update_register_proxy(
    proxy_id: i64,
    name: Option<&str>,
    enabled: Option<bool>,
    priority: Option<i64>,
) -> Result<Value, String> {
    if proxy_id < 1 {
        return Err("proxyId is required".to_string());
    }
    let mut payload = serde_json::Map::new();
    if let Some(value) = name {
        payload.insert("name".to_string(), Value::String(value.to_string()));
    }
    if let Some(value) = enabled {
        payload.insert("enabled".to_string(), Value::Bool(value));
    }
    if let Some(value) = priority {
        payload.insert("priority".to_string(), Value::Number(value.max(0).into()));
    }
    register_patch_json(
        &format!("/api/settings/proxies/{proxy_id}"),
        &Value::Object(payload),
    )
}

pub(crate) fn list_register_email_services(
    service_type: Option<&str>,
    enabled_only: bool,
) -> Result<Value, String> {
    let mut query = Vec::new();
    if let Some(service_type) = service_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query.push(("service_type".to_string(), service_type.to_string()));
    }
    if enabled_only {
        query.push(("enabled_only".to_string(), "true".to_string()));
    }
    register_get_json_with_query("/api/email-services", &query)
}

pub(crate) fn list_register_browserbase_configs() -> Result<Value, String> {
    register_get_json("/api/browserbase-configs")
}

pub(crate) fn read_register_browserbase_config_full(config_id: i64) -> Result<Value, String> {
    if config_id < 1 {
        return Err("configId is required".to_string());
    }
    register_get_json(&format!("/api/browserbase-configs/{config_id}/full"))
}

pub(crate) fn create_register_browserbase_config(
    name: &str,
    enabled: bool,
    priority: i64,
    config: Value,
) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    register_post_json(
        "/api/browserbase-configs",
        &json!({
            "name": name,
            "enabled": enabled,
            "priority": priority.max(0),
            "config": config,
        }),
    )
}

pub(crate) fn update_register_browserbase_config(
    config_id: i64,
    name: Option<&str>,
    enabled: Option<bool>,
    priority: Option<i64>,
    config: Option<Value>,
) -> Result<Value, String> {
    if config_id < 1 {
        return Err("configId is required".to_string());
    }
    let mut payload = serde_json::Map::new();
    if let Some(name) = name.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("name".to_string(), Value::String(name.to_string()));
    }
    if let Some(enabled) = enabled {
        payload.insert("enabled".to_string(), Value::Bool(enabled));
    }
    if let Some(priority) = priority {
        payload.insert(
            "priority".to_string(),
            Value::Number(priority.max(0).into()),
        );
    }
    if let Some(config) = config {
        payload.insert("config".to_string(), config);
    }
    register_post_json(
        &format!("/api/browserbase-configs/{config_id}"),
        &Value::Object(payload),
    )
}

pub(crate) fn delete_register_browserbase_config(config_id: i64) -> Result<Value, String> {
    if config_id < 1 {
        return Err("configId is required".to_string());
    }
    register_delete_json(&format!("/api/browserbase-configs/{config_id}"))
}

pub(crate) fn register_email_service_stats() -> Result<Value, String> {
    register_get_json("/api/email-services/stats")
}

pub(crate) fn get_register_temp_mail_cloudflare_settings() -> Result<Value, String> {
    register_get_json("/api/settings/temp-mail/cloudflare")
}

pub(crate) fn update_register_temp_mail_cloudflare_settings(payload: Value) -> Result<Value, String> {
    let normalized = match payload {
        Value::Object(map) => Value::Object(map),
        _ => Value::Object(serde_json::Map::new()),
    };
    register_post_json("/api/settings/temp-mail/cloudflare", &normalized)
}

pub(crate) fn read_register_email_service_full(service_id: i64) -> Result<Value, String> {
    if service_id < 1 {
        return Err("serviceId is required".to_string());
    }
    register_get_json(&format!("/api/email-services/{service_id}/full"))
}

pub(crate) fn create_register_email_service(
    service_type: &str,
    name: &str,
    enabled: bool,
    priority: i64,
    config: Value,
) -> Result<Value, String> {
    let service_type = service_type.trim();
    let name = name.trim();
    if service_type.is_empty() {
        return Err("serviceType is required".to_string());
    }
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    register_post_json(
        "/api/email-services",
        &json!({
            "service_type": service_type,
            "name": name,
            "enabled": enabled,
            "priority": priority.max(0),
            "config": config,
        }),
    )
}

pub(crate) fn update_register_email_service(
    service_id: i64,
    name: Option<&str>,
    enabled: Option<bool>,
    priority: Option<i64>,
    config: Option<Value>,
) -> Result<Value, String> {
    if service_id < 1 {
        return Err("serviceId is required".to_string());
    }
    let mut payload = serde_json::Map::new();
    if let Some(name) = name.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("name".to_string(), Value::String(name.to_string()));
    }
    if let Some(enabled) = enabled {
        payload.insert("enabled".to_string(), Value::Bool(enabled));
    }
    if let Some(priority) = priority {
        payload.insert(
            "priority".to_string(),
            Value::Number(priority.max(0).into()),
        );
    }
    if let Some(config) = config {
        payload.insert("config".to_string(), config);
    }
    register_patch_json(
        &format!("/api/email-services/{service_id}"),
        &Value::Object(payload),
    )
}

pub(crate) fn delete_register_email_service(service_id: i64) -> Result<Value, String> {
    if service_id < 1 {
        return Err("serviceId is required".to_string());
    }
    register_delete_json(&format!("/api/email-services/{service_id}"))
}

pub(crate) fn test_register_email_service(service_id: i64) -> Result<Value, String> {
    if service_id < 1 {
        return Err("serviceId is required".to_string());
    }
    register_post_json(
        &format!("/api/email-services/{service_id}/test"),
        &json!({}),
    )
}

pub(crate) fn set_register_email_service_enabled(
    service_id: i64,
    enabled: bool,
) -> Result<Value, String> {
    if service_id < 1 {
        return Err("serviceId is required".to_string());
    }
    let action = if enabled { "enable" } else { "disable" };
    register_post_json(
        &format!("/api/email-services/{service_id}/{action}"),
        &json!({}),
    )
}

pub(crate) fn batch_import_register_outlook(
    data: &str,
    enabled: bool,
    priority: i64,
) -> Result<Value, String> {
    let data = data.trim();
    if data.is_empty() {
        return Err("data is required".to_string());
    }
    register_post_json(
        "/api/email-services/outlook/batch-import",
        &json!({
            "data": data,
            "enabled": enabled,
            "priority": priority.max(0),
        }),
    )
}

pub(crate) fn batch_delete_register_outlook(service_ids: Vec<i64>) -> Result<Value, String> {
    let ids = service_ids
        .into_iter()
        .filter(|service_id| *service_id > 0)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("serviceIds is required".to_string());
    }
    register_delete_json_with_body("/api/email-services/outlook/batch", &json!(ids))
}

pub(crate) fn reorder_register_email_services(service_ids: Vec<i64>) -> Result<Value, String> {
    let ids = service_ids
        .into_iter()
        .filter(|service_id| *service_id > 0)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("serviceIds is required".to_string());
    }
    register_post_json("/api/email-services/reorder", &json!(ids))
}

pub(crate) fn test_register_tempmail(api_url: Option<&str>) -> Result<Value, String> {
    let api_url = api_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    register_post_json(
        "/api/email-services/test-tempmail",
        &json!({
            "api_url": api_url,
        }),
    )
}

pub(crate) fn read_register_task(task_uuid: &str) -> Result<RegisterTaskReadResponse, String> {
    let task_id = task_uuid.trim();
    if task_id.is_empty() {
        return Err("taskUuid is required".to_string());
    }
    let task = register_get_json(&format!("/api/registration/tasks/{task_id}"))?;
    let logs_payload = register_get_json(&format!("/api/registration/tasks/{task_id}/logs"))?;
    let status = task_status(&task);
    let email = task_result_email(&task);
    let logs = task_logs(&logs_payload);
    let joined_logs = logs.join("\n");
    let error_message = task_string_field(&task, "error_message");
    let result = task.get("result").cloned().unwrap_or(Value::Null);
    let can_import = status.eq_ignore_ascii_case("completed") && email.is_some();
    let imported_account_id = crate::storage_helpers::open_storage().and_then(|storage| {
        resolve_existing_imported_account_id(&storage, email.as_deref(), &result)
    });
    let is_imported = imported_account_id.is_some();
    let failure_reason = if matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "failed" | "cancelled"
    ) {
        classify_register_failure_reason(error_message.as_deref(), joined_logs.as_str())
    } else {
        None
    };
    Ok(RegisterTaskReadResponse {
        task_uuid: task_id.to_string(),
        status: status.clone(),
        email_service_id: task_i64_field(&task, "email_service_id"),
        proxy: task_string_field(&task, "proxy"),
        created_at: task_string_field(&task, "created_at"),
        started_at: task_string_field(&task, "started_at"),
        completed_at: task_string_field(&task, "completed_at"),
        error_message,
        failure_code: failure_reason.map(|(code, _)| code.to_string()),
        failure_label: failure_reason.map(|(_, label)| label.to_string()),
        email: email.clone(),
        can_import,
        imported_account_id,
        is_imported,
        requires_manual_import: can_import && !is_imported,
        result,
        logs,
    })
}

pub(crate) fn import_register_task(task_uuid: &str) -> Result<Value, String> {
    let task = read_register_task(task_uuid)?;
    if !task.status.eq_ignore_ascii_case("completed") {
        return Err("register task is not completed".to_string());
    }
    let email = task
        .email
        .clone()
        .ok_or_else(|| "register task result missing email".to_string())?;
    let chatgpt_account_id = task_result_string_field(
        &task.result,
        &[
            "account_id",
            "accountId",
            "chatgpt_account_id",
            "chatgptAccountId",
        ],
    );
    let workspace_id =
        task_result_string_field(&task.result, &["workspace_id", "workspaceId"]);

    let mut imported = import_remote_account_for_email(&email, chatgpt_account_id, workspace_id)?;
    if let Some(object) = imported.as_object_mut() {
        object.insert(
            "taskUuid".to_string(),
            Value::String(task_uuid.trim().to_string()),
        );
    }
    Ok(imported)
}

pub(crate) fn import_register_account_by_email(email: &str) -> Result<Value, String> {
    import_remote_account_for_email(email, None, None)
}

fn resolve_auto_register_recovery_plan() -> Result<AutoRegisterRecoveryPlan, String> {
    let payload = available_register_services()?;

    if recovery_group_available(&payload, &["tempmail"]) {
        return Ok(AutoRegisterRecoveryPlan {
            service_type: "tempmail".to_string(),
            email_service_id: None,
        });
    }
    if let Some(id) = first_recovery_service_id_from_group(
        &payload,
        &["customDomain", "custom_domain", "custom-domain"],
    ) {
        return Ok(AutoRegisterRecoveryPlan {
            service_type: "custom_domain".to_string(),
            email_service_id: Some(id),
        });
    }
    if let Some(id) = first_recovery_service_id_from_group(&payload, &["outlook"]) {
        return Ok(AutoRegisterRecoveryPlan {
            service_type: "outlook".to_string(),
            email_service_id: Some(id),
        });
    }
    if let Some(id) = first_recovery_service_id_from_group(&payload, &["tempMail", "temp_mail"]) {
        return Ok(AutoRegisterRecoveryPlan {
            service_type: "temp_mail".to_string(),
            email_service_id: Some(id),
        });
    }

    Err("no available email service for register auto login".to_string())
}

fn recovery_group_available(payload: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        payload
            .get(*key)
            .and_then(Value::as_object)
            .and_then(|group| group.get("available"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn first_recovery_service_id_from_group(payload: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_object)
            .and_then(|group| group.get("services"))
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find_map(|item| {
                    let id = item.get("id").and_then(Value::as_i64)?;
                    (id > 0).then_some(id)
                })
            })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_auto_register_batch_payload, classify_register_failure_reason,
        normalized_register_service_url,
        pick_remote_account_by_email, resolve_existing_imported_account_id_from_accounts,
        task_result_string_field,
        AutoRegisterRecoveryPlan, RegisterProxyItem,
    };
    use codexmanager_core::storage::{now_ts, Account};
    use serde_json::json;

    #[test]
    fn normalized_register_service_url_uses_default_and_trims_slash() {
        assert_eq!(
            normalized_register_service_url(None),
            "http://127.0.0.1:9000"
        );
        assert_eq!(
            normalized_register_service_url(Some(" http://example.com:8000/ ")),
            "http://example.com:8000"
        );
    }

    #[test]
    fn pick_remote_account_by_email_matches_case_insensitively() {
        let items = vec![
            serde_json::json!({"id": 1, "email": "first@example.com"}),
            serde_json::json!({"id": 2, "email": "Target@Example.com"}),
        ];
        let picked = pick_remote_account_by_email(&items, "target@example.com")
            .expect("account should match");
        assert_eq!(picked.get("id").and_then(|value| value.as_i64()), Some(2));
    }

    #[test]
    fn task_result_string_field_accepts_camel_case_aliases() {
        let task_result = json!({
            "accountId": "chatgpt-camel-id",
            "workspaceId": "workspace-camel-id",
            "chatgptAccountId": "chatgpt-account-camel-id"
        });

        assert_eq!(
            task_result_string_field(
                &task_result,
                &["account_id", "accountId", "chatgpt_account_id", "chatgptAccountId"],
            )
            .as_deref(),
            Some("chatgpt-camel-id")
        );
        assert_eq!(
            task_result_string_field(&task_result, &["workspace_id", "workspaceId"]).as_deref(),
            Some("workspace-camel-id")
        );
    }

    #[test]
    fn classify_register_failure_reason_detects_phone_required() {
        assert_eq!(
            classify_register_failure_reason(
                Some("OpenAI 注册后进入手机号验证，且登录回退未能拿到 OAuth 回调"),
                "",
            ),
            Some(("register_phone_required", "注册触发手机号验证"))
        );
    }

    #[test]
    fn classify_register_failure_reason_detects_email_otp_timeout() {
        assert_eq!(
            classify_register_failure_reason(None, "等待验证码超时"),
            Some(("register_email_otp_timeout", "邮箱验证码超时"))
        );
    }

    #[test]
    fn classify_register_failure_reason_detects_proxy_error() {
        assert_eq!(
            classify_register_failure_reason(Some("proxy authentication required"), ""),
            Some(("register_proxy_error", "注册代理异常"))
        );
    }

    #[test]
    fn auto_register_batch_payload_uses_any_auto_mode() {
        let payload = build_auto_register_batch_payload(&AutoRegisterRecoveryPlan {
            service_type: "custom_domain".to_string(),
            email_service_id: Some(12),
        });

        assert_eq!(
            payload.get("register_mode").and_then(|value| value.as_str()),
            Some("any_auto")
        );
        assert_eq!(
            payload.get("email_service_type").and_then(|value| value.as_str()),
            Some("custom_domain")
        );
        assert_eq!(
            payload.get("email_service_id").and_then(|value| value.as_i64()),
            Some(12)
        );
    }

    #[test]
    fn register_proxy_item_defaults_missing_is_default_to_false() {
        let item = serde_json::from_value::<RegisterProxyItem>(json!({
            "id": 1,
            "name": "pool-a",
            "type": "socks5",
            "host": "127.0.0.1",
            "port": 1080,
            "username": "tester",
            "enabled": true,
            "priority": 5
        }))
        .expect("register proxy item should parse without isDefault");

        assert!(!item.is_default);
    }

    #[test]
    fn register_proxy_item_accepts_is_default_alias() {
        let item = serde_json::from_value::<RegisterProxyItem>(json!({
            "id": 2,
            "name": "pool-b",
            "type": "http",
            "host": "127.0.0.2",
            "port": 8080,
            "enabled": true,
            "is_default": true,
            "priority": 1
        }))
        .expect("register proxy item should parse snake_case is_default");

        assert!(item.is_default);
    }

    #[test]
    fn resolve_existing_imported_account_id_matches_identity_hints() {
        let now = now_ts();
        let accounts = vec![Account {
            id: "acc-imported-1".to_string(),
            label: "imported@example.com".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("acct-1".to_string()),
            workspace_id: Some("org-1".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        }];

        let matched = resolve_existing_imported_account_id_from_accounts(
            accounts.as_slice(),
            Some("imported@example.com"),
            &serde_json::json!({
                "account_id": "acct-1",
                "workspace_id": "org-1"
            }),
        );
        assert_eq!(matched.as_deref(), Some("acc-imported-1"));
    }

    #[test]
    fn resolve_existing_imported_account_id_falls_back_to_unique_email_label() {
        let now = now_ts();
        let accounts = vec![Account {
            id: "acc-imported-2".to_string(),
            label: "fallback@example.com".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        }];

        let matched = resolve_existing_imported_account_id_from_accounts(
            accounts.as_slice(),
            Some("fallback@example.com"),
            &serde_json::json!({}),
        );
        assert_eq!(matched.as_deref(), Some("acc-imported-2"));
    }
}
