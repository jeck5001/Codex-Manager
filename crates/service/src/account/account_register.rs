use serde::Serialize;
use serde_json::{json, Value};
use std::thread;
use std::time::Duration;

const ENV_REGISTER_SERVICE_URL: &str = "CODEXMANAGER_REGISTER_SERVICE_URL";
const DEFAULT_REGISTER_SERVICE_URL: &str = "http://127.0.0.1:8000";
const REGISTER_BATCH_AUTO_IMPORT_POLL_INTERVAL_SECS: u64 = 3;
const REGISTER_BATCH_AUTO_IMPORT_TIMEOUT_SECS: u64 = 30 * 60;

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
    email: Option<String>,
    can_import: bool,
    result: Value,
    logs: Vec<String>,
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
        return Err(format!("register service http {}: {}", status.as_u16(), snippet));
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

fn import_remote_account_for_email(
    email: &str,
    chatgpt_account_id_hint: Option<String>,
    workspace_id_hint: Option<String>,
) -> Result<Value, String> {
    let normalized_email = email.trim();
    if normalized_email.is_empty() {
        return Err("email is required".to_string());
    }

    let remote_account = resolve_remote_account_for_email(normalized_email)?;
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
    let chatgpt_account_id =
        remote_account_string_field(&remote_account, "account_id").or(chatgpt_account_id_hint);
    let workspace_id =
        remote_account_string_field(&remote_account, "workspace_id").or(workspace_id_hint);

    let imported = crate::auth_account::login_with_chatgpt_auth_tokens(
        crate::auth_account::ChatgptAuthTokensLoginInput {
            access_token,
            refresh_token,
            id_token,
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
    proxy: Option<String>,
) -> Result<Value, String> {
    let service_type = email_service_type.trim();
    if service_type.is_empty() {
        return Err("emailServiceType is required".to_string());
    }
    register_post_json(
        "/api/registration/start",
        &json!({
            "email_service_type": service_type,
            "email_service_id": email_service_id,
            "proxy": proxy,
        }),
    )
}

pub(crate) fn start_register_batch(
    email_service_type: &str,
    email_service_id: Option<i64>,
    proxy: Option<String>,
    count: i64,
    interval_min: i64,
    interval_max: i64,
    concurrency: i64,
    mode: &str,
) -> Result<Value, String> {
    let service_type = email_service_type.trim();
    if service_type.is_empty() {
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

    let mut payload = register_post_json(
        "/api/registration/batch",
        &json!({
            "email_service_type": service_type,
            "email_service_id": email_service_id,
            "proxy": proxy,
            "count": count,
            "interval_min": interval_min,
            "interval_max": interval_max,
            "concurrency": concurrency,
            "mode": normalized_mode,
        }),
    )?;
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
    register_post_json(&format!("/api/registration/batch/{batch_id}/cancel"), &json!({}))
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
    register_get_json_with_query("/api/registration/tasks", &query)
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

pub(crate) fn delete_register_task(task_uuid: &str) -> Result<Value, String> {
    let task_uuid = task_uuid.trim();
    if task_uuid.is_empty() {
        return Err("taskUuid is required".to_string());
    }
    register_delete_json(&format!("/api/registration/tasks/{task_uuid}"))
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

pub(crate) fn register_email_service_stats() -> Result<Value, String> {
    register_get_json("/api/email-services/stats")
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
        payload.insert("priority".to_string(), Value::Number(priority.max(0).into()));
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
    register_post_json(&format!("/api/email-services/{service_id}/test"), &json!({}))
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
    Ok(RegisterTaskReadResponse {
        task_uuid: task_id.to_string(),
        status: status.clone(),
        email_service_id: task_i64_field(&task, "email_service_id"),
        proxy: task_string_field(&task, "proxy"),
        created_at: task_string_field(&task, "created_at"),
        started_at: task_string_field(&task, "started_at"),
        completed_at: task_string_field(&task, "completed_at"),
        error_message: task_string_field(&task, "error_message"),
        email: email.clone(),
        can_import: status.eq_ignore_ascii_case("completed") && email.is_some(),
        result: task.get("result").cloned().unwrap_or(Value::Null),
        logs: task_logs(&logs_payload),
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
    let chatgpt_account_id = task
        .result
        .get("account_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let workspace_id = task
        .result
        .get("workspace_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

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

#[cfg(test)]
mod tests {
    use super::{normalized_register_service_url, pick_remote_account_by_email};

    #[test]
    fn normalized_register_service_url_uses_default_and_trims_slash() {
        assert_eq!(
            normalized_register_service_url(None),
            "http://127.0.0.1:8000"
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
}
