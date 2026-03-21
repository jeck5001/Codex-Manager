use codexmanager_core::storage::{Account, Token};
use std::sync::atomic::Ordering;

use super::{
    mark_usage_unreachable_if_needed, open_storage, record_usage_refresh_failure,
    SESSION_PROBE_CURSOR, SESSION_PROBE_SAMPLE_SIZE,
};

#[derive(Clone)]
struct SessionProbeTask {
    account: Account,
    token: Token,
}

pub(crate) fn run_session_probe_batch() -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let accounts = storage.list_accounts().map_err(|err| err.to_string())?;
    let tokens = storage.list_tokens().map_err(|err| err.to_string())?;
    let tasks = build_session_probe_tasks(tokens, accounts);
    if tasks.is_empty() {
        return Ok(());
    }

    let total = tasks.len();
    let sample_size = SESSION_PROBE_SAMPLE_SIZE
        .load(Ordering::Relaxed)
        .max(1)
        .min(total);
    let start_cursor = SESSION_PROBE_CURSOR.load(Ordering::Relaxed) % total;
    let indices = session_probe_batch_indices(total, start_cursor, sample_size);

    let mut failures = 0usize;
    let mut successes = 0usize;
    for index in indices {
        let mut task = tasks[index].clone();
        match crate::gateway::probe_models_for_account(&storage, &task.account, &mut task.token) {
            Ok(_) => {
                successes = successes.saturating_add(1);
            }
            Err(err) => {
                failures = failures.saturating_add(1);
                record_usage_refresh_failure(&storage, &task.account.id, &err);
                mark_usage_unreachable_if_needed(&storage, &task.account.id, &err);
                log::warn!(
                    "session probe failed: account_id={} status={} err={}",
                    task.account.id,
                    task.account.status,
                    err
                );
            }
        }
    }

    SESSION_PROBE_CURSOR.store(
        next_session_probe_cursor(total, start_cursor, sample_size),
        Ordering::Relaxed,
    );

    if failures > 0 {
        log::warn!(
            "session probe batch completed with failures: sampled={} total={} successes={} failures={}",
            sample_size,
            total,
            successes,
            failures
        );
    }

    Ok(())
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
        "disabled" | "inactive" | "deactivated" | "unavailable"
    )
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
mod tests {
    use super::{
        account_status_allows_probe, next_session_probe_cursor, session_probe_batch_indices,
    };

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
        assert!(!account_status_allows_probe("disabled"));
        assert!(!account_status_allows_probe("deactivated"));
    }
}
