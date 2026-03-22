use codexmanager_core::rpc::types::GovernanceSummaryItem;
use codexmanager_core::storage::now_ts;
use std::collections::{BTreeMap, BTreeSet};

use crate::storage_helpers::open_storage;

const DEFAULT_GOVERNANCE_SUMMARY_WINDOW_SECS: i64 = 24 * 60 * 60;
const DEFAULT_GOVERNANCE_SUMMARY_EVENT_LIMIT: i64 = 500;

#[derive(Debug, Clone)]
struct GovernanceAggregate {
    code: String,
    label: String,
    target_status: String,
    count: i64,
    last_seen_at: Option<i64>,
    account_ids: BTreeSet<String>,
}

pub(crate) fn read_governance_summary() -> Result<Vec<GovernanceSummaryItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let since_ts = now_ts().saturating_sub(DEFAULT_GOVERNANCE_SUMMARY_WINDOW_SECS);
    let events = storage
        .list_recent_events_by_type(
            "account_status_update",
            since_ts,
            DEFAULT_GOVERNANCE_SUMMARY_EVENT_LIMIT,
        )
        .map_err(|err| format!("list governance events failed: {err}"))?;

    let mut aggregates = BTreeMap::<String, GovernanceAggregate>::new();
    for event in events {
        let Some((code, label, target_status)) = classify_governance_reason(event.message.as_str())
        else {
            continue;
        };
        let entry = aggregates
            .entry(code.to_string())
            .or_insert_with(|| GovernanceAggregate {
                code: code.to_string(),
                label: label.to_string(),
                target_status: target_status.to_string(),
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
        .map(|item| GovernanceSummaryItem {
            code: item.code,
            label: item.label,
            target_status: item.target_status,
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

fn classify_governance_reason(message: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let parsed = crate::account_status_reason::parse_account_status_event(message);
    let reason = parsed.reason_code.as_deref()?;
    let label = crate::account_status_reason::map_governance_reason_label(reason)?;
    let code = match reason {
        "auto_governance_deactivated" => "auto_deactivated",
        "auto_governance_refresh_token" => "refresh_token_disabled",
        "auto_governance_auth_failures" => "auth_failures_disabled",
        "auto_governance_suspected" => "suspected_disabled",
        "auto_governance_proxy_failures" => "proxy_failures_disabled",
        _ => return None,
    };
    let target_status = parsed.status.as_deref().unwrap_or("disabled");
    Some((
        code,
        label,
        if target_status == "deactivated" {
            "deactivated"
        } else {
            "disabled"
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::classify_governance_reason;

    #[test]
    fn classify_governance_reason_recognizes_expected_markers() {
        assert_eq!(
            classify_governance_reason("status=deactivated reason=auto_governance_deactivated"),
            Some(("auto_deactivated", "检测到账号已停用", "deactivated"))
        );
        assert_eq!(
            classify_governance_reason("status=disabled reason=auto_governance_refresh_token"),
            Some(("refresh_token_disabled", "Refresh 连续失效", "disabled"))
        );
        assert_eq!(
            classify_governance_reason("status=disabled reason=auto_governance_auth_failures"),
            Some(("auth_failures_disabled", "401/403 连续失败", "disabled"))
        );
        assert_eq!(
            classify_governance_reason("status=disabled reason=auto_governance_suspected"),
            Some(("suspected_disabled", "疑似风控/授权异常", "disabled"))
        );
        assert_eq!(
            classify_governance_reason("status=disabled reason=auto_governance_proxy_failures"),
            Some(("proxy_failures_disabled", "代理异常", "disabled"))
        );
        assert_eq!(
            classify_governance_reason("status=disabled reason=manual"),
            None
        );
    }
}
