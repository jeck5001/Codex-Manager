use gpttools_core::storage::{Account, Storage, Token, UsageSnapshotRecord};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::account_availability::is_available;

static CANDIDATE_CURSOR: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn rotate_candidates_for_fairness(candidates: &mut Vec<(Account, Token)>) {
    if candidates.len() <= 1 {
        return;
    }
    let cursor = CANDIDATE_CURSOR.fetch_add(1, Ordering::Relaxed);
    let offset = cursor % candidates.len();
    if offset > 0 {
        // 中文注释：轮转起点可把并发请求均匀打散到不同账号，降低首账号被并发打爆的概率。
        candidates.rotate_left(offset);
    }
}

pub(crate) fn collect_gateway_candidates(storage: &Storage) -> Result<Vec<(Account, Token)>, String> {
    // 选择可用账号作为网关上游候选
    let accounts = storage.list_accounts().map_err(|e| e.to_string())?;
    let tokens = storage.list_tokens().map_err(|e| e.to_string())?;
    let snaps = storage
        .latest_usage_snapshots_by_account()
        .map_err(|e| e.to_string())?;
    let mut token_map = HashMap::new();
    for token in tokens {
        token_map.insert(token.account_id.clone(), token);
    }
    let mut snap_map = HashMap::new();
    for snap in snaps {
        snap_map.insert(snap.account_id.clone(), snap);
    }

    let mut out = Vec::new();
    for account in &accounts {
        if account.status != "active" {
            continue;
        }
        let token = match token_map.get(&account.id) {
            Some(token) => token.clone(),
            None => continue,
        };
        let usage = snap_map.get(&account.id);
        if !is_available(usage) {
            continue;
        }
        out.push((account.clone(), token));
    }
    if out.is_empty() {
        let mut fallback = Vec::new();
        for account in &accounts {
            let token = match token_map.get(&account.id) {
                Some(token) => token.clone(),
                None => continue,
            };
            let usage = snap_map.get(&account.id);
            if !fallback_allowed(usage) {
                continue;
            }
            fallback.push((account.clone(), token));
        }
        if !fallback.is_empty() {
            log::warn!("gateway fallback: no active accounts, using {} candidates", fallback.len());
            return Ok(fallback);
        }
    }
    if out.is_empty() {
        log_no_candidates(&accounts, &token_map, &snap_map);
    }
    Ok(out)
}

fn fallback_allowed(usage: Option<&UsageSnapshotRecord>) -> bool {
    if let Some(record) = usage {
        if let Some(value) = record.used_percent {
            if value >= 100.0 {
                return false;
            }
        }
        if let Some(value) = record.secondary_used_percent {
            if value >= 100.0 {
                return false;
            }
        }
    }
    true
}

fn log_no_candidates(
    accounts: &[Account],
    token_map: &HashMap<String, Token>,
    snap_map: &HashMap<String, UsageSnapshotRecord>,
) {
    let db_path = std::env::var("GPTTOOLS_DB_PATH").unwrap_or_else(|_| "<unset>".to_string());
    log::warn!(
        "gateway no candidates: db_path={}, accounts={}, tokens={}, snapshots={}",
        db_path,
        accounts.len(),
        token_map.len(),
        snap_map.len()
    );
    for account in accounts {
        let usage = snap_map.get(&account.id);
        log::warn!(
            "gateway account: id={}, status={}, has_token={}, primary=({:?}/{:?}) secondary=({:?}/{:?})",
            account.id,
            account.status,
            token_map.contains_key(&account.id),
            usage.and_then(|u| u.used_percent),
            usage.and_then(|u| u.window_minutes),
            usage.and_then(|u| u.secondary_used_percent),
            usage.and_then(|u| u.secondary_window_minutes),
        );
    }
}
