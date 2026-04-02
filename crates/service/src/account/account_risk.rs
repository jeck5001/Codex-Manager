use codexmanager_core::storage::{now_ts, Account};

pub(crate) const ENV_NEW_ACCOUNT_PROTECTION_DAYS: &str = "CODEXMANAGER_NEW_ACCOUNT_PROTECTION_DAYS";
const DEFAULT_NEW_ACCOUNT_PROTECTION_DAYS: u64 = 3;
const MAX_NEW_ACCOUNT_PROTECTION_DAYS: u64 = 30;
const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
const NEW_ACCOUNT_PROTECTION_REASON: &str = "新号保护期内，已自动降优先级";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NewAccountProtectionState {
    pub until: i64,
    pub reason: &'static str,
}

pub(crate) fn current_new_account_protection_days() -> u64 {
    std::env::var(ENV_NEW_ACCOUNT_PROTECTION_DAYS)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(|days| days.min(MAX_NEW_ACCOUNT_PROTECTION_DAYS))
        .unwrap_or(DEFAULT_NEW_ACCOUNT_PROTECTION_DAYS)
}

pub(crate) fn current_new_account_protection_window_secs() -> i64 {
    let days = current_new_account_protection_days();
    let secs = (days as i64).saturating_mul(SECONDS_PER_DAY);
    secs.max(0)
}

pub(crate) fn derive_new_account_protection_state(
    account: &Account,
) -> Option<NewAccountProtectionState> {
    derive_new_account_protection_state_with_window(
        account,
        now_ts(),
        current_new_account_protection_window_secs(),
    )
}

pub(crate) fn derive_new_account_protection_state_with_window(
    account: &Account,
    now: i64,
    window_secs: i64,
) -> Option<NewAccountProtectionState> {
    if window_secs <= 0 || account.created_at <= 0 {
        return None;
    }
    let until = account.created_at.saturating_add(window_secs);
    if until <= now {
        return None;
    }
    Some(NewAccountProtectionState {
        until,
        reason: NEW_ACCOUNT_PROTECTION_REASON,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        current_new_account_protection_window_secs,
        derive_new_account_protection_state_with_window, Account, ENV_NEW_ACCOUNT_PROTECTION_DAYS,
    };

    fn sample_account(created_at: i64) -> Account {
        Account {
            id: "acc-risk".to_string(),
            label: "risk".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at,
            updated_at: created_at,
        }
    }

    #[test]
    fn new_account_protection_defaults_to_three_days() {
        let previous = std::env::var(ENV_NEW_ACCOUNT_PROTECTION_DAYS).ok();
        std::env::remove_var(ENV_NEW_ACCOUNT_PROTECTION_DAYS);

        assert_eq!(
            current_new_account_protection_window_secs(),
            3 * 24 * 60 * 60
        );

        if let Some(value) = previous {
            std::env::set_var(ENV_NEW_ACCOUNT_PROTECTION_DAYS, value);
        }
    }

    #[test]
    fn derive_new_account_protection_state_marks_recent_account() {
        let now = 1_700_000_000;
        let account = sample_account(now - 3600);
        let state =
            derive_new_account_protection_state_with_window(&account, now, 2 * 24 * 60 * 60)
                .expect("recent account should be protected");

        assert_eq!(state.until, account.created_at + 2 * 24 * 60 * 60);
    }

    #[test]
    fn derive_new_account_protection_state_ignores_mature_account() {
        let now = 1_700_000_000;
        let account = sample_account(now - 10 * 24 * 60 * 60);

        assert!(
            derive_new_account_protection_state_with_window(&account, now, 3 * 24 * 60 * 60)
                .is_none()
        );
    }
}
