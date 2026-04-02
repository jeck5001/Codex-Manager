use super::parse_bool_with_default;
use super::{
    get_persisted_app_setting, save_persisted_app_setting, save_persisted_bool_setting,
    APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY, APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY,
    APP_SETTING_UI_APPEARANCE_PRESET_KEY, APP_SETTING_UI_LOW_TRANSPARENCY_KEY,
    APP_SETTING_UI_THEME_KEY, APP_SETTING_UPDATE_AUTO_CHECK_KEY,
};

const DEFAULT_UI_THEME: &str = "tech";
const DEFAULT_UI_APPEARANCE_PRESET: &str = "classic";
const VALID_UI_THEMES: &[&str] = &[
    "tech", "dark", "dark-one", "business", "mint", "sunset", "grape", "ocean", "forest", "rose",
    "slate", "aurora",
];
const VALID_UI_APPEARANCE_PRESETS: &[&str] = &["modern", "classic"];

/// 函数 `normalize_ui_theme`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn normalize_ui_theme(raw: Option<&str>) -> String {
    let candidate = raw.unwrap_or(DEFAULT_UI_THEME).trim().to_ascii_lowercase();
    if VALID_UI_THEMES.iter().any(|theme| *theme == candidate) {
        candidate
    } else {
        DEFAULT_UI_THEME.to_string()
    }
}

/// 函数 `normalize_ui_appearance_preset`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn normalize_ui_appearance_preset(raw: Option<&str>) -> String {
    let candidate = raw
        .unwrap_or(DEFAULT_UI_APPEARANCE_PRESET)
        .trim()
        .to_ascii_lowercase();
    if VALID_UI_APPEARANCE_PRESETS
        .iter()
        .any(|preset| *preset == candidate)
    {
        candidate
    } else {
        DEFAULT_UI_APPEARANCE_PRESET.to_string()
    }
}

/// 函数 `current_update_auto_check_enabled`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_update_auto_check_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_UPDATE_AUTO_CHECK_KEY)
        .map(|value| parse_bool_with_default(&value, true))
        .unwrap_or(true)
}

/// 函数 `set_update_auto_check_enabled`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
///
/// # 返回
/// 返回函数执行结果
pub fn set_update_auto_check_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_UPDATE_AUTO_CHECK_KEY, enabled)?;
    Ok(enabled)
}

/// 函数 `current_close_to_tray_on_close_setting`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_close_to_tray_on_close_setting() -> bool {
    get_persisted_app_setting(APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

/// 函数 `set_close_to_tray_on_close_setting`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
///
/// # 返回
/// 返回函数执行结果
pub fn set_close_to_tray_on_close_setting(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_CLOSE_TO_TRAY_ON_CLOSE_KEY, enabled)?;
    Ok(enabled)
}

/// 函数 `current_lightweight_mode_on_close_to_tray_setting`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_lightweight_mode_on_close_to_tray_setting() -> bool {
    get_persisted_app_setting(APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

/// 函数 `set_lightweight_mode_on_close_to_tray_setting`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
///
/// # 返回
/// 返回函数执行结果
pub fn set_lightweight_mode_on_close_to_tray_setting(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_LIGHTWEIGHT_MODE_ON_CLOSE_TO_TRAY_KEY, enabled)?;
    Ok(enabled)
}

/// 函数 `current_ui_low_transparency_enabled`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_ui_low_transparency_enabled() -> bool {
    get_persisted_app_setting(APP_SETTING_UI_LOW_TRANSPARENCY_KEY)
        .map(|value| parse_bool_with_default(&value, false))
        .unwrap_or(false)
}

/// 函数 `set_ui_low_transparency_enabled`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
///
/// # 返回
/// 返回函数执行结果
pub fn set_ui_low_transparency_enabled(enabled: bool) -> Result<bool, String> {
    save_persisted_bool_setting(APP_SETTING_UI_LOW_TRANSPARENCY_KEY, enabled)?;
    Ok(enabled)
}

/// 函数 `current_ui_theme`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_ui_theme() -> String {
    normalize_ui_theme(get_persisted_app_setting(APP_SETTING_UI_THEME_KEY).as_deref())
}

/// 函数 `set_ui_theme`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - theme: 参数 theme
///
/// # 返回
/// 返回函数执行结果
pub fn set_ui_theme(theme: Option<&str>) -> Result<String, String> {
    let normalized = normalize_ui_theme(theme);
    save_persisted_app_setting(APP_SETTING_UI_THEME_KEY, Some(&normalized))?;
    Ok(normalized)
}

/// 函数 `current_ui_appearance_preset`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_ui_appearance_preset() -> String {
    normalize_ui_appearance_preset(
        get_persisted_app_setting(APP_SETTING_UI_APPEARANCE_PRESET_KEY).as_deref(),
    )
}

/// 函数 `set_ui_appearance_preset`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - preset: 参数 preset
///
/// # 返回
/// 返回函数执行结果
pub fn set_ui_appearance_preset(preset: Option<&str>) -> Result<String, String> {
    let normalized = normalize_ui_appearance_preset(preset);
    save_persisted_app_setting(APP_SETTING_UI_APPEARANCE_PRESET_KEY, Some(&normalized))?;
    Ok(normalized)
}
