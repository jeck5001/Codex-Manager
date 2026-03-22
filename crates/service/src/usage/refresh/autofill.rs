use codexmanager_core::storage::{Storage, UsageSnapshotRecord};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::{
    open_storage, AUTO_REGISTER_POOL_ENABLED, AUTO_REGISTER_READY_ACCOUNT_COUNT,
    AUTO_REGISTER_READY_REMAIN_PERCENT,
};

static AUTO_REGISTER_POOL_WORKER_RUNNING: AtomicBool = AtomicBool::new(false);

const AUTO_REGISTER_TASK_POLL_INTERVAL_SECS: u64 = 3;
const AUTO_REGISTER_TASK_TIMEOUT_SECS: u64 = 20 * 60;

#[derive(Debug, Clone)]
struct AutoRegisterServicePlan {
    service_type: String,
    email_service_id: Option<i64>,
    label: String,
}

pub(crate) fn maybe_trigger_auto_register_pool_fill() -> Result<(), String> {
    if !AUTO_REGISTER_POOL_ENABLED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let ready_account_target = AUTO_REGISTER_READY_ACCOUNT_COUNT.load(Ordering::Relaxed);
    let ready_remain_percent = AUTO_REGISTER_READY_REMAIN_PERCENT
        .load(Ordering::Relaxed)
        .min(100);

    if ready_account_target == 0 {
        return Ok(());
    }

    if AUTO_REGISTER_POOL_WORKER_RUNNING.load(Ordering::Relaxed) {
        return Ok(());
    }

    if has_active_remote_register_tasks()? {
        log::info!("auto register pool fill skipped: active register tasks already running");
        return Ok(());
    }

    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let ready_account_count = count_ready_accounts(&storage, ready_remain_percent)?;
    let fill_count = desired_fill_count(ready_account_count, ready_account_target);
    if fill_count == 0 {
        return Ok(());
    }

    let plan = resolve_auto_register_service_plan()?;
    if AUTO_REGISTER_POOL_WORKER_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_err()
    {
        return Ok(());
    }

    let worker_plan = plan.clone();
    let _ = thread::Builder::new()
        .name("auto-register-pool-fill".to_string())
        .spawn(move || {
            let started_at = Instant::now();
            let result = run_auto_register_pool_fill(fill_count, worker_plan.clone());
            match result {
                Ok(imported) => {
                    log::info!(
                        "auto register pool fill finished: requested={} imported={} service={} elapsed_ms={}",
                        fill_count,
                        imported,
                        worker_plan.label,
                        started_at.elapsed().as_millis()
                    );
                }
                Err(err) => {
                    log::warn!(
                        "auto register pool fill failed: requested={} service={} err={}",
                        fill_count,
                        worker_plan.label,
                        err
                    );
                }
            }
            AUTO_REGISTER_POOL_WORKER_RUNNING.store(false, Ordering::SeqCst);
        })
        .map_err(|err| {
            AUTO_REGISTER_POOL_WORKER_RUNNING.store(false, Ordering::SeqCst);
            format!("spawn auto register pool fill worker failed: {err}")
        })?;

    log::info!(
        "auto register pool fill triggered: ready_accounts={} trigger_threshold={} remain_threshold={} fill_count={} service={}",
        ready_account_count,
        ready_account_target,
        ready_remain_percent,
        fill_count,
        plan.label
    );
    Ok(())
}

fn run_auto_register_pool_fill(
    fill_count: usize,
    plan: AutoRegisterServicePlan,
) -> Result<usize, String> {
    let started = crate::account_register::start_register_batch(
        plan.service_type.as_str(),
        plan.email_service_id,
        None,
        i64::try_from(fill_count).map_err(|_| "fill count overflow".to_string())?,
        0,
        0,
        i64::try_from(fill_count.min(3)).map_err(|_| "concurrency overflow".to_string())?,
        "parallel",
    )?;

    let task_uuids = extract_task_uuids(&started);
    if task_uuids.is_empty() {
        return Err("auto register batch returned no task uuids".to_string());
    }

    let deadline = Instant::now() + Duration::from_secs(AUTO_REGISTER_TASK_TIMEOUT_SECS);
    let mut pending = task_uuids.iter().cloned().collect::<HashSet<_>>();
    let mut imported = 0usize;

    while !pending.is_empty() && Instant::now() < deadline {
        let current = pending.iter().cloned().collect::<Vec<_>>();
        for task_uuid in current {
            let snapshot = match crate::account_register::read_register_task(task_uuid.as_str()) {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    log::warn!(
                        "auto register pool fill read task failed: task_uuid={} err={}",
                        task_uuid,
                        err
                    );
                    continue;
                }
            };

            if !is_register_task_terminal(snapshot.status()) {
                continue;
            }

            pending.remove(task_uuid.as_str());
            if snapshot.can_import() {
                match crate::account_register::import_register_task(task_uuid.as_str()) {
                    Ok(imported_result) => {
                        imported = imported.saturating_add(1);
                        let account_id = imported_result
                            .get("accountId")
                            .or_else(|| imported_result.get("account_id"))
                            .and_then(Value::as_str)
                            .unwrap_or("--");
                        log::info!(
                            "auto register pool fill imported account: task_uuid={} email={} account_id={}",
                            task_uuid,
                            snapshot.email().unwrap_or("--"),
                            account_id
                        );
                    }
                    Err(err) => {
                        log::warn!(
                            "auto register pool fill import failed: task_uuid={} email={} err={}",
                            task_uuid,
                            snapshot.email().unwrap_or("--"),
                            err
                        );
                    }
                }
                continue;
            }

            log::warn!(
                "auto register pool fill task ended without importable account: task_uuid={} status={} err={}",
                task_uuid,
                snapshot.status(),
                snapshot.error_message().unwrap_or("--")
            );
        }

        if !pending.is_empty() {
            thread::sleep(Duration::from_secs(AUTO_REGISTER_TASK_POLL_INTERVAL_SECS));
        }
    }

    if !pending.is_empty() {
        return Err(format!(
            "auto register tasks timed out: pending={}",
            pending.len()
        ));
    }

    Ok(imported)
}

fn desired_fill_count(ready_account_count: usize, ready_account_target: usize) -> usize {
    if ready_account_count > ready_account_target {
        0
    } else {
        ready_account_target
            .saturating_add(1)
            .saturating_sub(ready_account_count)
            .max(1)
    }
}

fn count_ready_accounts(storage: &Storage, remain_percent_threshold: u64) -> Result<usize, String> {
    let active_accounts = storage
        .list_accounts()
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|account| account_status_is_routable(account.status.as_str()))
        .collect::<Vec<_>>();
    if active_accounts.is_empty() {
        return Ok(0);
    }

    let token_account_ids = storage
        .list_tokens()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|token| token.account_id)
        .collect::<HashSet<_>>();
    if token_account_ids.is_empty() {
        return Ok(0);
    }

    let snapshots = storage
        .latest_usage_snapshots_by_account()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|snapshot| (snapshot.account_id.clone(), snapshot))
        .collect::<HashMap<_, _>>();

    Ok(active_accounts
        .into_iter()
        .filter(|account| token_account_ids.contains(&account.id))
        .filter(|account| {
            snapshots
                .get(&account.id)
                .map(|snapshot| snapshot_meets_ready_threshold(snapshot, remain_percent_threshold))
                .unwrap_or(false)
        })
        .count())
}

fn account_status_is_routable(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "inactive" | "disabled" | "unavailable" | "deactivated"
    )
}

fn snapshot_meets_ready_threshold(
    snapshot: &UsageSnapshotRecord,
    remain_percent_threshold: u64,
) -> bool {
    let threshold = remain_percent_threshold as f64;
    let Some(primary_used) = snapshot.used_percent else {
        return false;
    };
    if snapshot.window_minutes.is_none() || remaining_percent(primary_used) < threshold {
        return false;
    }

    match (
        snapshot.secondary_used_percent,
        snapshot.secondary_window_minutes,
    ) {
        (None, None) => true,
        (Some(secondary_used), Some(_)) => remaining_percent(secondary_used) >= threshold,
        _ => false,
    }
}

fn remaining_percent(used_percent: f64) -> f64 {
    (100.0 - used_percent).clamp(0.0, 100.0)
}

fn has_active_remote_register_tasks() -> Result<bool, String> {
    for status in ["pending", "running"] {
        let payload = crate::account_register::list_register_tasks(1, 100, Some(status))?;
        let total = payload
            .get("total")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| {
                payload
                    .get("tasks")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0)
            });
        if total > 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn resolve_auto_register_service_plan() -> Result<AutoRegisterServicePlan, String> {
    let payload = crate::account_register::available_register_services()?;

    if group_available(&payload, &["tempmail"]) {
        return Ok(AutoRegisterServicePlan {
            service_type: "tempmail".to_string(),
            email_service_id: None,
            label: "tempmail".to_string(),
        });
    }
    if let Some((id, label)) = first_service_from_group(
        &payload,
        &["customDomain", "custom_domain", "custom-domain"],
    ) {
        return Ok(AutoRegisterServicePlan {
            service_type: "custom_domain".to_string(),
            email_service_id: Some(id),
            label,
        });
    }
    if let Some((id, label)) = first_service_from_group(&payload, &["outlook"]) {
        return Ok(AutoRegisterServicePlan {
            service_type: "outlook".to_string(),
            email_service_id: Some(id),
            label,
        });
    }
    if let Some((id, label)) = first_service_from_group(&payload, &["tempMail", "temp_mail"]) {
        return Ok(AutoRegisterServicePlan {
            service_type: "temp_mail".to_string(),
            email_service_id: Some(id),
            label,
        });
    }

    Err("no available email service for auto register".to_string())
}

fn group_available(payload: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        payload
            .get(*key)
            .and_then(Value::as_object)
            .and_then(|group| group.get("available"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn first_service_from_group(payload: &Value, keys: &[&str]) -> Option<(i64, String)> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_object)
            .and_then(|group| group.get("services"))
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find_map(|item| {
                    let id = item.get("id").and_then(Value::as_i64)?;
                    if id < 1 {
                        return None;
                    }
                    let label = item
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or(*key)
                        .to_string();
                    Some((id, label))
                })
            })
    })
}

fn extract_task_uuids(payload: &Value) -> Vec<String> {
    payload
        .get("taskUuids")
        .or_else(|| payload.get("task_uuids"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_register_task_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "completed" | "failed" | "cancelled"
    )
}

#[cfg(test)]
mod tests {
    use super::{desired_fill_count, snapshot_meets_ready_threshold};
    use codexmanager_core::storage::UsageSnapshotRecord;

    fn snapshot(
        used_percent: Option<f64>,
        secondary_used_percent: Option<f64>,
    ) -> UsageSnapshotRecord {
        UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent,
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent,
            secondary_window_minutes: secondary_used_percent.map(|_| 10080),
            secondary_resets_at: None,
            credits_json: None,
            captured_at: 1,
        }
    }

    #[test]
    fn fill_count_only_triggers_when_ready_accounts_hit_threshold() {
        assert_eq!(desired_fill_count(4, 3), 0);
        assert_eq!(desired_fill_count(3, 3), 1);
        assert_eq!(desired_fill_count(2, 3), 2);
        assert_eq!(desired_fill_count(0, 1), 2);
    }

    #[test]
    fn ready_threshold_requires_all_known_usage_windows_to_meet_remaining_percent() {
        assert!(snapshot_meets_ready_threshold(
            &snapshot(Some(70.0), None),
            20
        ));
        assert!(!snapshot_meets_ready_threshold(
            &snapshot(Some(81.0), None),
            20
        ));
        assert!(snapshot_meets_ready_threshold(
            &snapshot(Some(70.0), Some(75.0)),
            20
        ));
        assert!(!snapshot_meets_ready_threshold(
            &snapshot(Some(70.0), Some(81.0)),
            20
        ));
    }
}
