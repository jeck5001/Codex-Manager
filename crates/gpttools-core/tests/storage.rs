use gpttools_core::storage::{now_ts, Account, Storage, Token, UsageSnapshotRecord};

#[test]
fn storage_can_insert_account_and_token() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let account = Account {
        id: "acc-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct_123".to_string()),
        workspace_id: Some("org_123".to_string()),
        workspace_name: None,
        note: None,
        tags: None,
        group_name: None,
        sort: 0,
        status: "healthy".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    let token = Token {
        account_id: "acc-1".to_string(),
        id_token: "id".to_string(),
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };
    storage.insert_token(&token).expect("insert token");

    assert_eq!(storage.account_count().expect("count accounts"), 1);
    assert_eq!(storage.token_count().expect("count tokens"), 1);
}

#[test]
fn storage_login_session_roundtrip() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let session = gpttools_core::storage::LoginSession {
        login_id: "login-1".to_string(),
        code_verifier: "verifier".to_string(),
        state: "state".to_string(),
        status: "pending".to_string(),
        error: None,
        note: None,
        tags: None,
        group_name: None,
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage
        .insert_login_session(&session)
        .expect("insert session");
    let loaded = storage
        .get_login_session("login-1")
        .expect("load session")
        .expect("session exists");
    assert_eq!(loaded.status, "pending");
}

#[test]
fn storage_can_update_account_status() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let account = Account {
        id: "acc-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct_123".to_string()),
        workspace_id: Some("org_123".to_string()),
        workspace_name: None,
        note: None,
        tags: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    storage
        .update_account_status("acc-1", "inactive")
        .expect("update status");

    let loaded = storage
        .list_accounts()
        .expect("list accounts")
        .into_iter()
        .find(|acc| acc.id == "acc-1")
        .expect("account exists");

    assert_eq!(loaded.status, "inactive");
}

#[test]
fn latest_usage_snapshots_break_ties_by_latest_id() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let tie_ts = now_ts();

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent: Some(10.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: tie_ts,
        })
        .expect("insert first snapshot");

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-1".to_string(),
            used_percent: Some(30.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: tie_ts,
        })
        .expect("insert second snapshot with same timestamp");

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-2".to_string(),
            used_percent: Some(50.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: tie_ts - 10,
        })
        .expect("insert snapshot for acc-2");

    let latest = storage
        .latest_usage_snapshots_by_account()
        .expect("read latest snapshots");

    assert_eq!(latest.len(), 2);
    assert_eq!(latest[0].account_id, "acc-1");

    let acc1 = latest
        .iter()
        .find(|item| item.account_id == "acc-1")
        .expect("acc-1 exists");
    assert_eq!(acc1.used_percent, Some(30.0));
}
