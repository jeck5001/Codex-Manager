use super::{
    get_persisted_app_setting, parse_bool_with_default, save_persisted_app_setting,
    save_persisted_bool_setting, APP_SETTING_MCP_ENABLED_KEY, APP_SETTING_MCP_PORT_KEY,
};

pub const DEFAULT_MCP_PORT: u16 = 48762;

fn normalize_mcp_port(raw: Option<&str>) -> Result<u16, String> {
    let value = raw.unwrap_or_default().trim();
    if value.is_empty() {
        return Ok(DEFAULT_MCP_PORT);
    }
    let port = value
        .parse::<u16>()
        .map_err(|err| format!("invalid MCP port: {err}"))?;
    if port == 0 {
        return Err("mcpPort must be between 1 and 65535".to_string());
    }
    Ok(port)
}

fn current_env_mcp_enabled() -> Option<bool> {
    let raw = std::env::var("CODEXMANAGER_MCP_ENABLED").ok()?;
    Some(parse_bool_with_default(&raw, true))
}

fn current_env_mcp_port() -> Option<u16> {
    let raw = std::env::var("CODEXMANAGER_MCP_PORT").ok()?;
    normalize_mcp_port(Some(&raw)).ok()
}

pub fn current_mcp_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_MCP_ENABLED_KEY)
        .map(|value| parse_bool_with_default(&value, true))
        .or_else(current_env_mcp_enabled)
        .unwrap_or(true)
}

pub fn set_mcp_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_MCP_ENABLED_KEY, enabled)?;
    Ok(enabled)
}

pub fn current_mcp_port() -> u16 {
    get_persisted_app_setting(APP_SETTING_MCP_PORT_KEY)
        .and_then(|value| normalize_mcp_port(Some(&value)).ok())
        .or_else(current_env_mcp_port)
        .unwrap_or(DEFAULT_MCP_PORT)
}

pub fn set_mcp_port(port: u16) -> Result<u16, String> {
    if port == 0 {
        return Err("mcpPort must be between 1 and 65535".to_string());
    }
    let port_text = port.to_string();
    save_persisted_app_setting(APP_SETTING_MCP_PORT_KEY, Some(&port_text))?;
    Ok(port)
}
