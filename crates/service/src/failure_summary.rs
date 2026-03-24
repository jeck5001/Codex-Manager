use chrono::{DateTime, NaiveDateTime, Utc};
use codexmanager_core::rpc::types::FailureReasonSummaryItem;
use codexmanager_core::storage::now_ts;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::storage_helpers::open_storage;

const DEFAULT_FAILURE_SUMMARY_WINDOW_SECS: i64 = 24 * 60 * 60;
const DEFAULT_FAILURE_SUMMARY_EVENT_LIMIT: i64 = 500;
const DEFAULT_REGISTER_FAILURE_PAGE_SIZE: i64 = 100;
const DEFAULT_REGISTER_FAILURE_PAGE_LIMIT: i64 = 3;

#[derive(Debug, Clone)]
struct FailureReasonAggregate {
    code: String,
    label: String,
    count: i64,
    last_seen_at: Option<i64>,
    account_ids: BTreeSet<String>,
}

fn record_failure_reason(
    aggregates: &mut BTreeMap<String, FailureReasonAggregate>,
    code: &str,
    label: &str,
    last_seen_at: i64,
    account_or_entity_id: Option<&str>,
) {
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
            .map(|current| current.max(last_seen_at))
            .unwrap_or(last_seen_at),
    );
    if let Some(identifier) = account_or_entity_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        entry.account_ids.insert(identifier.to_string());
    }
}

fn parse_register_task_ts(raw: Option<&str>) -> Option<i64> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }

    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp())
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).timestamp())
        })
}

fn merge_register_failure_summary(
    aggregates: &mut BTreeMap<String, FailureReasonAggregate>,
    since_ts: i64,
) -> Result<(), String> {
    for page in 1..=DEFAULT_REGISTER_FAILURE_PAGE_LIMIT {
        let payload = crate::account_register::list_register_tasks(
            page,
            DEFAULT_REGISTER_FAILURE_PAGE_SIZE,
            Some("failed"),
        )?;
        let Some(tasks) = payload.get("tasks").and_then(Value::as_array) else {
            break;
        };
        if tasks.is_empty() {
            break;
        }

        let mut reached_old_records = false;
        for task in tasks {
            let completed_at = task
                .get("completed_at")
                .or_else(|| task.get("completedAt"))
                .and_then(Value::as_str);
            let created_at = task
                .get("created_at")
                .or_else(|| task.get("createdAt"))
                .and_then(Value::as_str);
            let task_ts = parse_register_task_ts(completed_at)
                .or_else(|| parse_register_task_ts(created_at))
                .unwrap_or_default();
            if task_ts < since_ts {
                reached_old_records = true;
                continue;
            }

            let error_message = task
                .get("error_message")
                .or_else(|| task.get("errorMessage"))
                .and_then(Value::as_str);
            let logs = task.get("logs").and_then(Value::as_str).unwrap_or_default();
            let failure_reason = task
                .get("failureCode")
                .and_then(Value::as_str)
                .zip(task.get("failureLabel").and_then(Value::as_str))
                .or_else(|| {
                    crate::account_register::classify_register_failure_reason(error_message, logs)
                });
            let Some((code, label)) = failure_reason else {
                continue;
            };

            let affected_id = task
                .get("email")
                .and_then(Value::as_str)
                .or_else(|| task.get("task_uuid").and_then(Value::as_str))
                .or_else(|| task.get("taskUuid").and_then(Value::as_str));
            record_failure_reason(aggregates, code, label, task_ts, affected_id);
        }

        if reached_old_records {
            break;
        }
    }

    Ok(())
}

pub(crate) fn read_failure_reason_summary() -> Result<Vec<FailureReasonSummaryItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let since_ts = now_ts().saturating_sub(DEFAULT_FAILURE_SUMMARY_WINDOW_SECS);
    let events = storage
        .list_recent_events_by_type(
            "usage_refresh_failed",
            since_ts,
            DEFAULT_FAILURE_SUMMARY_EVENT_LIMIT,
        )
        .map_err(|err| format!("list failure events failed: {err}"))?;

    let mut aggregates = BTreeMap::<String, FailureReasonAggregate>::new();
    for event in events {
        let (code, label) = classify_failure_reason(&event.message);
        let account_id = event
            .account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        record_failure_reason(&mut aggregates, code, label, event.created_at, account_id);
    }
    if let Err(err) = merge_register_failure_summary(&mut aggregates, since_ts) {
        log::warn!("read register failure summary skipped: {err}");
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
    if let Some((code, label)) = classify_deactivated_failure(message) {
        return (code, label);
    }
    if let Some(reason) = crate::usage_http::refresh_token_auth_error_reason_from_message(message) {
        return match reason.as_code() {
            "refresh_token_expired" => ("refresh_token_expired", "Refresh 已过期"),
            "refresh_token_reused" => ("refresh_token_reused", "Refresh 已复用"),
            "refresh_token_invalidated" => ("refresh_token_invalidated", "Refresh 已失效"),
            _ => ("refresh_token_invalid", "Refresh 刷新失败"),
        };
    }
    if normalized.contains("status 429") || normalized.contains("status=429") {
        return ("usage_rate_limited", "接口限流");
    }
    if normalized.contains("status 403") || normalized.contains("status=403") {
        return ("usage_forbidden", "访问受限");
    }
    if normalized.contains("status 401") || normalized.contains("status=401") {
        return ("usage_unauthorized", "授权失效");
    }
    if normalized.contains("status 5") || normalized.contains("status=5") {
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

fn classify_deactivated_failure(message: &str) -> Option<(&'static str, &'static str)> {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.contains("workspace has been deactivated")
        || normalized.contains("workspace is deactivated")
        || normalized.contains("workspace_deactivated")
    {
        return Some(("workspace_deactivated", "工作区已停用"));
    }
    if normalized.contains("your openai account has been deactivated")
        || (normalized.contains("account has been deactivated")
            && normalized.contains("help.openai.com"))
    {
        return Some(("account_deactivated", "账号已停用"));
    }
    None
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
    fn classify_failure_reason_detects_deactivated_workspace() {
        let (code, label) =
            classify_failure_reason("HTTP 403: This workspace has been deactivated.");
        assert_eq!(code, "workspace_deactivated");
        assert_eq!(label, "工作区已停用");
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
