use gpttools_core::storage::Storage;

use crate::account_availability::{Availability, evaluate_snapshot};
use crate::account_status::set_account_status;

pub(crate) fn should_failover_after_refresh(
    storage: &Storage,
    account_id: &str,
    refresh_result: Result<(), String>,
) -> bool {
    match refresh_result {
        Ok(_) => {
            let snap = storage
                .latest_usage_snapshots_by_account()
                .ok()
                .and_then(|snaps| snaps.into_iter().find(|s| s.account_id == account_id));
            match snap.as_ref().map(evaluate_snapshot) {
                Some(Availability::Unavailable(reason)) => {
                    set_account_status(storage, account_id, "inactive", reason);
                    true
                }
                Some(Availability::Available) => false,
                None => {
                    set_account_status(storage, account_id, "inactive", "usage_missing_snapshot");
                    true
                }
            }
        }
        Err(err) => {
            if err.starts_with("usage endpoint status") {
                set_account_status(storage, account_id, "inactive", "usage_unreachable");
                true
            } else {
                false
            }
        }
    }
}
