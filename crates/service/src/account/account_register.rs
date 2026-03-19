use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

const ENV_REGISTER_SERVICE_URL: &str = "CODEXMANAGER_REGISTER_SERVICE_URL";
const DEFAULT_REGISTER_SERVICE_URL: &str = "http://127.0.0.1:8000";

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

    let remote_account = resolve_remote_account_for_email(&email)?;
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
    let chatgpt_account_id = remote_account_string_field(&remote_account, "account_id")
        .or_else(|| {
            task.result
                .get("account_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        });
    let workspace_id = remote_account_string_field(&remote_account, "workspace_id").or_else(|| {
        task.result
            .get("workspace_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    });

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
        "taskUuid": task_uuid.trim(),
        "email": email,
        "remoteAccountId": remote_account_id,
        "accountId": imported.account_id,
        "chatgptAccountId": imported.chatgpt_account_id,
        "workspaceId": imported.workspace_id,
        "type": imported.kind,
    }))
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
