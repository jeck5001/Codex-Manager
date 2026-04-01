use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

const ENV_ACCOUNT_RECOVERY_SOURCE_URL: &str = "CODEXMANAGER_ACCOUNT_RECOVERY_SOURCE_URL";

#[derive(Debug, Deserialize)]
struct RpcEnvelope {
    result: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteAccountListResult {
    items: Vec<RemoteAccountSummary>,
}

#[derive(Debug, Deserialize)]
struct RemoteAccountSummary {
    id: String,
    label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteExportDataResult {
    files: Vec<RemoteExportFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteExportFile {
    content: String,
}

#[derive(Debug, Deserialize)]
struct RemoteExportPayload {
    tokens: RemoteExportTokens,
    meta: RemoteExportMeta,
}

#[derive(Debug, Deserialize)]
struct RemoteExportTokens {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    id_token: String,
    #[serde(default)]
    cookies: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteExportMeta {
    label: String,
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

pub(crate) fn recovery_source_configured() -> bool {
    current_recovery_source_url().is_some()
}

pub(crate) fn import_remote_recovery_account(
    email_hint: Option<&str>,
    chatgpt_account_id_hint: Option<&str>,
    workspace_id_hint: Option<&str>,
) -> Result<Value, String> {
    let candidate_ids = match normalized_hint(email_hint) {
        Some(email) => find_remote_account_ids_by_email(email)?,
        None => Vec::new(),
    };

    let exports = if candidate_ids.is_empty() {
        export_remote_accounts(None)?
    } else {
        export_remote_accounts(Some(candidate_ids.as_slice()))?
    };
    let export = pick_matching_export(
        exports.as_slice(),
        email_hint,
        chatgpt_account_id_hint,
        workspace_id_hint,
    )
    .ok_or_else(|| {
        format!(
            "account recovery source account not found for identity: {}",
            describe_identity(email_hint, chatgpt_account_id_hint, workspace_id_hint)
        )
    })?;

    let imported = crate::auth_account::login_with_chatgpt_auth_tokens(
        crate::auth_account::ChatgptAuthTokensLoginInput {
            access_token: export.tokens.access_token.clone(),
            refresh_token: Some(export.tokens.refresh_token.clone())
                .filter(|value| !value.trim().is_empty()),
            id_token: Some(export.tokens.id_token.clone()).filter(|value| !value.trim().is_empty()),
            cookies: export
                .tokens
                .cookies
                .clone()
                .filter(|value| !value.trim().is_empty()),
            email_hint: Some(export.meta.label.clone()),
            chatgpt_account_id: export.meta.chatgpt_account_id.clone(),
            workspace_id: export.meta.workspace_id.clone(),
            chatgpt_plan_type: None,
        },
    )?;

    Ok(json!({
        "email": export.meta.label,
        "accountId": imported.account_id,
        "chatgptAccountId": imported.chatgpt_account_id,
        "workspaceId": imported.workspace_id,
        "type": imported.kind,
    }))
}

fn normalized_hint(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn current_recovery_source_url() -> Option<String> {
    std::env::var(ENV_ACCOUNT_RECOVERY_SOURCE_URL)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn recovery_source_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("build account recovery source client failed: {err}"))
}

fn recovery_source_rpc(method: &str, params: Value) -> Result<Value, String> {
    let base_url = current_recovery_source_url()
        .ok_or_else(|| "account recovery source url not configured".to_string())?;
    let client = recovery_source_http_client()?;
    let response = client
        .post(format!("{base_url}/api/rpc"))
        .json(&json!({
            "id": 1,
            "method": method,
            "params": params,
        }))
        .send()
        .map_err(|err| format!("request account recovery source failed: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .unwrap_or_else(|_| String::from("<unreadable body>"));
        let snippet = body.chars().take(300).collect::<String>();
        return Err(format!(
            "account recovery source http {}: {}",
            status.as_u16(),
            snippet
        ));
    }
    let envelope = response
        .json::<RpcEnvelope>()
        .map_err(|err| format!("parse account recovery source response failed: {err}"))?;
    Ok(envelope.result)
}

fn find_remote_account_ids_by_email(email: &str) -> Result<Vec<String>, String> {
    let result = recovery_source_rpc(
        "account/list",
        json!({
            "page": 1,
            "pageSize": 500,
            "query": email,
        }),
    )?;
    let list = serde_json::from_value::<RemoteAccountListResult>(result)
        .map_err(|err| format!("parse account recovery source account/list failed: {err}"))?;
    Ok(list
        .items
        .into_iter()
        .filter(|item| item.label.trim().eq_ignore_ascii_case(email.trim()))
        .map(|item| item.id)
        .collect())
}

fn export_remote_accounts(
    account_ids: Option<&[String]>,
) -> Result<Vec<RemoteExportPayload>, String> {
    let params = match account_ids {
        Some(ids) if !ids.is_empty() => json!({ "accountIds": ids }),
        _ => json!({}),
    };
    let result = recovery_source_rpc("account/exportData", params)?;
    let export = serde_json::from_value::<RemoteExportDataResult>(result)
        .map_err(|err| format!("parse account recovery source account/exportData failed: {err}"))?;
    export
        .files
        .into_iter()
        .map(|file| {
            serde_json::from_str::<RemoteExportPayload>(&file.content).map_err(|err| {
                format!("parse account recovery source export content failed: {err}")
            })
        })
        .collect()
}

fn pick_matching_export<'a>(
    exports: &'a [RemoteExportPayload],
    email_hint: Option<&str>,
    chatgpt_account_id_hint: Option<&str>,
    workspace_id_hint: Option<&str>,
) -> Option<&'a RemoteExportPayload> {
    find_export_by_identity(
        exports,
        chatgpt_account_id_hint,
        workspace_id_hint,
        email_hint,
        true,
    )
    .or_else(|| {
        find_export_by_identity(
            exports,
            chatgpt_account_id_hint,
            workspace_id_hint,
            email_hint,
            false,
        )
    })
}

fn find_export_by_identity<'a>(
    exports: &'a [RemoteExportPayload],
    chatgpt_account_id_hint: Option<&str>,
    workspace_id_hint: Option<&str>,
    email_hint: Option<&str>,
    require_identity: bool,
) -> Option<&'a RemoteExportPayload> {
    exports.iter().find(|item| {
        let chatgpt_matches = hints_match(
            item.meta.chatgpt_account_id.as_deref(),
            normalized_hint(chatgpt_account_id_hint),
        );
        let workspace_matches = hints_match(
            item.meta.workspace_id.as_deref(),
            normalized_hint(workspace_id_hint),
        );
        let email_matches =
            hints_match(Some(item.meta.label.as_str()), normalized_hint(email_hint));

        if require_identity {
            (chatgpt_matches && normalized_hint(chatgpt_account_id_hint).is_some())
                || (workspace_matches && normalized_hint(workspace_id_hint).is_some())
                || (chatgpt_matches
                    && workspace_matches
                    && normalized_hint(chatgpt_account_id_hint).is_some()
                    && normalized_hint(workspace_id_hint).is_some())
        } else {
            email_matches
        }
    })
}

fn hints_match(left: Option<&str>, right: Option<&str>) -> bool {
    match (normalized_hint(left), normalized_hint(right)) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => false,
    }
}

fn describe_identity(
    email_hint: Option<&str>,
    chatgpt_account_id_hint: Option<&str>,
    workspace_id_hint: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(email) = normalized_hint(email_hint) {
        parts.push(format!("email: {email}"));
    }
    if let Some(chatgpt_account_id) = normalized_hint(chatgpt_account_id_hint) {
        parts.push(format!("account_id: {chatgpt_account_id}"));
    }
    if let Some(workspace_id) = normalized_hint(workspace_id_hint) {
        parts.push(format!("workspace_id: {workspace_id}"));
    }
    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join(", ")
    }
}
