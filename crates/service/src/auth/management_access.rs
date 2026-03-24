use crate::app_settings::{
    get_persisted_app_setting, normalize_optional_text, save_persisted_app_setting,
    APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY,
};

fn current_env_remote_management_secret() -> Option<String> {
    normalize_optional_text(
        std::env::var("CODEXMANAGER_REMOTE_MANAGEMENT_SECRET")
            .ok()
            .as_deref(),
    )
}

pub fn current_remote_management_secret_hash() -> Option<String> {
    get_persisted_app_setting(APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY)
        .and_then(|value| normalize_optional_text(Some(&value)))
}

pub fn remote_management_secret_configured() -> bool {
    current_env_remote_management_secret().is_some()
        || current_remote_management_secret_hash().is_some()
}

pub fn set_remote_management_secret(secret: Option<&str>) -> Result<bool, String> {
    match normalize_optional_text(secret) {
        Some(value) => {
            let hashed = super::secret_store::hash_secret(&value);
            save_persisted_app_setting(
                APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY,
                Some(&hashed),
            )?;
            Ok(true)
        }
        None => {
            save_persisted_app_setting(APP_SETTING_REMOTE_MANAGEMENT_SECRET_HASH_KEY, Some(""))?;
            Ok(false)
        }
    }
}

pub fn verify_remote_management_secret(candidate: &str) -> bool {
    let normalized_candidate = candidate.trim();
    if normalized_candidate.is_empty() {
        return false;
    }
    if let Some(env_secret) = current_env_remote_management_secret() {
        return super::rpc::constant_time_eq(
            env_secret.as_bytes(),
            normalized_candidate.as_bytes(),
        );
    }
    let Some(stored_hash) = current_remote_management_secret_hash() else {
        return false;
    };
    super::secret_store::verify_secret_hash(normalized_candidate, &stored_hash)
}
