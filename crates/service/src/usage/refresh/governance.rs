use codexmanager_core::storage::{now_ts, Event, Storage};
use std::collections::BTreeMap;

use super::{
    open_storage, AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED,
    AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD,
    AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD,
    AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS,
};

const AUTO_GOVERNANCE_SCAN_EVENT_LIMIT: i64 = 2_000;
const USAGE_REFRESH_FAILED_EVENT_TYPE: &str = "usage_refresh_failed";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GovernanceFailureStats {
    deactivated_failures: usize,
    refresh_token_failures: usize,
    auth_failures: usize,
    proxy_failures: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GovernanceFailureKind {
    Deactivated,
    RefreshToken,
    Auth,
    Proxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GovernanceAction {
    None,
    MarkDeactivated,
    DisableForRefreshToken,
    DisableForSuspected,
    DisableForProxy,
}

pub(crate) fn maybe_trigger_auto_account_governance() -> Result<(), String> {
    if !AUTO_DISABLE_RISKY_ACCOUNTS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    let failure_threshold = AUTO_DISABLE_RISKY_ACCOUNTS_FAILURE_THRESHOLD
        .load(std::sync::atomic::Ordering::Relaxed)
        .max(1);
    let health_threshold = i32::try_from(
        AUTO_DISABLE_RISKY_ACCOUNTS_HEALTH_SCORE_THRESHOLD
            .load(std::sync::atomic::Ordering::Relaxed)
            .min(200),
    )
    .unwrap_or(60);
    let lookback_mins = AUTO_DISABLE_RISKY_ACCOUNTS_LOOKBACK_MINS
        .load(std::sync::atomic::Ordering::Relaxed)
        .max(1);

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let since_ts = now_ts().saturating_sub(
        i64::try_from(lookback_mins)
            .unwrap_or(i64::MAX / 60)
            .saturating_mul(60),
    );
    let recent_events = storage
        .list_recent_events_by_type(
            USAGE_REFRESH_FAILED_EVENT_TYPE,
            since_ts,
            AUTO_GOVERNANCE_SCAN_EVENT_LIMIT,
        )
        .map_err(|err| format!("list governance failure events failed: {err}"))?;
    if recent_events.is_empty() {
        return Ok(());
    }

    let failure_map = aggregate_recent_failures(recent_events);
    if failure_map.is_empty() {
        return Ok(());
    }

    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts for governance failed: {err}"))?
        .into_iter()
        .map(|account| (account.id.clone(), account))
        .collect::<BTreeMap<_, _>>();

    let mut changed = 0usize;
    for (account_id, stats) in failure_map {
        let Some(account) = accounts.get(&account_id) else {
            continue;
        };
        let health_score = crate::gateway::route_health_score(account_id.as_str());
        match decide_governance_action(
            account.status.as_str(),
            health_score,
            stats,
            failure_threshold,
            health_threshold,
        ) {
            GovernanceAction::None => {}
            GovernanceAction::MarkDeactivated => {
                apply_governance_status(
                    &storage,
                    account_id.as_str(),
                    "deactivated",
                    "auto_governance_deactivated",
                    stats,
                    health_score,
                    lookback_mins,
                );
                changed = changed.saturating_add(1);
            }
            GovernanceAction::DisableForRefreshToken => {
                apply_governance_status(
                    &storage,
                    account_id.as_str(),
                    "disabled",
                    "auto_governance_refresh_token",
                    stats,
                    health_score,
                    lookback_mins,
                );
                changed = changed.saturating_add(1);
            }
            GovernanceAction::DisableForSuspected => {
                apply_governance_status(
                    &storage,
                    account_id.as_str(),
                    "disabled",
                    "auto_governance_suspected",
                    stats,
                    health_score,
                    lookback_mins,
                );
                changed = changed.saturating_add(1);
            }
            GovernanceAction::DisableForProxy => {
                apply_governance_status(
                    &storage,
                    account_id.as_str(),
                    "disabled",
                    "auto_governance_proxy_failures",
                    stats,
                    health_score,
                    lookback_mins,
                );
                changed = changed.saturating_add(1);
            }
        }
    }

    if changed > 0 {
        log::warn!(
            "auto account governance updated accounts: changed={} lookback_mins={} failure_threshold={} health_threshold={}",
            changed,
            lookback_mins,
            failure_threshold,
            health_threshold
        );
    }

    Ok(())
}

fn aggregate_recent_failures(events: Vec<Event>) -> BTreeMap<String, GovernanceFailureStats> {
    let mut stats_map = BTreeMap::<String, GovernanceFailureStats>::new();
    for event in events {
        let Some(account_id) = event
            .account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(kind) = classify_governance_failure(event.message.as_str()) else {
            continue;
        };
        let stats = stats_map.entry(account_id.to_string()).or_default();
        match kind {
            GovernanceFailureKind::Deactivated => {
                stats.deactivated_failures = stats.deactivated_failures.saturating_add(1);
            }
            GovernanceFailureKind::RefreshToken => {
                stats.refresh_token_failures = stats.refresh_token_failures.saturating_add(1);
            }
            GovernanceFailureKind::Auth => {
                stats.auth_failures = stats.auth_failures.saturating_add(1);
            }
            GovernanceFailureKind::Proxy => {
                stats.proxy_failures = stats.proxy_failures.saturating_add(1);
            }
        }
    }
    stats_map
}

fn classify_governance_failure(message: &str) -> Option<GovernanceFailureKind> {
    if super::errors::usage_error_indicates_deactivated_account(message) {
        return Some(GovernanceFailureKind::Deactivated);
    }
    if crate::usage_http::refresh_token_auth_error_reason_from_message(message).is_some() {
        return Some(GovernanceFailureKind::RefreshToken);
    }

    let normalized = message.trim().to_ascii_lowercase();
    if normalized.contains("proxy_auth_required")
        || normalized.contains("proxy authentication required")
        || normalized.contains("backend proxy error:")
        || (normalized.contains("proxy") && normalized.contains("connect"))
    {
        return Some(GovernanceFailureKind::Proxy);
    }
    if normalized.contains("status 401") || normalized.contains("status 403") {
        return Some(GovernanceFailureKind::Auth);
    }
    None
}

fn decide_governance_action(
    status: &str,
    health_score: i32,
    stats: GovernanceFailureStats,
    failure_threshold: usize,
    health_threshold: i32,
) -> GovernanceAction {
    if !status_allows_governance(status) {
        return GovernanceAction::None;
    }
    if stats.deactivated_failures > 0 {
        return GovernanceAction::MarkDeactivated;
    }
    if stats.refresh_token_failures >= failure_threshold {
        return GovernanceAction::DisableForRefreshToken;
    }
    if stats.proxy_failures >= failure_threshold {
        return GovernanceAction::DisableForProxy;
    }
    if stats.auth_failures >= failure_threshold && health_score <= health_threshold {
        return GovernanceAction::DisableForSuspected;
    }
    GovernanceAction::None
}

fn status_allows_governance(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "disabled" | "inactive" | "deactivated"
    )
}

fn apply_governance_status(
    storage: &Storage,
    account_id: &str,
    status: &str,
    reason: &str,
    stats: GovernanceFailureStats,
    health_score: i32,
    lookback_mins: u64,
) {
    crate::account_status::set_account_status(storage, account_id, status, reason);
    log::warn!(
        "auto account governance applied: account_id={} status={} reason={} lookback_mins={} health_score={} deactivated_failures={} refresh_token_failures={} auth_failures={} proxy_failures={}",
        account_id,
        status,
        reason,
        lookback_mins,
        health_score,
        stats.deactivated_failures,
        stats.refresh_token_failures,
        stats.auth_failures,
        stats.proxy_failures
    );
}

#[cfg(test)]
mod tests {
    use super::{
        classify_governance_failure, decide_governance_action, GovernanceAction,
        GovernanceFailureKind, GovernanceFailureStats,
    };

    #[test]
    fn classify_governance_failure_detects_expected_high_risk_errors() {
        assert_eq!(
            classify_governance_failure(
                "HTTP 401: Your OpenAI account has been deactivated, please check your email for more information. If you feel this is an error, contact us through our help center at help.openai.com"
            ),
            Some(GovernanceFailureKind::Deactivated)
        );
        assert_eq!(
            classify_governance_failure(
                "refresh token failed with status 401: Your access token could not be refreshed because your refresh token has expired. Please log out and sign in again."
            ),
            Some(GovernanceFailureKind::RefreshToken)
        );
        assert_eq!(
            classify_governance_failure("usage endpoint status 403: access forbidden"),
            Some(GovernanceFailureKind::Auth)
        );
        assert_eq!(
            classify_governance_failure(
                "backend proxy error: proxy authentication required"
            ),
            Some(GovernanceFailureKind::Proxy)
        );
        assert_eq!(
            classify_governance_failure("usage endpoint status 429: rate limited"),
            None
        );
    }

    #[test]
    fn governance_decision_prefers_deactivated_and_skips_manual_disabled_accounts() {
        let stats = GovernanceFailureStats {
            deactivated_failures: 1,
            refresh_token_failures: 5,
            auth_failures: 5,
            proxy_failures: 5,
        };
        assert_eq!(
            decide_governance_action("active", 120, stats, 3, 60),
            GovernanceAction::MarkDeactivated
        );
        assert_eq!(
            decide_governance_action("disabled", 20, stats, 3, 60),
            GovernanceAction::None
        );
    }

    #[test]
    fn governance_decision_only_disables_auth_failures_when_health_is_low_enough() {
        let stats = GovernanceFailureStats {
            deactivated_failures: 0,
            refresh_token_failures: 0,
            auth_failures: 3,
            proxy_failures: 0,
        };
        assert_eq!(
            decide_governance_action("active", 55, stats, 3, 60),
            GovernanceAction::DisableForSuspected
        );
        assert_eq!(
            decide_governance_action("active", 90, stats, 3, 60),
            GovernanceAction::None
        );
    }

    #[test]
    fn governance_decision_disables_proxy_failures_before_auth_suspected() {
        let stats = GovernanceFailureStats {
            deactivated_failures: 0,
            refresh_token_failures: 0,
            auth_failures: 1,
            proxy_failures: 3,
        };
        assert_eq!(
            decide_governance_action("active", 120, stats, 3, 60),
            GovernanceAction::DisableForProxy
        );
    }
}
