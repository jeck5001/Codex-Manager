use crate::app_settings::{
    get_persisted_app_setting, normalize_optional_text, save_persisted_app_setting,
    APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY,
};
use serde_json::Value;

pub fn current_web_access_password_hash() -> Option<String> {
    get_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY)
}

pub fn web_access_password_configured() -> bool {
    current_web_access_password_hash().is_some()
}

pub fn set_web_access_password(password: Option<&str>) -> Result<bool, String> {
    match normalize_optional_text(password) {
        Some(value) => {
            let hashed = super::secret_store::hash_secret(&value);
            save_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY, Some(&hashed))?;
            Ok(true)
        }
        None => {
            crate::clear_web_access_two_factor()?;
            save_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY, Some(""))?;
            Ok(false)
        }
    }
}

pub fn web_auth_status_value() -> Result<Value, String> {
    Ok(serde_json::json!({
        "passwordConfigured": web_access_password_configured(),
        "twoFactorEnabled": crate::web_auth_two_factor_enabled(),
        "recoveryCodesRemaining": crate::auth::web_access_2fa::web_auth_two_factor_recovery_codes_remaining(),
    }))
}

pub fn verify_web_access_password(password: &str) -> bool {
    let Some(stored_hash) = current_web_access_password_hash() else {
        return true;
    };
    super::secret_store::verify_secret_hash(password, &stored_hash)
}

pub fn build_web_access_session_token(password_hash: &str, rpc_token: &str) -> String {
    hex_sha256(format!("codexmanager-web-auth-session:{password_hash}:{rpc_token}").as_bytes())
}

fn hex_sha256(bytes: impl AsRef<[u8]>) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    let digest = hasher.finalize();
    hex_encode(digest.as_slice())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
