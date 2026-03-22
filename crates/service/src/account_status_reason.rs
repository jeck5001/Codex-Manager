#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedAccountStatusEvent {
    pub status: Option<String>,
    pub reason_code: Option<String>,
    pub reason_label: Option<String>,
    pub governance_reason_label: Option<String>,
    pub isolation_reason_code: Option<String>,
    pub isolation_reason_label: Option<String>,
}

pub(crate) fn parse_account_status_event(message: &str) -> ParsedAccountStatusEvent {
    let status = extract_field(message, "status");
    let reason_code = extract_field(message, "reason");
    let reason_label = reason_code
        .as_deref()
        .map(|reason| map_account_status_reason_label(reason).to_string());
    let governance_reason_label = reason_code
        .as_deref()
        .and_then(map_governance_reason_label)
        .map(ToString::to_string);
    let (isolation_reason_code, isolation_reason_label) = reason_code
        .as_deref()
        .and_then(map_isolation_reason)
        .map(|(code, label)| (Some(code.to_string()), Some(label.to_string())))
        .unwrap_or((None, None));

    ParsedAccountStatusEvent {
        status,
        reason_code,
        reason_label,
        governance_reason_label,
        isolation_reason_code,
        isolation_reason_label,
    }
}

pub(crate) fn map_governance_reason_label(reason: &str) -> Option<&'static str> {
    match reason.trim().to_ascii_lowercase().as_str() {
        "auto_governance_deactivated" => Some("检测到账号已停用"),
        "auto_governance_refresh_token" => Some("Refresh 连续失效"),
        "auto_governance_auth_failures" => Some("401/403 连续失败"),
        "auto_governance_suspected" => Some("疑似风控/授权异常"),
        "auto_governance_proxy_failures" => Some("代理异常"),
        _ => None,
    }
}

pub(crate) fn map_account_status_reason_label(reason: &str) -> &'static str {
    match reason.trim().to_ascii_lowercase().as_str() {
        "usage_ok" => "用量恢复正常",
        "usage_http_deactivated" => "检测到账号已停用",
        "usage_http_401" => "授权失效",
        "manual_disable" => "手动禁用",
        "manual_disable_many" => "批量禁用",
        "manual_enable" => "手动启用",
        "manual_enable_many" => "批量启用",
        "auto_governance_deactivated" => "检测到账号已停用",
        "auto_governance_refresh_token" => "Refresh 连续失效",
        "auto_governance_auth_failures" => "401/403 连续失败",
        "auto_governance_suspected" => "疑似风控/授权异常",
        "auto_governance_proxy_failures" => "代理异常",
        "refresh_token_invalid:expired" => "Refresh 已过期",
        "refresh_token_invalid:reused" => "Refresh 已复用",
        "refresh_token_invalid:invalidated" => "Refresh 已失效",
        "refresh_token_invalid:invalid" => "Refresh 刷新失败",
        _ => "状态已变更",
    }
}

pub(crate) fn map_isolation_reason(reason: &str) -> Option<(&'static str, &'static str)> {
    let normalized = reason.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "usage_http_deactivated" | "auto_governance_deactivated" => {
            Some(("deactivated", "检测到账号已停用"))
        }
        "usage_http_401" | "auto_governance_auth_failures" => {
            Some(("auth_invalid", "授权失效 / 401-403"))
        }
        "auto_governance_suspected" => Some(("suspected", "疑似风控/授权异常")),
        "auto_governance_proxy_failures" => Some(("proxy_error", "代理异常")),
        "auto_governance_refresh_token" => Some(("refresh_token", "Refresh 连续失效")),
        _ if normalized.starts_with("refresh_token_invalid:") => {
            Some(("refresh_token", map_account_status_reason_label(reason)))
        }
        _ => None,
    }
}

fn extract_field(message: &str, key: &str) -> Option<String> {
    message
        .split_whitespace()
        .find_map(|segment| segment.strip_prefix(&format!("{key}=")))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::{map_account_status_reason_label, parse_account_status_event};

    #[test]
    fn parse_account_status_event_extracts_reason_and_governance_label() {
        let parsed =
            parse_account_status_event("status=disabled reason=auto_governance_refresh_token");
        assert_eq!(parsed.status.as_deref(), Some("disabled"));
        assert_eq!(
            parsed.reason_code.as_deref(),
            Some("auto_governance_refresh_token")
        );
        assert_eq!(parsed.reason_label.as_deref(), Some("Refresh 连续失效"));
        assert_eq!(
            parsed.governance_reason_label.as_deref(),
            Some("Refresh 连续失效")
        );
        assert_eq!(
            parsed.isolation_reason_code.as_deref(),
            Some("refresh_token")
        );
        assert_eq!(
            parsed.isolation_reason_label.as_deref(),
            Some("Refresh 连续失效")
        );
    }

    #[test]
    fn parse_account_status_event_supports_new_isolation_labels() {
        let parsed = parse_account_status_event("status=disabled reason=auto_governance_suspected");
        assert_eq!(parsed.reason_label.as_deref(), Some("疑似风控/授权异常"));
        assert_eq!(
            parsed.governance_reason_label.as_deref(),
            Some("疑似风控/授权异常")
        );
        assert_eq!(
            map_account_status_reason_label("auto_governance_proxy_failures"),
            "代理异常"
        );
        assert_eq!(parsed.isolation_reason_code.as_deref(), Some("suspected"));
        assert_eq!(
            parsed.isolation_reason_label.as_deref(),
            Some("疑似风控/授权异常")
        );
    }

    #[test]
    fn map_account_status_reason_label_covers_refresh_token_variants() {
        assert_eq!(
            map_account_status_reason_label("refresh_token_invalid:expired"),
            "Refresh 已过期"
        );
        assert_eq!(
            map_account_status_reason_label("refresh_token_invalid:invalidated"),
            "Refresh 已失效"
        );
    }

    #[test]
    fn map_isolation_reason_covers_direct_status_failures() {
        let parsed = parse_account_status_event("status=unavailable reason=usage_http_401");
        assert_eq!(
            parsed.isolation_reason_code.as_deref(),
            Some("auth_invalid")
        );
        assert_eq!(
            parsed.isolation_reason_label.as_deref(),
            Some("授权失效 / 401-403")
        );
    }
}
