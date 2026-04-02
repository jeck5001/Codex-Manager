use super::*;

/// 函数 `same_account_reuses_exchange_lock`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn same_account_reuses_exchange_lock() {
    let _guard = crate::test_env_guard();
    clear_account_token_exchange_locks_for_tests();
    let first = account_token_exchange_lock("acc-1");
    let second = account_token_exchange_lock("acc-1");
    assert!(Arc::ptr_eq(&first, &second));
}

/// 函数 `stale_unshared_exchange_lock_entry_is_reclaimed`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn stale_unshared_exchange_lock_entry_is_reclaimed() {
    let _guard = crate::test_env_guard();
    clear_account_token_exchange_locks_for_tests();
    let first = account_token_exchange_lock("acc-1");
    let weak = Arc::downgrade(&first);
    drop(first);

    let lock = ACCOUNT_TOKEN_EXCHANGE_LOCKS
        .get_or_init(|| Mutex::new(AccountTokenExchangeLockTable::default()));
    let mut table = lock.lock().expect("token exchange table lock");
    let now = now_ts();
    table
        .entries
        .get_mut("acc-1")
        .expect("token exchange entry")
        .last_seen_at = now - ACCOUNT_TOKEN_EXCHANGE_LOCK_TTL_SECS - 1;
    table.last_cleanup_at = now - ACCOUNT_TOKEN_EXCHANGE_LOCK_CLEANUP_INTERVAL_SECS - 1;
    drop(table);

    let _second = account_token_exchange_lock("acc-1");
    assert!(weak.upgrade().is_none());
}

/// 函数 `stale_shared_exchange_lock_entry_is_not_reclaimed`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn stale_shared_exchange_lock_entry_is_not_reclaimed() {
    let _guard = crate::test_env_guard();
    clear_account_token_exchange_locks_for_tests();
    let first = account_token_exchange_lock("acc-1");

    let lock = ACCOUNT_TOKEN_EXCHANGE_LOCKS
        .get_or_init(|| Mutex::new(AccountTokenExchangeLockTable::default()));
    let mut table = lock.lock().expect("token exchange table lock");
    let now = now_ts();
    table
        .entries
        .get_mut("acc-1")
        .expect("token exchange entry")
        .last_seen_at = now - ACCOUNT_TOKEN_EXCHANGE_LOCK_TTL_SECS - 1;
    table.last_cleanup_at = now - ACCOUNT_TOKEN_EXCHANGE_LOCK_CLEANUP_INTERVAL_SECS - 1;
    drop(table);

    let second = account_token_exchange_lock("acc-1");
    assert!(Arc::ptr_eq(&first, &second));
}

/// 函数 `fallback_to_access_token_uses_runtime_access_token_when_exchange_fails`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn fallback_to_access_token_uses_runtime_access_token_when_exchange_fails() {
    let token = Token {
        account_id: "acc-2".to_string(),
        id_token: "runtime-id-token".to_string(),
        access_token: "runtime-access-token".to_string(),
        refresh_token: String::new(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };

    let bearer =
        fallback_to_access_token(&token, "api key exchange failed").expect("fallback bearer");
    assert_eq!(bearer, "runtime-access-token");
}
