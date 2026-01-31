use gpttools_core::storage::{now_ts, Account, Storage, Token};

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
