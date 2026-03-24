use super::{
    get_persisted_app_setting, parse_bool_with_default, save_persisted_bool_setting,
    APP_SETTING_REMOTE_MANAGEMENT_ENABLED_KEY,
};

fn current_env_remote_management_enabled() -> Option<bool> {
    let raw = std::env::var("CODEXMANAGER_REMOTE_MANAGEMENT_ENABLED").ok()?;
    Some(parse_bool_with_default(&raw, false))
}

pub fn current_remote_management_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_REMOTE_MANAGEMENT_ENABLED_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .or_else(current_env_remote_management_enabled)
        .unwrap_or(false)
}

pub fn set_remote_management_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_REMOTE_MANAGEMENT_ENABLED_KEY, enabled)?;
    Ok(enabled)
}
