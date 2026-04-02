mod items;

use serde_json::Value;

pub(crate) use items::ENV_OVERRIDE_CATALOG;
use items::{APP_SETTINGS_ENV_RESERVED_KEYS, APP_SETTINGS_ENV_UNSUPPORTED_KEYS};

#[derive(Clone, Copy)]
pub(super) struct EnvOverrideCatalogItem {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    pub(super) scope: &'static str,
    pub(super) apply_mode: &'static str,
    pub(super) default_value: &'static str,
}

impl EnvOverrideCatalogItem {
    pub(super) const fn new(
        key: &'static str,
        label: &'static str,
        scope: &'static str,
        apply_mode: &'static str,
        default_value: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            scope,
            apply_mode,
            default_value,
        }
    }
}

/// 函数 `env_override_reserved_keys`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn env_override_reserved_keys() -> &'static [&'static str] {
    APP_SETTINGS_ENV_RESERVED_KEYS
}

/// 函数 `env_override_unsupported_keys`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn env_override_unsupported_keys() -> &'static [&'static str] {
    APP_SETTINGS_ENV_UNSUPPORTED_KEYS
}

/// 函数 `editable_env_override_catalog`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn editable_env_override_catalog(
) -> impl Iterator<Item = &'static EnvOverrideCatalogItem> {
    ENV_OVERRIDE_CATALOG
        .iter()
        .filter(|item| !is_env_override_reserved_key(item.key))
}

/// 函数 `env_override_catalog_item`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn env_override_catalog_item(key: &str) -> Option<&'static EnvOverrideCatalogItem> {
    editable_env_override_catalog().find(|item| item.key.eq_ignore_ascii_case(key))
}

/// 函数 `is_env_override_catalog_key`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn is_env_override_catalog_key(key: &str) -> bool {
    env_override_catalog_item(key).is_some()
}

/// 函数 `is_env_override_unsupported_key`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn is_env_override_unsupported_key(key: &str) -> bool {
    APP_SETTINGS_ENV_UNSUPPORTED_KEYS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(key))
}

/// 函数 `is_env_override_reserved_key`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn is_env_override_reserved_key(key: &str) -> bool {
    APP_SETTINGS_ENV_RESERVED_KEYS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(key))
}

/// 函数 `env_override_catalog_value`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn env_override_catalog_value() -> Vec<Value> {
    editable_env_override_catalog()
        .map(|item| {
            serde_json::json!({
                "key": item.key,
                "label": item.label,
                "scope": item.scope,
                "applyMode": item.apply_mode,
                "defaultValue": super::snapshot::env_override_default_value(item.key),
            })
        })
        .collect()
}
