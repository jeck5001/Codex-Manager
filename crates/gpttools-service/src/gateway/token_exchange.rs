use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpttools_core::auth::{DEFAULT_CLIENT_ID, DEFAULT_ISSUER};
use gpttools_core::storage::{Account, Storage, Token};

use crate::auth_tokens;
use crate::usage_http::refresh_access_token;

static ACCOUNT_TOKEN_EXCHANGE_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    OnceLock::new();

pub(super) fn account_token_exchange_lock(account_id: &str) -> Arc<Mutex<()>> {
    let lock = ACCOUNT_TOKEN_EXCHANGE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = lock.lock() else {
        return Arc::new(Mutex::new(()));
    };
    map.entry(account_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn find_cached_api_key_access_token(storage: &Storage, account_id: &str) -> Option<String> {
    storage
        .list_tokens()
        .ok()?
        .into_iter()
        .find(|t| t.account_id == account_id)
        .and_then(|t| t.api_key_access_token)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn exchange_and_persist_api_key_access_token(
    storage: &Storage,
    token: &mut Token,
    issuer: &str,
    client_id: &str,
) -> Result<String, String> {
    let exchanged = auth_tokens::obtain_api_key(issuer, client_id, &token.id_token)?;
    token.api_key_access_token = Some(exchanged.clone());
    let _ = storage.insert_token(token);
    Ok(exchanged)
}

fn fallback_to_access_token(token: &Token, exchange_error: &str) -> Result<String, String> {
    let fallback = token.access_token.trim();
    if fallback.is_empty() {
        return Err(exchange_error.to_string());
    }
    log::warn!(
        "api_key_access_token exchange unavailable; fallback to access_token: {}",
        exchange_error
    );
    Ok(fallback.to_string())
}

pub(super) fn resolve_openai_bearer_token(
    storage: &Storage,
    account: &Account,
    token: &mut Token,
) -> Result<String, String> {
    if let Some(existing) = token
        .api_key_access_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Ok(existing.to_string());
    }

    let exchange_lock = account_token_exchange_lock(&account.id);
    let _guard = exchange_lock
        .lock()
        .map_err(|_| "token exchange lock poisoned".to_string())?;

    if let Some(existing) = token
        .api_key_access_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Ok(existing.to_string());
    }

    if let Some(cached) = find_cached_api_key_access_token(storage, &account.id) {
        // 中文注释：并发下后到线程优先复用已落库的新 token，避免重复 token exchange 打上游。
        token.api_key_access_token = Some(cached.clone());
        return Ok(cached);
    }

    let client_id =
        std::env::var("GPTTOOLS_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
    let issuer_env = std::env::var("GPTTOOLS_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.to_string());
    let issuer = if account.issuer.trim().is_empty() {
        issuer_env
    } else {
        account.issuer.clone()
    };

    match exchange_and_persist_api_key_access_token(storage, token, &issuer, &client_id) {
        Ok(token) => return Ok(token),
        Err(exchange_err) => {
            if !token.refresh_token.trim().is_empty() {
                match refresh_access_token(&issuer, &client_id, &token.refresh_token) {
                    Ok(refreshed) => {
                        token.access_token = refreshed.access_token;
                        if let Some(refresh_token) = refreshed.refresh_token {
                            token.refresh_token = refresh_token;
                        }
                        if let Some(id_token) = refreshed.id_token {
                            token.id_token = id_token;
                        }
                        let _ = storage.insert_token(token);

                        if !token.id_token.trim().is_empty() {
                            if let Ok(exchanged) = exchange_and_persist_api_key_access_token(
                                storage, token, &issuer, &client_id,
                            ) {
                                return Ok(exchanged);
                            }
                        }
                    }
                    Err(refresh_err) => {
                        log::warn!(
                            "refresh token before api_key_access_token exchange failed: {}",
                            refresh_err
                        );
                    }
                }
            }

            fallback_to_access_token(token, &exchange_err)
        }
    }
}
