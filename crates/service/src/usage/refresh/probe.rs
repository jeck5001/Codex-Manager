use codexmanager_core::rpc::types::{
    HealthcheckConfigResult, HealthcheckFailureAccountResult, HealthcheckRunResult,
};
use codexmanager_core::storage::{now_ts, Account, Storage, Token};
use crossbeam_channel::unbounded;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::thread;

use super::{
    mark_usage_unreachable_if_needed, maybe_trigger_auto_account_governance, open_storage,
    record_usage_refresh_failure,
    SESSION_PROBE_CURSOR, SESSION_PROBE_INTERVAL_SECS, SESSION_PROBE_POLLING_ENABLED,
    SESSION_PROBE_SAMPLE_SIZE,
};

static LAST_SESSION_PROBE_RESULT: OnceLock<Mutex<Option<HealthcheckRunResult>>> = OnceLock::new();
const ENV_SESSION_PROBE_WORKERS: &str = "CODEXMANAGER_SESSION_PROBE_WORKERS";
const DEFAULT_SESSION_PROBE_WORKERS: usize = 4;

#[derive(Clone)]
struct SessionProbeTask {
    account: Account,
    token: Token,
}

#[derive(Default)]
struct SessionProbeBatchOutcome {
    success_count: usize,
    failure_count: usize,
    failed_accounts: Vec<HealthcheckFailureAccountResult>,
}

pub(crate) fn current_healthcheck_config() -> HealthcheckConfigResult {
    HealthcheckConfigResult {
        enabled: SESSION_PROBE_POLLING_ENABLED.load(Ordering::Relaxed),
        interval_secs: SESSION_PROBE_INTERVAL_SECS.load(Ordering::Relaxed),
        sample_size: SESSION_PROBE_SAMPLE_SIZE.load(Ordering::Relaxed),
        recent_run: last_session_probe_result(),
    }
}

pub(crate) fn last_session_probe_result() -> Option<HealthcheckRunResult> {
    let lock = LAST_SESSION_PROBE_RESULT.get_or_init(|| Mutex::new(None));
    crate::lock_utils::lock_recover(lock, "last_session_probe_result").clone()
}

fn store_last_session_probe_result(summary: &HealthcheckRunResult) {
    let lock = LAST_SESSION_PROBE_RESULT.get_or_init(|| Mutex::new(None));
    let mut state = crate::lock_utils::lock_recover(lock, "last_session_probe_result");
    *state = Some(summary.clone());
}

pub(crate) fn run_session_probe_batch() -> Result<HealthcheckRunResult, String> {
    let started_at = now_ts();
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let accounts = storage.list_accounts().map_err(|err| err.to_string())?;
    let tokens = storage.list_tokens().map_err(|err| err.to_string())?;
    let tasks = build_session_probe_tasks(tokens, accounts);
    if tasks.is_empty() {
        let summary = HealthcheckRunResult {
            started_at: Some(started_at),
            finished_at: Some(now_ts()),
            total_accounts: 0,
            sampled_accounts: 0,
            success_count: 0,
            failure_count: 0,
            failed_accounts: Vec::new(),
        };
        store_last_session_probe_result(&summary);
        if let Err(err) = maybe_trigger_auto_account_governance() {
            log::warn!("session probe follow-up governance evaluation failed: {}", err);
        }
        if let Err(err) = crate::alert_engine::run_alert_checks_once() {
            log::warn!("session probe follow-up alert evaluation failed: {}", err);
        }
        return Ok(summary);
    }

    let total = tasks.len();
    let sample_size = SESSION_PROBE_SAMPLE_SIZE
        .load(Ordering::Relaxed)
        .max(1)
        .min(total);
    let start_cursor = SESSION_PROBE_CURSOR.load(Ordering::Relaxed) % total;
    let indices = session_probe_batch_indices(total, start_cursor, sample_size);

    let sampled_tasks = indices
        .into_iter()
        .map(|index| tasks[index].clone())
        .collect::<Vec<_>>();
    let outcome = run_session_probe_tasks(sampled_tasks)?;

    SESSION_PROBE_CURSOR.store(
        next_session_probe_cursor(total, start_cursor, sample_size),
        Ordering::Relaxed,
    );

    if outcome.failure_count > 0 {
        log::warn!(
            "session probe batch completed with failures: sampled={} total={} successes={} failures={}",
            sample_size,
            total,
            outcome.success_count,
            outcome.failure_count
        );
    }

    let summary = HealthcheckRunResult {
        started_at: Some(started_at),
        finished_at: Some(now_ts()),
        total_accounts: total as i64,
        sampled_accounts: sample_size as i64,
        success_count: outcome.success_count as i64,
        failure_count: outcome.failure_count as i64,
        failed_accounts: outcome.failed_accounts,
    };
    store_last_session_probe_result(&summary);
    if let Err(err) = maybe_trigger_auto_account_governance() {
        log::warn!("session probe follow-up governance evaluation failed: {}", err);
    }
    if let Err(err) = crate::alert_engine::run_alert_checks_once() {
        log::warn!("session probe follow-up alert evaluation failed: {}", err);
    }

    Ok(summary)
}

fn build_session_probe_tasks(tokens: Vec<Token>, accounts: Vec<Account>) -> Vec<SessionProbeTask> {
    let account_map = accounts
        .into_iter()
        .filter(|account| account_status_allows_probe(account.status.as_str()))
        .map(|account| (account.id.clone(), account))
        .collect::<std::collections::BTreeMap<_, _>>();

    tokens
        .into_iter()
        .filter_map(|token| {
            account_map
                .get(&token.account_id)
                .cloned()
                .map(|account| SessionProbeTask { account, token })
        })
        .collect()
}

fn account_status_allows_probe(status: &str) -> bool {
    !matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "disabled" | "inactive" | "deactivated"
    )
}

fn recover_account_after_success(storage: &Storage, account: &Account) {
    let status = account.status.trim();
    if status.eq_ignore_ascii_case("disabled")
        || status.eq_ignore_ascii_case("inactive")
        || status.eq_ignore_ascii_case("deactivated")
    {
        return;
    }
    crate::account_status::set_account_status(storage, &account.id, "active", "session_probe_ok");
    crate::gateway::clear_account_cooldown(&account.id);
}

fn run_session_probe_tasks(
    tasks: Vec<SessionProbeTask>,
) -> Result<SessionProbeBatchOutcome, String> {
    let total = tasks.len();
    if total == 0 {
        return Ok(SessionProbeBatchOutcome::default());
    }

    let worker_count = session_probe_worker_count(total);
    if worker_count <= 1 {
        let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
        let mut outcome = SessionProbeBatchOutcome::default();
        for task in tasks {
            run_session_probe_task(&storage, task, &mut outcome);
        }
        return Ok(outcome);
    }

    let (sender, receiver) = unbounded::<SessionProbeTask>();
    for task in tasks {
        sender
            .send(task)
            .map_err(|_| "enqueue session probe task failed".to_string())?;
    }
    drop(sender);

    thread::scope(|scope| -> Result<SessionProbeBatchOutcome, String> {
        let mut handles = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let receiver = receiver.clone();
            handles.push(scope.spawn(move || {
                let storage = open_storage().ok_or_else(|| {
                    format!("session probe worker {worker_index} storage unavailable")
                })?;
                let mut outcome = SessionProbeBatchOutcome::default();
                while let Ok(task) = receiver.recv() {
                    run_session_probe_task(&storage, task, &mut outcome);
                }
                Ok::<SessionProbeBatchOutcome, String>(outcome)
            }));
        }

        let mut aggregated = SessionProbeBatchOutcome::default();
        for handle in handles {
            match handle.join() {
                Ok(Ok(outcome)) => {
                    aggregated.success_count = aggregated
                        .success_count
                        .saturating_add(outcome.success_count);
                    aggregated.failure_count = aggregated
                        .failure_count
                        .saturating_add(outcome.failure_count);
                    aggregated.failed_accounts.extend(outcome.failed_accounts);
                }
                Ok(Err(err)) => return Err(err),
                Err(_) => return Err("session probe worker panicked".to_string()),
            }
        }
        Ok(aggregated)
    })
}

fn run_session_probe_task(
    storage: &Storage,
    mut task: SessionProbeTask,
    outcome: &mut SessionProbeBatchOutcome,
) {
    match crate::gateway::probe_models_for_account(storage, &task.account, &mut task.token) {
        Ok(_) => {
            outcome.success_count = outcome.success_count.saturating_add(1);
            recover_account_after_success(storage, &task.account);
        }
        Err(err) => {
            outcome.failure_count = outcome.failure_count.saturating_add(1);
            record_usage_refresh_failure(storage, &task.account.id, &err);
            mark_usage_unreachable_if_needed(storage, &task.account.id, &err);
            outcome
                .failed_accounts
                .push(HealthcheckFailureAccountResult {
                    account_id: task.account.id.clone(),
                    label: Some(task.account.label.clone()),
                    reason: err.clone(),
                });
            log::warn!(
                "session probe failed: account_id={} status={} err={}",
                task.account.id,
                task.account.status,
                err
            );
        }
    }
}

fn session_probe_worker_count(total_tasks: usize) -> usize {
    parse_session_probe_worker_count(std::env::var(ENV_SESSION_PROBE_WORKERS).ok(), total_tasks)
}

fn parse_session_probe_worker_count(raw: Option<String>, total_tasks: usize) -> usize {
    if total_tasks == 0 {
        return 0;
    }

    raw.as_deref()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_SESSION_PROBE_WORKERS)
        .min(total_tasks)
        .max(1)
}

fn session_probe_batch_indices(total: usize, cursor: usize, sample_size: usize) -> Vec<usize> {
    if total == 0 || sample_size == 0 {
        return Vec::new();
    }
    let start = cursor % total;
    (0..sample_size.min(total))
        .map(|offset| (start + offset) % total)
        .collect()
}

fn next_session_probe_cursor(total: usize, cursor: usize, processed: usize) -> usize {
    if total == 0 {
        return 0;
    }
    (cursor % total + processed.min(total)) % total
}

#[cfg(test)]
pub(crate) fn clear_session_probe_state_for_tests() {
    let lock = LAST_SESSION_PROBE_RESULT.get_or_init(|| Mutex::new(None));
    *crate::lock_utils::lock_recover(lock, "last_session_probe_result") = None;
    SESSION_PROBE_CURSOR.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use codexmanager_core::storage::{now_ts, Account, Storage, Token};

    use super::{
        account_status_allows_probe, build_session_probe_tasks, next_session_probe_cursor,
        parse_session_probe_worker_count, recover_account_after_success,
        session_probe_batch_indices,
    };

    fn account(id: &str, status: &str) -> Account {
        let now = now_ts();
        Account {
            id: id.to_string(),
            label: id.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some(format!("acct-{id}")),
            workspace_id: Some(format!("org-{id}")),
            group_name: None,
            sort: 0,
            status: status.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn token(account_id: &str) -> Token {
        let now = now_ts();
        Token {
            account_id: account_id.to_string(),
            id_token: "header.payload.sig".to_string(),
            access_token: format!("access-{account_id}"),
            refresh_token: format!("refresh-{account_id}"),
            api_key_access_token: None,
            last_refresh: now,
        }
    }

    #[test]
    fn session_probe_indices_wrap_round_robin() {
        assert_eq!(session_probe_batch_indices(5, 3, 2), vec![3, 4]);
        assert_eq!(session_probe_batch_indices(5, 4, 3), vec![4, 0, 1]);
        assert_eq!(next_session_probe_cursor(5, 4, 3), 2);
    }

    #[test]
    fn session_probe_skips_non_routable_statuses() {
        assert!(account_status_allows_probe("active"));
        assert!(account_status_allows_probe("healthy"));
        assert!(account_status_allows_probe("unavailable"));
        assert!(!account_status_allows_probe("disabled"));
        assert!(!account_status_allows_probe("deactivated"));
    }

    #[test]
    fn session_probe_worker_count_uses_default_and_caps_at_batch_size() {
        assert_eq!(parse_session_probe_worker_count(None, 2), 2);
        assert_eq!(
            parse_session_probe_worker_count(Some("16".to_string()), 3),
            3
        );
        assert_eq!(
            parse_session_probe_worker_count(Some("0".to_string()), 5),
            4
        );
    }

    #[test]
    fn build_session_probe_tasks_skips_disabled_accounts() {
        let tasks = build_session_probe_tasks(
            vec![token("acc-unavailable"), token("acc-disabled")],
            vec![
                account("acc-unavailable", "unavailable"),
                account("acc-disabled", "disabled"),
            ],
        );

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].account.id, "acc-unavailable");
        assert_eq!(tasks[0].token.account_id, "acc-unavailable");
    }

    #[test]
    fn recover_account_after_success_restores_unavailable_but_keeps_disabled() {
        let storage = Storage::open_in_memory().expect("open");
        storage.init().expect("init");

        let unavailable = account("acc-recover", "unavailable");
        let disabled = account("acc-disabled", "disabled");
        storage
            .insert_account(&unavailable)
            .expect("insert unavailable account");
        storage
            .insert_account(&disabled)
            .expect("insert disabled account");

        recover_account_after_success(&storage, &unavailable);
        recover_account_after_success(&storage, &disabled);

        let recovered = storage
            .find_account_by_id("acc-recover")
            .expect("find recovered")
            .expect("stored recovered");
        assert_eq!(recovered.status, "active");

        let still_disabled = storage
            .find_account_by_id("acc-disabled")
            .expect("find disabled")
            .expect("stored disabled");
        assert_eq!(still_disabled.status, "disabled");
    }
}
