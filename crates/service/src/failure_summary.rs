use codexmanager_core::rpc::types::FailureReasonSummaryItem;
use codexmanager_core::storage::now_ts;
use std::collections::{BTreeMap, BTreeSet};

use crate::storage_helpers::open_storage;

const DEFAULT_FAILURE_SUMMARY_WINDOW_SECS: i64 = 24 * 60 * 60;
const DEFAULT_FAILURE_SUMMARY_EVENT_LIMIT: i64 = 500;

#[derive(Debug, Clone)]
struct FailureReasonAggregate {
    code: String,
    label: String,
    count: i64,
    last_seen_at: Option<i64>,
    account_ids: BTreeSet<String>,
}

pub(crate) fn read_failure_reason_summary() -> Result<Vec<FailureReasonSummaryItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let since_ts = now_ts().saturating_sub(DEFAULT_FAILURE_SUMMARY_WINDOW_SECS);
    let events = storage
        .list_recent_events_by_type("usage_refresh_failed", since_ts, DEFAULT_FAILURE_SUMMARY_EVENT_LIMIT)
        .map_err(|err| format!("list failure events failed: {err}"))?;

    let mut aggregates = BTreeMap::<String, FailureReasonAggregate>::new();
    for event in events {
        let (code, label) = classify_failure_reason(&event.message);
        let entry = aggregates
            .entry(code.to_string())
            .or_insert_with(|| FailureReasonAggregate {
                code: code.to_string(),
                label: label.to_string(),
                count: 0,
                last_seen_at: None,
                account_ids: BTreeSet::new(),
            });
        entry.count += 1;
        entry.last_seen_at = Some(
            entry
                .last_seen_at
                .map(|current| current.max(event.created_at))
                .unwrap_or(event.created_at),
        );
        if let Some(account_id) = event
            .account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            entry.account_ids.insert(account_id.to_string());
        }
    }

    let mut items = aggregates
        .into_values()
        .map(|item| FailureReasonSummaryItem {
            code: item.code,
            label: item.label,
            count: item.count,
            affected_accounts: item.account_ids.len() as i64,
            last_seen_at: item.last_seen_at,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| right.last_seen_at.cmp(&left.last_seen_at))
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(items)
}

fn classify_failure_reason(message: &str) -> (&'static str, &'static str) {
    let normalized = message.trim().to_ascii_lowercase();
    if looks_like_deactivated_account_error(message) {
        return ("account_deactivated", "账号已停用");
    }
    if let Some(reason) = crate::usage_http::refresh_token_auth_error_reason_from_message(message) {
        return match reason.as_code() {
            "refresh_token_expired" => ("refresh_token_expired", "Refresh 已过期"),
            "refresh_token_reused" => ("refresh_token_reused", "Refresh 已复用"),
            "refresh_token_invalidated" => ("refresh_token_invalidated", "Refresh 已失效"),
            _ => ("refresh_token_invalid", "Refresh 刷新失败"),
        };
    }
    if normalized.contains("status 429") {
        return ("usage_rate_limited", "接口限流");
    }
    if normalized.contains("status 403") {
        return ("usage_forbidden", "访问受限");
    }
    if normalized.contains("status 401") {
        return ("usage_unauthorized", "授权失效");
    }
    if normalized.contains("status 5") {
        return ("usage_upstream_server_error", "上游服务异常");
    }
    if normalized.contains("timeout") {
        return ("network_timeout", "网络超时");
    }
    if normalized.contains("dns") {
        return ("network_dns", "DNS 异常");
    }
    if normalized.contains("connection") || normalized.contains("connect") {
        return ("network_connection", "连接异常");
    }
    ("other_failure", "其他异常")
}

fn looks_like_deactivated_account_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.contains("your openai account has been deactivated")
        || (normalized.contains("account has been deactivated")
            && normalized.contains("help.openai.com"))
}

#[cfg(test)]
mod tests {
    use super::classify_failure_reason;

    #[test]
    fn classify_failure_reason_detects_deactivated_account() {
        let (code, label) = classify_failure_reason(
            "HTTP 401: Your OpenAI account has been deactivated, please check your email for more information. If you feel this is an error, contact us through our help center at help.openai.com",
        );
        assert_eq!(code, "account_deactivated");
        assert_eq!(label, "账号已停用");
    }

    #[test]
    fn classify_failure_reason_detects_refresh_token_expired() {
        let (code, label) = classify_failure_reason(
            "refresh token failed with status 401: Your access token could not be refreshed because your refresh token has expired. Please log out and sign in again.",
        );
        assert_eq!(code, "refresh_token_expired");
        assert_eq!(label, "Refresh 已过期");
    }
}
