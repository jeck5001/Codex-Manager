use codexmanager_core::auth::{
    build_authorize_url, device_redirect_uri, device_token_url, device_usercode_url,
    device_verification_url, generate_pkce, generate_state, parse_id_token_claims,
    DEFAULT_CLIENT_ID, DEFAULT_ISSUER,
};
use codexmanager_core::rpc::types::{AccountAuthRecoveryResult, DeviceAuthInfo, LoginStartResult};
use codexmanager_core::storage::{now_ts, Account, Event, LoginSession, Storage};

use crate::auth_callback::{ensure_login_server, resolve_redirect_uri};
use crate::storage_helpers::open_storage;

fn recovered_account_result(account_id: String) -> AccountAuthRecoveryResult {
    AccountAuthRecoveryResult {
        status: "recovered".to_string(),
        account_id,
        login_id: None,
        auth_url: None,
        warning: None,
    }
}

fn activate_recovered_account(storage: &Storage, account_id: &str) {
    crate::account_status::set_account_status(storage, account_id, "active", "auth_recovered");
}

fn imported_account_id(payload: &serde_json::Value) -> Result<String, String> {
    payload
        .get("accountId")
        .or_else(|| payload.get("account_id"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "recovery import result missing accountId".to_string())
}

pub(crate) fn login_start(
    login_type: &str,
    open_browser: bool,
    note: Option<String>,
    tags: Option<String>,
    group_name: Option<String>,
    workspace_id: Option<String>,
) -> Result<LoginStartResult, String> {
    // 读取登录相关配置
    let issuer =
        std::env::var("CODEXMANAGER_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.to_string());
    let client_id =
        std::env::var("CODEXMANAGER_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
    let originator = crate::gateway::current_originator();
    if login_type != "device" {
        ensure_login_server()?;
    }
    let redirect_uri = if login_type == "device" {
        std::env::var("CODEXMANAGER_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:1455/auth/callback".to_string())
    } else {
        resolve_redirect_uri().unwrap_or_else(|| "http://localhost:1455/auth/callback".to_string())
    };

    // 生成 PKCE 与状态
    let pkce = generate_pkce();
    let state = generate_state();

    // 写入登录会话
    if let Some(storage) = open_storage() {
        let _ = storage.insert_login_session(&LoginSession {
            login_id: state.clone(),
            code_verifier: pkce.code_verifier.clone(),
            state: state.clone(),
            status: "pending".to_string(),
            error: None,
            workspace_id: workspace_id.clone(),
            note,
            tags,
            group_name,
            created_at: now_ts(),
            updated_at: now_ts(),
        });
    }

    // 构造登录地址
    let auth_url = if login_type == "device" {
        device_verification_url(&issuer)
    } else {
        build_authorize_url(
            &issuer,
            &client_id,
            &redirect_uri,
            &pkce.code_challenge,
            &state,
            &originator,
            workspace_id.as_deref(),
        )
    };

    // 设备登录信息
    let device = if login_type == "device" {
        Some(DeviceAuthInfo {
            user_code_url: device_usercode_url(&issuer),
            token_url: device_token_url(&issuer),
            verification_url: device_verification_url(&issuer),
            redirect_uri: device_redirect_uri(&issuer),
        })
    } else {
        None
    };

    // 写入事件日志
    if let Some(storage) = open_storage() {
        let _ = storage.insert_event(&Event {
            account_id: None,
            event_type: "login_start".to_string(),
            message: format!(
                "{{\"login_id\":\"{}\",\"code_verifier\":\"{}\"}}",
                state, pkce.code_verifier
            ),
            created_at: now_ts(),
        });
    }

    // 可选自动打开浏览器
    if login_type != "device" && open_browser {
        let _ = webbrowser::open(&auth_url);
    }

    Ok(LoginStartResult {
        auth_url,
        login_id: state,
        login_type: login_type.to_string(),
        issuer,
        client_id,
        redirect_uri,
        warning: None,
        device,
    })
}

pub(crate) fn login_status(login_id: &str) -> serde_json::Value {
    // 查询登录会话状态
    if login_id.is_empty() {
        return serde_json::json!({ "status": "unknown" });
    }
    let storage = match open_storage() {
        Some(storage) => storage,
        None => return serde_json::json!({ "status": "unknown" }),
    };
    let session = match storage.get_login_session(login_id) {
        Ok(Some(session)) => session,
        _ => return serde_json::json!({ "status": "unknown" }),
    };
    serde_json::json!({
        "status": session.status,
        "error": session.error,
        "updatedAt": session.updated_at
    })
}

pub(crate) fn recover_account_auth(
    account_id: &str,
    _open_browser: bool,
) -> Result<AccountAuthRecoveryResult, String> {
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("missing accountId".to_string());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let account = storage
        .find_account_by_id(normalized_account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("account not found: {normalized_account_id}"))?;

    if crate::auth_account::refresh_chatgpt_auth_tokens_for_account(normalized_account_id).is_ok() {
        activate_recovered_account(&storage, &account.id);
        return Ok(recovered_account_result(account.id));
    }

    let recovery_email = resolve_account_recovery_email(&storage, &account);
    let mut recovery_errors = Vec::new();

    if crate::account_recovery_source::recovery_source_configured() {
        match crate::account_recovery_source::import_remote_recovery_account(
            recovery_email.as_deref(),
            account.chatgpt_account_id.as_deref(),
            account.workspace_id.as_deref(),
        ) {
            Ok(imported) => {
                let recovered_account_id = imported_account_id(&imported)?;
                activate_recovered_account(&storage, recovered_account_id.as_str());
                return Ok(recovered_account_result(recovered_account_id));
            }
            Err(err) => recovery_errors.push(err),
        }
    }

    if let Some(email) = recovery_email.as_deref() {
        match crate::account_register::refresh_and_import_register_account_by_email(
            email,
            account.chatgpt_account_id.clone(),
            account.workspace_id.clone(),
        ) {
            Ok(imported) => {
                let recovered_account_id = imported_account_id(&imported)?;
                activate_recovered_account(&storage, recovered_account_id.as_str());
                return Ok(recovered_account_result(recovered_account_id));
            }
            Err(err) => recovery_errors.push(err),
        }
    } else {
        recovery_errors.push(format!("account {} missing recoverable email", account.id));
    }

    match crate::account_register::auto_register_and_import_account() {
        Ok(imported) => {
            let recovered_account_id = imported_account_id(&imported)?;
            activate_recovered_account(&storage, recovered_account_id.as_str());
            return Ok(recovered_account_result(recovered_account_id));
        }
        Err(err) => recovery_errors.push(err),
    }

    Err(recovery_errors
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("account {} missing recoverable email", account.id)))
}

fn resolve_account_recovery_email(storage: &Storage, account: &Account) -> Option<String> {
    let label = account.label.trim();
    if label.contains('@') {
        return Some(label.to_string());
    }

    let token = storage
        .find_token_by_account_id(&account.id)
        .ok()
        .flatten()?;
    for raw_token in [&token.id_token, &token.access_token] {
        let normalized = raw_token.trim();
        if normalized.is_empty() {
            continue;
        }
        if let Ok(claims) = parse_id_token_claims(normalized) {
            if let Some(email) = claims.email.as_deref() {
                let normalized_email = email.trim();
                if !normalized_email.is_empty() {
                    return Some(normalized_email.to_string());
                }
            }
        }
    }

    None
}
