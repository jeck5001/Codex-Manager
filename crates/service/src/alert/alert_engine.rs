use codexmanager_core::storage::{
    now_ts, Account, AlertChannel, AlertRule, RequestLogFilterInput, UsageSnapshotRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use crate::alert_sender::send_alert;
use crate::storage_helpers::open_storage;
use crate::usage_scheduler::{run_blocking_poll_loop, BlockingPollLoopConfig};

const ALERT_RUNTIME_STATES_KEY: &str = "alert.runtime_states";
const ALERT_POLL_INTERVAL_SECS: u64 = 60;
const ALERT_POLL_JITTER_SECS: u64 = 5;
const ALERT_POLL_FAILURE_BACKOFF_MAX_SECS: u64 = 300;

static ALERT_POLLING_STARTED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AlertRuntimeState {
    active: bool,
    last_sent_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct RuleEvaluation {
    triggered: bool,
    title: String,
    message: String,
    payload: Value,
}

pub(crate) fn ensure_alert_polling() {
    ALERT_POLLING_STARTED.get_or_init(|| {
        let _ = thread::Builder::new()
            .name("alert-polling".to_string())
            .spawn(move || {
                run_blocking_poll_loop(
                    BlockingPollLoopConfig {
                        loop_name: "alert polling",
                        interval: Duration::from_secs(ALERT_POLL_INTERVAL_SECS),
                        jitter: Duration::from_secs(ALERT_POLL_JITTER_SECS),
                        failure_backoff_cap: Duration::from_secs(
                            ALERT_POLL_FAILURE_BACKOFF_MAX_SECS,
                        ),
                    },
                    run_alert_checks_once,
                    |_| true,
                );
            });
    });
}

pub(crate) fn run_alert_checks_once() -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let rules = storage
        .list_alert_rules()
        .map_err(|err| format!("list alert rules failed: {err}"))?;
    if rules.is_empty() {
        return Ok(());
    }

    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts failed: {err}"))?;
    let snapshots = storage
        .latest_usage_snapshots_by_account()
        .map_err(|err| format!("list usage snapshots failed: {err}"))?;
    let channels = storage
        .list_alert_channels()
        .map_err(|err| format!("list alert channels failed: {err}"))?;
    let channel_map = channels
        .into_iter()
        .filter(|item| item.enabled)
        .map(|item| (item.id.clone(), item))
        .collect::<HashMap<_, _>>();
    let mut runtime_states = load_runtime_states(&storage);
    let mut next_states = BTreeMap::new();

    for rule in rules.into_iter().filter(|item| item.enabled) {
        let config = parse_rule_config(&rule);
        let cooldown_secs = config_i64(&config, "cooldownSecs", 1800).max(0);
        let evaluation = evaluate_rule(&storage, &rule, &config, &accounts, &snapshots)?;
        let previous = runtime_states.remove(&rule.id).unwrap_or_default();
        let channel_ids = config_string_array(&config, "channelIds");
        let eligible_channels = channel_ids
            .iter()
            .filter_map(|channel_id| channel_map.get(channel_id).cloned())
            .collect::<Vec<_>>();
        let now = now_ts();

        if evaluation.triggered {
            let should_send = !previous.active
                || previous
                    .last_sent_at
                    .map(|last| now.saturating_sub(last) >= cooldown_secs)
                    .unwrap_or(true);
            if should_send {
                dispatch_alert(
                    &storage,
                    &rule,
                    &eligible_channels,
                    "triggered",
                    &evaluation.title,
                    &evaluation.message,
                    &evaluation.payload,
                );
            }
            next_states.insert(
                rule.id.clone(),
                AlertRuntimeState {
                    active: true,
                    last_sent_at: if should_send {
                        Some(now)
                    } else {
                        previous.last_sent_at
                    },
                },
            );
            continue;
        }

        if previous.active {
            dispatch_alert(
                &storage,
                &rule,
                &eligible_channels,
                "recovered",
                &format!("告警恢复: {}", rule.name),
                &evaluation.message,
                &evaluation.payload,
            );
            next_states.insert(
                rule.id.clone(),
                AlertRuntimeState {
                    active: false,
                    last_sent_at: Some(now),
                },
            );
            continue;
        }

        next_states.insert(rule.id.clone(), previous);
    }

    save_runtime_states(&storage, &next_states)?;
    Ok(())
}

fn dispatch_alert(
    storage: &codexmanager_core::storage::Storage,
    rule: &AlertRule,
    channels: &[AlertChannel],
    history_status: &str,
    title: &str,
    message: &str,
    payload: &Value,
) {
    if channels.is_empty() {
        let _ = storage.insert_alert_history(
            Some(rule.id.as_str()),
            None,
            "delivery_failed",
            &format!("{}: no enabled channels configured", rule.name),
        );
        return;
    }

    for channel in channels {
        match send_alert(channel, title, message, payload) {
            Ok(_) => {
                let _ = storage.insert_alert_history(
                    Some(rule.id.as_str()),
                    Some(channel.id.as_str()),
                    history_status,
                    &format!("{}: {}", rule.name, message),
                );
            }
            Err(err) => {
                let _ = storage.insert_alert_history(
                    Some(rule.id.as_str()),
                    Some(channel.id.as_str()),
                    "delivery_failed",
                    &format!("{} via {} failed: {}", rule.name, channel.name, err),
                );
            }
        }
    }
}

fn evaluate_rule(
    storage: &codexmanager_core::storage::Storage,
    rule: &AlertRule,
    config: &Value,
    accounts: &[Account],
    snapshots: &[UsageSnapshotRecord],
) -> Result<RuleEvaluation, String> {
    match rule.rule_type.as_str() {
        "token_refresh_fail" => evaluate_token_refresh_fail(storage, rule, config),
        "usage_threshold" => Ok(evaluate_usage_threshold(rule, config, accounts, snapshots)),
        "error_rate" => evaluate_error_rate(storage, rule, config),
        "all_unavailable" => Ok(evaluate_all_unavailable(rule, accounts)),
        other => Err(format!("unsupported alert rule type: {other}")),
    }
}

fn evaluate_token_refresh_fail(
    storage: &codexmanager_core::storage::Storage,
    rule: &AlertRule,
    config: &Value,
) -> Result<RuleEvaluation, String> {
    let threshold = config_i64(config, "threshold", 3).max(1);
    let window_minutes = config_i64(config, "windowMinutes", 60).max(1);
    let since_ts = now_ts().saturating_sub(window_minutes.saturating_mul(60));
    let events = storage
        .list_recent_events_by_type("usage_refresh_failed", since_ts, 500)
        .map_err(|err| format!("list refresh failure events failed: {err}"))?;
    let mut counts = HashMap::<String, i64>::new();
    for event in &events {
        if let Some(account_id) = event.account_id.as_ref() {
            *counts.entry(account_id.clone()).or_insert(0) += 1;
        }
    }
    let mut offenders = counts
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .collect::<Vec<_>>();
    offenders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let triggered = !offenders.is_empty();
    let offender_labels = offenders
        .iter()
        .take(3)
        .map(|(account_id, count)| format!("{account_id}({count})"))
        .collect::<Vec<_>>();
    Ok(RuleEvaluation {
        triggered,
        title: format!("告警触发: {}", rule.name),
        message: if triggered {
            format!(
                "最近 {} 分钟内存在 {} 个账号 token 刷新失败次数达到阈值 {}：{}",
                window_minutes,
                offenders.len(),
                threshold,
                offender_labels.join(", ")
            )
        } else {
            format!("Token 刷新失败已恢复到阈值 {} 以下", threshold)
        },
        payload: json!({
            "ruleId": rule.id,
            "ruleType": rule.rule_type,
            "threshold": threshold,
            "windowMinutes": window_minutes,
            "offenders": offender_labels,
        }),
    })
}

fn evaluate_usage_threshold(
    rule: &AlertRule,
    config: &Value,
    accounts: &[Account],
    snapshots: &[UsageSnapshotRecord],
) -> RuleEvaluation {
    let threshold = config_f64(config, "thresholdPercent", 90.0).clamp(0.0, 100.0);
    let account_map = accounts
        .iter()
        .map(|item| (item.id.as_str(), item.label.as_str()))
        .collect::<HashMap<_, _>>();
    let mut offenders = snapshots
        .iter()
        .filter_map(|snapshot| {
            let primary_hit = snapshot.used_percent.unwrap_or(0.0) >= threshold;
            let secondary_hit = snapshot.secondary_used_percent.unwrap_or(0.0) >= threshold;
            if !primary_hit && !secondary_hit {
                return None;
            }
            let label = account_map
                .get(snapshot.account_id.as_str())
                .copied()
                .unwrap_or(snapshot.account_id.as_str());
            let highest_used = snapshot
                .secondary_used_percent
                .unwrap_or(0.0)
                .max(snapshot.used_percent.unwrap_or(0.0));
            Some(format!("{label}({highest_used:.0}%)"))
        })
        .collect::<Vec<_>>();
    offenders.sort();
    let triggered = !offenders.is_empty();
    RuleEvaluation {
        triggered,
        title: format!("告警触发: {}", rule.name),
        message: if triggered {
            format!(
                "共有 {} 个账号额度使用率达到阈值 {:.0}%：{}",
                offenders.len(),
                threshold,
                offenders
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("所有账号额度使用率已回落到 {:.0}% 以下", threshold)
        },
        payload: json!({
            "ruleId": rule.id,
            "ruleType": rule.rule_type,
            "thresholdPercent": threshold,
            "offenders": offenders,
        }),
    }
}

fn evaluate_error_rate(
    storage: &codexmanager_core::storage::Storage,
    rule: &AlertRule,
    config: &Value,
) -> Result<RuleEvaluation, String> {
    let threshold = config_f64(config, "thresholdPercent", 20.0).clamp(0.0, 100.0);
    let window_minutes = config_i64(config, "windowMinutes", 5).max(1);
    let min_requests = config_i64(config, "minRequests", 20).max(1);
    let since_ts = now_ts().saturating_sub(window_minutes.saturating_mul(60));
    let summary = storage
        .summarize_request_logs_filtered_with_filters(RequestLogFilterInput {
            query: None,
            status_filter: None,
            key_id: None,
            model: None,
            time_from: Some(since_ts),
            time_to: None,
        })
        .map_err(|err| format!("summarize request logs failed: {err}"))?;
    let total = summary.count.max(0);
    let error_rate = if total > 0 {
        (summary.error_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let triggered = total >= min_requests && error_rate >= threshold;
    Ok(RuleEvaluation {
        triggered,
        title: format!("告警触发: {}", rule.name),
        message: if triggered {
            format!(
                "最近 {} 分钟请求错误率为 {:.1}%（错误 {}/总计 {}）",
                window_minutes, error_rate, summary.error_count, total
            )
        } else {
            format!(
                "最近 {} 分钟请求错误率已回落到 {:.1}%（错误 {}/总计 {}）",
                window_minutes, error_rate, summary.error_count, total
            )
        },
        payload: json!({
            "ruleId": rule.id,
            "ruleType": rule.rule_type,
            "thresholdPercent": threshold,
            "windowMinutes": window_minutes,
            "minRequests": min_requests,
            "errorCount": summary.error_count,
            "totalCount": total,
            "errorRate": error_rate,
        }),
    })
}

fn evaluate_all_unavailable(rule: &AlertRule, accounts: &[Account]) -> RuleEvaluation {
    let total_accounts = accounts.len();
    let available_accounts = accounts
        .iter()
        .filter(|account| account_status_is_routable(account.status.as_str()))
        .count();
    let triggered = total_accounts > 0 && available_accounts == 0;
    RuleEvaluation {
        triggered,
        title: format!("告警触发: {}", rule.name),
        message: if triggered {
            format!("当前 {} 个账号全部不可路由", total_accounts)
        } else {
            format!("当前已有 {} 个账号恢复为可路由状态", available_accounts)
        },
        payload: json!({
            "ruleId": rule.id,
            "ruleType": rule.rule_type,
            "totalAccounts": total_accounts,
            "availableAccounts": available_accounts,
        }),
    }
}

fn account_status_is_routable(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "inactive" | "disabled" | "unavailable" | "deactivated"
    )
}

fn parse_rule_config(rule: &AlertRule) -> Value {
    serde_json::from_str(rule.config_json.as_str()).unwrap_or_else(|_| json!({}))
}

fn config_i64(config: &Value, key: &str, fallback: i64) -> i64 {
    config.get(key).and_then(Value::as_i64).unwrap_or(fallback)
}

fn config_f64(config: &Value, key: &str, fallback: f64) -> f64 {
    config.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn config_string_array(config: &Value, key: &str) -> Vec<String> {
    config
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn load_runtime_states(
    storage: &codexmanager_core::storage::Storage,
) -> BTreeMap<String, AlertRuntimeState> {
    storage
        .get_app_setting(ALERT_RUNTIME_STATES_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<BTreeMap<String, AlertRuntimeState>>(&raw).ok())
        .unwrap_or_default()
}

fn save_runtime_states(
    storage: &codexmanager_core::storage::Storage,
    states: &BTreeMap<String, AlertRuntimeState>,
) -> Result<(), String> {
    let raw = serde_json::to_string(states)
        .map_err(|err| format!("serialize alert runtime states failed: {err}"))?;
    storage
        .set_app_setting(ALERT_RUNTIME_STATES_KEY, &raw, now_ts())
        .map_err(|err| format!("persist alert runtime states failed: {err}"))
}
