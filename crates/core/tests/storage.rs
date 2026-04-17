use codexmanager_core::storage::{
    now_ts, Account, AlertChannel, AlertRule, ApiKey, ModelPricing, PluginRecord, RequestLog,
    RequestTokenStat, Storage, Token, UsageSnapshotRecord,
};
use std::ffi::OsString;

struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}

fn override_gateway_quota_env(enabled: &str, threshold: &str) -> EnvRestore {
    let keys = [
        "CODEXMANAGER_GATEWAY_QUOTA_PROTECTION_ENABLED",
        "CODEXMANAGER_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT",
    ];
    let previous = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    std::env::set_var(keys[0], enabled);
    std::env::set_var(keys[1], threshold);
    EnvRestore(previous)
}

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
fn storage_can_find_token_and_account_by_account_id() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let account = Account {
        id: "acc-find-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct_find".to_string()),
        workspace_id: Some("org_find".to_string()),
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    let token = Token {
        account_id: "acc-find-1".to_string(),
        id_token: "id-find".to_string(),
        access_token: "access-find".to_string(),
        refresh_token: "refresh-find".to_string(),
        api_key_access_token: Some("api-key-find".to_string()),
        last_refresh: now_ts(),
    };
    storage.insert_token(&token).expect("insert token");

    let found_account = storage
        .find_account_by_id("acc-find-1")
        .expect("find account")
        .expect("account exists");
    assert_eq!(found_account.id, "acc-find-1");

    let found_token = storage
        .find_token_by_account_id("acc-find-1")
        .expect("find token")
        .expect("token exists");
    assert_eq!(found_token.account_id, "acc-find-1");
    assert_eq!(
        found_token.api_key_access_token.as_deref(),
        Some("api-key-find")
    );

    assert!(storage
        .find_account_by_id("missing-account")
        .expect("find missing account")
        .is_none());
    assert!(storage
        .find_token_by_account_id("missing-account")
        .expect("find missing token")
        .is_none());
}

#[test]
fn token_upsert_keeps_refresh_schedule_columns() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let account = Account {
        id: "acc-schedule-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    let token = Token {
        account_id: "acc-schedule-1".to_string(),
        id_token: "id-1".to_string(),
        access_token: "access-1".to_string(),
        refresh_token: "refresh-1".to_string(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };
    storage.insert_token(&token).expect("insert token");
    storage
        .update_token_refresh_schedule("acc-schedule-1", Some(4_102_444_800), Some(4_102_444_200))
        .expect("set schedule");

    let token2 = Token {
        account_id: "acc-schedule-1".to_string(),
        id_token: "id-2".to_string(),
        access_token: "access-2".to_string(),
        refresh_token: "refresh-2".to_string(),
        api_key_access_token: Some("api-key".to_string()),
        last_refresh: now_ts(),
    };
    storage.insert_token(&token2).expect("upsert token");

    let due = storage
        .list_tokens_due_for_refresh(4_102_444_100, 10)
        .expect("list due");
    assert!(due.is_empty());
    let due2 = storage
        .list_tokens_due_for_refresh(4_102_444_300, 10)
        .expect("list due2");
    assert_eq!(due2.len(), 1);
    assert_eq!(due2[0].account_id, "acc-schedule-1");
}

#[test]
fn storage_login_session_roundtrip() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let session = codexmanager_core::storage::LoginSession {
        login_id: "login-1".to_string(),
        code_verifier: "verifier".to_string(),
        state: "state".to_string(),
        status: "pending".to_string(),
        error: None,
        workspace_id: Some("org_123".to_string()),
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
    assert_eq!(loaded.workspace_id.as_deref(), Some("org_123"));
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
fn storage_updates_account_status_only_when_changed() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let account = Account {
        id: "acc-conditional-1".to_string(),
        label: "main".to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: Some("acct_123".to_string()),
        workspace_id: Some("org_123".to_string()),
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    storage.insert_account(&account).expect("insert account");

    let unchanged = storage
        .update_account_status_if_changed("acc-conditional-1", "active")
        .expect("conditional update unchanged");
    assert!(!unchanged);

    let changed = storage
        .update_account_status_if_changed("acc-conditional-1", "inactive")
        .expect("conditional update changed");
    assert!(changed);

    let loaded = storage
        .find_account_by_id("acc-conditional-1")
        .expect("find account")
        .expect("account exists");
    assert_eq!(loaded.status, "inactive");
}

#[test]
fn storage_can_roundtrip_api_key_response_cache_config() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let now = now_ts();
    let key = ApiKey {
        id: "gk-response-cache-roundtrip".to_string(),
        name: Some("cache".to_string()),
        model_slug: Some("gpt-5.3-codex".to_string()),
        reasoning_effort: Some("medium".to_string()),
        client_type: "codex".to_string(),
        protocol_type: "openai_compat".to_string(),
        auth_scheme: "authorization_bearer".to_string(),
        upstream_base_url: None,
        static_headers_json: None,
        key_hash: "hash-response-cache-roundtrip".to_string(),
        status: "active".to_string(),
        created_at: now,
        last_used_at: None,
        expires_at: None,
    };
    storage.insert_api_key(&key).expect("insert api key");

    assert!(storage
        .find_api_key_response_cache_config_by_id(&key.id)
        .expect("find empty config")
        .is_none());

    storage
        .upsert_api_key_response_cache_config(&key.id, true)
        .expect("enable response cache");
    let enabled = storage
        .find_api_key_response_cache_config_by_id(&key.id)
        .expect("find enabled config")
        .expect("enabled config exists");
    assert!(enabled.enabled);

    storage
        .upsert_api_key_response_cache_config(&key.id, false)
        .expect("disable response cache");
    let disabled = storage
        .find_api_key_response_cache_config_by_id(&key.id)
        .expect("find disabled config")
        .expect("disabled config exists");
    assert!(!disabled.enabled);
}

#[test]
fn storage_account_usage_filters_support_sql_pagination() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let now = now_ts();

    let accounts = [
        (
            "acc-active-1",
            "active",
            Some("alpha"),
            Some(10.0),
            Some(10.0),
        ),
        ("acc-low-1", "active", Some("alpha"), Some(85.0), Some(85.0)),
        (
            "acc-inactive-low",
            "inactive",
            Some("beta"),
            Some(90.0),
            Some(90.0),
        ),
        (
            "acc-healthy-1",
            "healthy",
            Some("beta"),
            Some(30.0),
            Some(30.0),
        ),
        ("acc-no-snapshot", "active", Some("beta"), None, None),
    ];

    for (idx, (id, status, group_name, primary_used, low_used)) in accounts.iter().enumerate() {
        storage
            .insert_account(&Account {
                id: (*id).to_string(),
                label: format!("Account {idx}"),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: group_name.map(|value| value.to_string()),
                sort: idx as i64,
                status: (*status).to_string(),
                created_at: now + idx as i64,
                updated_at: now + idx as i64,
            })
            .expect("insert account");

        if let Some(used_percent) = primary_used {
            storage
                .insert_usage_snapshot(&UsageSnapshotRecord {
                    account_id: (*id).to_string(),
                    used_percent: Some(*used_percent),
                    window_minutes: Some(300),
                    resets_at: None,
                    secondary_used_percent: Some(low_used.expect("secondary used")),
                    secondary_window_minutes: Some(120),
                    secondary_resets_at: None,
                    credits_json: None,
                    captured_at: now + idx as i64,
                })
                .expect("insert usage snapshot");
        }
    }

    assert_eq!(
        storage
            .account_count_active_available(None, None)
            .expect("count active available"),
        3
    );
    assert_eq!(
        storage
            .account_count_low_quota(None, None)
            .expect("count low quota"),
        2
    );

    let active_page = storage
        .list_accounts_active_available(None, None, Some((0, 2)))
        .expect("list active page");
    let active_ids = active_page
        .iter()
        .map(|account| account.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(active_ids, vec!["acc-active-1", "acc-low-1"]);

    let low_alpha = storage
        .list_accounts_low_quota(None, Some("alpha"), None)
        .expect("list low alpha");
    let low_alpha_ids = low_alpha
        .iter()
        .map(|account| account.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(low_alpha_ids, vec!["acc-low-1"]);
}

#[test]
fn storage_low_quota_filters_exclude_unavailable_accounts() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let now = now_ts();

    let accounts = [
        ("acc-low-active", "active", 88.0),
        ("acc-low-unavailable", "unavailable", 92.0),
    ];

    for (idx, (id, status, used_percent)) in accounts.iter().enumerate() {
        storage
            .insert_account(&Account {
                id: (*id).to_string(),
                label: (*id).to_string(),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: idx as i64,
                status: (*status).to_string(),
                created_at: now + idx as i64,
                updated_at: now + idx as i64,
            })
            .expect("insert account");

        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: (*id).to_string(),
                used_percent: Some(*used_percent),
                window_minutes: Some(300),
                resets_at: None,
                secondary_used_percent: Some(*used_percent),
                secondary_window_minutes: Some(120),
                secondary_resets_at: None,
                credits_json: None,
                captured_at: now + idx as i64,
            })
            .expect("insert usage snapshot");
    }

    assert_eq!(
        storage
            .account_count_low_quota(None, None)
            .expect("count low quota"),
        1
    );

    let low_quota_ids = storage
        .list_accounts_low_quota(None, None, None)
        .expect("list low quota")
        .into_iter()
        .map(|account| account.id)
        .collect::<Vec<_>>();
    assert_eq!(low_quota_ids, vec!["acc-low-active".to_string()]);
}

#[test]
fn storage_gateway_candidates_exclude_unavailable_or_missing_token_accounts() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let now = now_ts();

    let accounts = [
        ("acc-ready", "active", 0_i64),
        ("acc-no-snapshot", "active", 1_i64),
        ("acc-exhausted", "active", 2_i64),
        ("acc-partial", "active", 3_i64),
        ("acc-inactive", "inactive", 4_i64),
        ("acc-no-token", "active", 5_i64),
    ];
    for (id, status, sort) in accounts {
        storage
            .insert_account(&Account {
                id: id.to_string(),
                label: id.to_string(),
                issuer: "https://auth.openai.com".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort,
                status: status.to_string(),
                created_at: now + sort,
                updated_at: now + sort,
            })
            .expect("insert account");
    }

    for id in [
        "acc-ready",
        "acc-no-snapshot",
        "acc-exhausted",
        "acc-partial",
        "acc-inactive",
    ] {
        storage
            .insert_token(&Token {
                account_id: id.to_string(),
                id_token: format!("id-{id}"),
                access_token: format!("access-{id}"),
                refresh_token: format!("refresh-{id}"),
                api_key_access_token: None,
                last_refresh: now,
            })
            .expect("insert token");
    }

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-ready".to_string(),
            used_percent: Some(12.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert ready usage");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-exhausted".to_string(),
            used_percent: Some(100.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert exhausted usage");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-partial".to_string(),
            used_percent: Some(20.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: Some(10.0),
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert partial usage");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-inactive".to_string(),
            used_percent: Some(10.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert inactive usage");

    let candidates = storage
        .list_gateway_candidates()
        .expect("list gateway candidates");
    let candidate_ids = candidates
        .iter()
        .map(|(account, _)| account.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(candidate_ids, vec!["acc-ready", "acc-no-snapshot"]);
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

#[test]
fn gateway_candidates_respect_quota_protection_threshold() {
    let _env = override_gateway_quota_env("1", "10");
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let now = now_ts();

    for (id, used) in [("acc-safe", 89.0_f64), ("acc-guarded", 90.0_f64)] {
        storage
            .insert_account(&Account {
                id: id.to_string(),
                label: id.to_string(),
                issuer: "issuer".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: 0,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            })
            .expect("insert account");
        storage
            .insert_token(&Token {
                account_id: id.to_string(),
                id_token: "id".to_string(),
                access_token: "access".to_string(),
                refresh_token: "refresh".to_string(),
                api_key_access_token: None,
                last_refresh: now,
            })
            .expect("insert token");
        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: id.to_string(),
                used_percent: Some(used),
                window_minutes: Some(300),
                resets_at: None,
                secondary_used_percent: None,
                secondary_window_minutes: None,
                secondary_resets_at: None,
                credits_json: None,
                captured_at: now,
            })
            .expect("insert usage");
    }

    let candidates = storage
        .list_gateway_candidates()
        .expect("list gateway candidates");
    let candidate_ids = candidates
        .iter()
        .map(|(account, _)| account.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(candidate_ids, vec!["acc-safe"]);
}

#[test]
fn request_logs_support_prefixed_query_filters() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_request_log(&RequestLog {
            trace_id: Some("trc-alpha-extra".to_string()),
            key_id: Some("key-alpha-extra".to_string()),
            account_id: Some("acc-1".to_string()),
            initial_account_id: Some("acc-1".to_string()),
            attempted_account_ids_json: Some(r#"["acc-1"]"#.to_string()),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("weighted".to_string()),
            requested_model: None,
            model_fallback_path_json: None,
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/chat/completions".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-5.1".to_string()),
            reasoning_effort: Some("low".to_string()),
            response_adapter: Some("OpenAIChatCompletionsJson".to_string()),
            upstream_url: Some("https://chatgpt.com/backend-api/codex/v1/responses".to_string()),
            status_code: Some(201),
            duration_ms: Some(320),
            input_tokens: Some(11),
            cached_input_tokens: Some(3),
            output_tokens: Some(7),
            total_tokens: Some(18),
            reasoning_output_tokens: Some(2),
            estimated_cost_usd: Some(0.0),
            error: None,
            created_at: now_ts() - 2,
        })
        .expect("insert request log 0");

    storage
        .insert_request_log(&RequestLog {
            trace_id: Some("trc-alpha".to_string()),
            key_id: Some("key-alpha".to_string()),
            account_id: Some("acc-1".to_string()),
            initial_account_id: Some("acc-1".to_string()),
            attempted_account_ids_json: Some(r#"["acc-1"]"#.to_string()),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("least-latency".to_string()),
            requested_model: None,
            model_fallback_path_json: None,
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-5.1".to_string()),
            reasoning_effort: Some("low".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://chatgpt.com/backend-api/codex/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(210),
            input_tokens: Some(9),
            cached_input_tokens: Some(1),
            output_tokens: Some(5),
            total_tokens: Some(14),
            reasoning_output_tokens: Some(1),
            estimated_cost_usd: Some(0.0),
            error: None,
            created_at: now_ts() - 1,
        })
        .expect("insert request log 1");

    storage
        .insert_request_log(&RequestLog {
            trace_id: Some("trc-beta".to_string()),
            key_id: Some("key-beta".to_string()),
            account_id: Some("acc-2".to_string()),
            initial_account_id: Some("acc-2".to_string()),
            attempted_account_ids_json: Some(r#"["acc-2"]"#.to_string()),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("cost-first".to_string()),
            requested_model: None,
            model_fallback_path_json: None,
            request_path: "/v1/models".to_string(),
            original_path: Some("/v1/models".to_string()),
            adapted_path: Some("/v1/models".to_string()),
            method: "GET".to_string(),
            model: Some("gpt-4.1".to_string()),
            reasoning_effort: Some("xhigh".to_string()),
            response_adapter: None,
            upstream_url: Some("https://api.openai.com/v1/models".to_string()),
            status_code: Some(503),
            duration_ms: Some(1800),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            reasoning_output_tokens: None,
            estimated_cost_usd: Some(0.0),
            error: Some("upstream timeout".to_string()),
            created_at: now_ts(),
        })
        .expect("insert request log 2");

    let method_filtered = storage
        .list_request_logs(Some("method:GET"), 100)
        .expect("filter by method");
    assert_eq!(method_filtered.len(), 1);
    assert_eq!(method_filtered[0].method, "GET");

    let status_filtered = storage
        .list_request_logs(Some("status:5xx"), 100)
        .expect("filter by status range");
    assert_eq!(status_filtered.len(), 1);
    assert_eq!(status_filtered[0].status_code, Some(503));

    let key_filtered = storage
        .list_request_logs(Some("key:key-alpha"), 100)
        .expect("filter by key id");
    assert_eq!(key_filtered.len(), 2);

    let key_exact_filtered = storage
        .list_request_logs(Some("key:=key-alpha"), 100)
        .expect("filter by exact key id");
    assert_eq!(key_exact_filtered.len(), 1);
    assert_eq!(key_exact_filtered[0].key_id.as_deref(), Some("key-alpha"));

    let trace_filtered = storage
        .list_request_logs(Some("trace:=trc-alpha"), 100)
        .expect("filter by trace id");
    assert_eq!(trace_filtered.len(), 1);
    assert_eq!(trace_filtered[0].trace_id.as_deref(), Some("trc-alpha"));

    let original_path_filtered = storage
        .list_request_logs(Some("original:=/v1/chat/completions"), 100)
        .expect("filter by original path");
    assert_eq!(original_path_filtered.len(), 1);
    assert_eq!(
        original_path_filtered[0].original_path.as_deref(),
        Some("/v1/chat/completions")
    );

    let adapter_filtered = storage
        .list_request_logs(Some("adapter:=OpenAIChatCompletionsJson"), 100)
        .expect("filter by response adapter");
    assert_eq!(adapter_filtered.len(), 1);
    assert_eq!(
        adapter_filtered[0].response_adapter.as_deref(),
        Some("OpenAIChatCompletionsJson")
    );

    let strategy_filtered = storage
        .list_request_logs(Some("least-latency"), 100)
        .expect("filter by route strategy");
    assert_eq!(strategy_filtered.len(), 1);
    assert_eq!(
        strategy_filtered[0].route_strategy.as_deref(),
        Some("least-latency")
    );

    let fallback_filtered = storage
        .list_request_logs(Some("timeout"), 100)
        .expect("fallback fuzzy query");
    assert_eq!(fallback_filtered.len(), 1);
    assert_eq!(
        fallback_filtered[0].error.as_deref(),
        Some("upstream timeout")
    );
}

#[test]
fn request_log_today_summary_reads_from_token_stats_table() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let created_at = now_ts();
    let request_log_id = storage
        .insert_request_log(&RequestLog {
            trace_id: Some("trc-summary".to_string()),
            key_id: Some("key-summary".to_string()),
            account_id: Some("acc-summary".to_string()),
            initial_account_id: Some("acc-summary".to_string()),
            attempted_account_ids_json: Some(r#"["acc-summary"]"#.to_string()),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("weighted".to_string()),
            requested_model: None,
            model_fallback_path_json: None,
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-5.3-codex".to_string()),
            reasoning_effort: Some("high".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(1450),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            reasoning_output_tokens: None,
            estimated_cost_usd: None,
            error: None,
            created_at,
        })
        .expect("insert request log");

    storage
        .insert_request_token_stat(&RequestTokenStat {
            request_log_id,
            key_id: Some("key-summary".to_string()),
            account_id: Some("acc-summary".to_string()),
            model: Some("gpt-5.3-codex".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(80),
            output_tokens: Some(22),
            total_tokens: Some(142),
            reasoning_output_tokens: Some(9),
            estimated_cost_usd: Some(0.33),
            created_at,
        })
        .expect("insert token stat");

    let summary = storage
        .summarize_request_logs_between(created_at - 1, created_at + 1)
        .expect("summarize");
    assert_eq!(summary.input_tokens, 120);
    assert_eq!(summary.cached_input_tokens, 80);
    assert_eq!(summary.output_tokens, 22);
    assert_eq!(summary.reasoning_output_tokens, 9);
    assert!(summary.estimated_cost_usd > 0.32);
}

#[test]
fn insert_request_log_with_token_stat_writes_both_tables_in_one_call() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let created_at = now_ts();

    let (request_log_id, token_stat_error) = storage
        .insert_request_log_with_token_stat(
            &RequestLog {
                trace_id: Some("trc-atomic".to_string()),
                key_id: Some("key-atomic".to_string()),
                account_id: Some("acc-atomic".to_string()),
                initial_account_id: Some("acc-atomic".to_string()),
                attempted_account_ids_json: Some(r#"["acc-atomic"]"#.to_string()),
                candidate_count: None,
                attempted_count: None,
                skipped_count: None,
                skipped_cooldown_count: None,
                skipped_inflight_count: None,
                route_strategy: Some("balanced".to_string()),
                requested_model: None,
                model_fallback_path_json: None,
                request_path: "/v1/responses".to_string(),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some("gpt-5.3-codex".to_string()),
                reasoning_effort: Some("high".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
                status_code: Some(200),
                duration_ms: Some(980),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: None,
                created_at,
            },
            &RequestTokenStat {
                request_log_id: 0,
                key_id: Some("key-atomic".to_string()),
                account_id: Some("acc-atomic".to_string()),
                model: Some("gpt-5.3-codex".to_string()),
                input_tokens: Some(10),
                cached_input_tokens: Some(2),
                output_tokens: Some(5),
                total_tokens: Some(15),
                reasoning_output_tokens: Some(1),
                estimated_cost_usd: Some(0.01),
                created_at,
            },
        )
        .expect("insert request log with token stat");

    assert!(request_log_id > 0);
    assert!(
        token_stat_error.is_none(),
        "token stat insert should succeed: {:?}",
        token_stat_error
    );

    let logs = storage
        .list_request_logs(Some("key:=key-atomic"), 10)
        .expect("list logs");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].key_id.as_deref(), Some("key-atomic"));
    assert_eq!(logs[0].input_tokens, Some(10));
    assert_eq!(logs[0].cached_input_tokens, Some(2));
    assert_eq!(logs[0].output_tokens, Some(5));
    assert_eq!(logs[0].total_tokens, Some(15));
    assert_eq!(logs[0].reasoning_output_tokens, Some(1));
}

#[test]
fn clear_request_logs_keeps_token_stats_for_usage_summary() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let created_at = now_ts();
    let request_log_id = storage
        .insert_request_log(&RequestLog {
            trace_id: Some("trc-clear".to_string()),
            key_id: Some("key-clear".to_string()),
            account_id: Some("acc-clear".to_string()),
            initial_account_id: Some("acc-clear".to_string()),
            attempted_account_ids_json: Some(r#"["acc-clear"]"#.to_string()),
            candidate_count: None,
            attempted_count: None,
            skipped_count: None,
            skipped_cooldown_count: None,
            skipped_inflight_count: None,
            route_strategy: Some("ordered".to_string()),
            requested_model: None,
            model_fallback_path_json: None,
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-5.3-codex".to_string()),
            reasoning_effort: Some("high".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(760),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            reasoning_output_tokens: None,
            estimated_cost_usd: None,
            error: None,
            created_at,
        })
        .expect("insert request log");
    storage
        .insert_request_token_stat(&RequestTokenStat {
            request_log_id,
            key_id: Some("key-clear".to_string()),
            account_id: Some("acc-clear".to_string()),
            model: Some("gpt-5.3-codex".to_string()),
            input_tokens: Some(100),
            cached_input_tokens: Some(30),
            output_tokens: Some(20),
            total_tokens: Some(120),
            reasoning_output_tokens: Some(5),
            estimated_cost_usd: Some(0.12),
            created_at,
        })
        .expect("insert token stat");

    storage.clear_request_logs().expect("clear request logs");

    let logs = storage.list_request_logs(None, 100).expect("list logs");
    assert!(logs.is_empty(), "request logs should be cleared");

    let summary = storage
        .summarize_request_logs_between(created_at - 1, created_at + 1)
        .expect("summarize");
    assert_eq!(summary.input_tokens, 100);
    assert_eq!(summary.cached_input_tokens, 30);
    assert_eq!(summary.output_tokens, 20);
    assert_eq!(summary.reasoning_output_tokens, 5);
    assert!(summary.estimated_cost_usd > 0.11);
}

#[test]
fn request_token_stats_can_summarize_total_tokens_by_key() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let created_at = now_ts();

    for (request_log_id, key_id, total_tokens, input_tokens, cached_input_tokens, output_tokens) in [
        (101_i64, "gk_alpha", Some(120_i64), None, None, None),
        (
            102_i64,
            "gk_alpha",
            None,
            Some(90_i64),
            Some(30_i64),
            Some(25_i64),
        ),
        (103_i64, "gk_beta", Some(75_i64), None, None, None),
        (104_i64, "", Some(999_i64), None, None, None),
    ] {
        storage
            .insert_request_token_stat(&RequestTokenStat {
                request_log_id,
                key_id: if key_id.is_empty() {
                    None
                } else {
                    Some(key_id.to_string())
                },
                account_id: Some("acc-summary".to_string()),
                model: Some("gpt-5.3-codex".to_string()),
                input_tokens,
                cached_input_tokens,
                output_tokens,
                total_tokens,
                reasoning_output_tokens: Some(0),
                estimated_cost_usd: Some(0.0),
                created_at,
            })
            .expect("insert token stat");
    }

    let summary = storage
        .summarize_request_token_stats_by_key()
        .expect("summarize by key");

    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].key_id, "gk_alpha");
    assert_eq!(summary[0].total_tokens, 205);
    assert_eq!(summary[1].key_id, "gk_beta");
    assert_eq!(summary[1].total_tokens, 75);
}

#[test]
fn usage_snapshots_can_prune_history_per_account() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let now = now_ts();

    for offset in 0..5 {
        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: "acc-prune-1".to_string(),
                used_percent: Some(10.0 + offset as f64),
                window_minutes: Some(300),
                resets_at: None,
                secondary_used_percent: None,
                secondary_window_minutes: None,
                secondary_resets_at: None,
                credits_json: None,
                captured_at: now + offset,
            })
            .expect("insert acc-prune-1 snapshot");
    }

    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-prune-2".to_string(),
            used_percent: Some(30.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert acc-prune-2 snapshot");

    let deleted = storage
        .prune_usage_snapshots_for_account("acc-prune-1", 2)
        .expect("prune snapshots");
    assert_eq!(deleted, 3);

    let kept = storage
        .usage_snapshot_count_for_account("acc-prune-1")
        .expect("count kept");
    assert_eq!(kept, 2);

    let untouched = storage
        .usage_snapshot_count_for_account("acc-prune-2")
        .expect("count untouched");
    assert_eq!(untouched, 1);
}

#[test]
fn storage_api_keys_include_profile_fields() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");
    let expires_at = now_ts() + 3600;

    storage
        .insert_api_key(&ApiKey {
            id: "key-1".to_string(),
            name: Some("main".to_string()),
            model_slug: Some("claude-sonnet-4".to_string()),
            reasoning_effort: Some("medium".to_string()),
            client_type: "claude_code".to_string(),
            protocol_type: "anthropic_native".to_string(),
            auth_scheme: "x_api_key".to_string(),
            upstream_base_url: Some("https://api.anthropic.com".to_string()),
            static_headers_json: Some("{\"anthropic-version\":\"2023-06-01\"}".to_string()),
            key_hash: "hash-1".to_string(),
            status: "active".to_string(),
            created_at: now_ts(),
            last_used_at: None,
            expires_at: Some(expires_at),
        })
        .expect("insert key");

    let key = storage
        .list_api_keys()
        .expect("list keys")
        .into_iter()
        .find(|item| item.id == "key-1")
        .expect("key exists");
    assert_eq!(key.client_type, "claude_code");
    assert_eq!(key.protocol_type, "anthropic_native");
    assert_eq!(key.auth_scheme, "x_api_key");
    assert_eq!(key.model_slug.as_deref(), Some("claude-sonnet-4"));
    assert_eq!(key.expires_at, Some(expires_at));
}

#[test]
fn storage_can_roundtrip_api_key_secret() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_api_key(&ApiKey {
            id: "key-secret-1".to_string(),
            name: Some("secret".to_string()),
            model_slug: None,
            reasoning_effort: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: "hash-secret-1".to_string(),
            status: "active".to_string(),
            created_at: now_ts(),
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert key");

    storage
        .upsert_api_key_secret("key-secret-1", "sk-secret-value")
        .expect("upsert secret");

    let loaded = storage
        .find_api_key_secret_by_id("key-secret-1")
        .expect("load secret");
    assert_eq!(loaded.as_deref(), Some("sk-secret-value"));

    storage.delete_api_key("key-secret-1").expect("delete key");
    let removed = storage
        .find_api_key_secret_by_id("key-secret-1")
        .expect("load removed secret");
    assert!(removed.is_none());
}

#[test]
fn storage_can_roundtrip_api_key_rate_limit_config() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_api_key(&ApiKey {
            id: "key-rate-1".to_string(),
            name: Some("rate".to_string()),
            model_slug: None,
            reasoning_effort: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: "hash-rate-1".to_string(),
            status: "active".to_string(),
            created_at: now_ts(),
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert key");

    storage
        .upsert_api_key_rate_limit("key-rate-1", Some(10), Some(1000), Some(50))
        .expect("upsert rate limit");
    let config = storage
        .find_api_key_rate_limit_by_id("key-rate-1")
        .expect("load rate limit")
        .expect("rate limit exists");
    assert_eq!(config.key_id, "key-rate-1");
    assert_eq!(config.rpm, Some(10));
    assert_eq!(config.tpm, Some(1000));
    assert_eq!(config.daily_limit, Some(50));

    storage
        .upsert_api_key_rate_limit("key-rate-1", None, None, None)
        .expect("clear rate limit");
    let cleared = storage
        .find_api_key_rate_limit_by_id("key-rate-1")
        .expect("reload rate limit");
    assert!(cleared.is_none(), "rate limit row should be removed");
}

#[test]
fn storage_can_roundtrip_api_key_model_fallback_config() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_api_key(&ApiKey {
            id: "key-fallback-1".to_string(),
            name: Some("fallback".to_string()),
            model_slug: Some("o3".to_string()),
            reasoning_effort: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: "hash-fallback-1".to_string(),
            status: "active".to_string(),
            created_at: now_ts(),
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert key");

    storage
        .upsert_api_key_model_fallback(
            "key-fallback-1",
            &[
                "o3".to_string(),
                "o4-mini".to_string(),
                "gpt-4o".to_string(),
            ],
        )
        .expect("upsert model fallback");
    let config = storage
        .find_api_key_model_fallback_by_id("key-fallback-1")
        .expect("load model fallback")
        .expect("model fallback exists");
    assert_eq!(config.key_id, "key-fallback-1");
    assert_eq!(config.model_chain_json, r#"["o3","o4-mini","gpt-4o"]"#);

    storage
        .upsert_api_key_model_fallback("key-fallback-1", &[])
        .expect("clear model fallback");
    let cleared = storage
        .find_api_key_model_fallback_by_id("key-fallback-1")
        .expect("reload model fallback");
    assert!(cleared.is_none(), "model fallback row should be removed");
}

#[test]
fn storage_can_roundtrip_api_key_allowed_models_config() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_api_key(&ApiKey {
            id: "key-allowed-1".to_string(),
            name: Some("allowed".to_string()),
            model_slug: None,
            reasoning_effort: None,
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: "hash-allowed-1".to_string(),
            status: "active".to_string(),
            created_at: now_ts(),
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert key");

    storage
        .update_api_key_allowed_models("key-allowed-1", Some("[\"gpt-5\",\"o3\",\"gpt-5\"]"))
        .expect("save allowed models");
    let config = storage
        .find_api_key_allowed_models_by_id("key-allowed-1")
        .expect("load allowed models");
    assert_eq!(config.as_deref(), Some("[\"gpt-5\",\"o3\",\"gpt-5\"]"));

    storage
        .update_api_key_allowed_models("key-allowed-1", None)
        .expect("clear allowed models");
    let cleared = storage
        .find_api_key_allowed_models_by_id("key-allowed-1")
        .expect("reload allowed models");
    assert!(cleared.is_none(), "allowed models should be cleared");
}

#[test]
fn storage_can_roundtrip_alert_rules_channels_and_history() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let created_at = now_ts();
    storage
        .upsert_alert_rule(&AlertRule {
            id: "ar_usage_1".to_string(),
            name: "额度超限".to_string(),
            rule_type: "usage_threshold".to_string(),
            config_json: r#"{"thresholdPercent":90,"channelIds":["ac_webhook_1"]}"#.to_string(),
            enabled: true,
            created_at,
            updated_at: created_at,
        })
        .expect("upsert alert rule");
    storage
        .upsert_alert_channel(&AlertChannel {
            id: "ac_webhook_1".to_string(),
            name: "Webhook 通知".to_string(),
            channel_type: "webhook".to_string(),
            config_json: r#"{"url":"http://127.0.0.1:18081/hook"}"#.to_string(),
            enabled: true,
            created_at,
            updated_at: created_at,
        })
        .expect("upsert alert channel");

    let rule = storage
        .find_alert_rule_by_id("ar_usage_1")
        .expect("find alert rule")
        .expect("rule exists");
    assert_eq!(rule.rule_type, "usage_threshold");

    let channel = storage
        .find_alert_channel_by_id("ac_webhook_1")
        .expect("find alert channel")
        .expect("channel exists");
    assert_eq!(channel.channel_type, "webhook");

    let history_id = storage
        .insert_alert_history(
            Some("ar_usage_1"),
            Some("ac_webhook_1"),
            "test_success",
            "test delivered",
        )
        .expect("insert alert history");
    assert!(history_id > 0);

    let history = storage.list_alert_history(10).expect("list alert history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].rule_name.as_deref(), Some("额度超限"));
    assert_eq!(history[0].channel_name.as_deref(), Some("Webhook 通知"));

    storage
        .delete_alert_rule("ar_usage_1")
        .expect("delete alert rule");
    storage
        .delete_alert_channel("ac_webhook_1")
        .expect("delete alert channel");
    assert!(storage
        .find_alert_rule_by_id("ar_usage_1")
        .expect("reload rule")
        .is_none());
    assert!(storage
        .find_alert_channel_by_id("ac_webhook_1")
        .expect("reload channel")
        .is_none());
}

#[test]
fn storage_can_roundtrip_model_pricing_config() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .replace_model_pricing(&[
            ModelPricing {
                model_slug: "gpt-4o".to_string(),
                input_price_per_1k: 0.005,
                output_price_per_1k: 0.015,
                updated_at: now_ts(),
            },
            ModelPricing {
                model_slug: "o3".to_string(),
                input_price_per_1k: 0.02,
                output_price_per_1k: 0.08,
                updated_at: now_ts(),
            },
        ])
        .expect("replace model pricing");

    let items = storage.list_model_pricing().expect("list pricing");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].model_slug, "gpt-4o");
    assert_eq!(items[1].model_slug, "o3");
    assert_eq!(items[1].input_price_per_1k, 0.02);
    assert_eq!(items[1].output_price_per_1k, 0.08);

    storage
        .replace_model_pricing(&[])
        .expect("clear model pricing");
    let cleared = storage.list_model_pricing().expect("list cleared pricing");
    assert!(cleared.is_empty(), "model pricing rows should be removed");
}

#[test]
fn storage_can_roundtrip_plugin_registry_entries() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let created_at = now_ts();
    let plugin = PluginRecord {
        id: "plugin-lua-quota-guard".to_string(),
        name: "额度保护".to_string(),
        description: Some("请求前拦截高风险模型".to_string()),
        runtime: "lua".to_string(),
        hook_points_json: r#"["pre_route"]"#.to_string(),
        script_content: "return { allow = true }".to_string(),
        enabled: true,
        timeout_ms: 80,
        created_at,
        updated_at: created_at,
    };
    storage.upsert_plugin(&plugin).expect("insert plugin");

    let inserted = storage
        .find_plugin_by_id("plugin-lua-quota-guard")
        .expect("find inserted plugin")
        .expect("plugin exists");
    assert_eq!(inserted.name, "额度保护");
    assert_eq!(
        inserted.description.as_deref(),
        Some("请求前拦截高风险模型")
    );
    assert_eq!(inserted.hook_points_json, r#"["pre_route"]"#);
    assert_eq!(inserted.timeout_ms, 80);
    assert!(inserted.enabled);

    let updated = PluginRecord {
        id: plugin.id.clone(),
        name: "额度保护 v2".to_string(),
        description: Some("补充响应后审计".to_string()),
        runtime: "lua".to_string(),
        hook_points_json: r#"["pre_route","post_response"]"#.to_string(),
        script_content: "return { allow = false, reason = 'quota' }".to_string(),
        enabled: false,
        timeout_ms: 95,
        created_at,
        updated_at: created_at + 60,
    };
    storage.upsert_plugin(&updated).expect("update plugin");

    let items = storage.list_plugins().expect("list plugins");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "额度保护 v2");
    assert_eq!(items[0].description.as_deref(), Some("补充响应后审计"));
    assert_eq!(
        items[0].hook_points_json,
        r#"["pre_route","post_response"]"#
    );
    assert_eq!(
        items[0].script_content,
        "return { allow = false, reason = 'quota' }"
    );
    assert!(!items[0].enabled);
    assert_eq!(items[0].timeout_ms, 95);
    assert_eq!(items[0].created_at, created_at);

    storage
        .delete_plugin("plugin-lua-quota-guard")
        .expect("delete plugin");
    assert!(storage
        .find_plugin_by_id("plugin-lua-quota-guard")
        .expect("find deleted plugin")
        .is_none());
}

#[test]
fn storage_can_summarize_cost_usage_by_key_model_and_day() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    storage
        .insert_request_token_stat(&RequestTokenStat {
            request_log_id: 1,
            key_id: Some("key-a".to_string()),
            account_id: Some("acc-a".to_string()),
            model: Some("o3".to_string()),
            input_tokens: Some(100),
            cached_input_tokens: Some(20),
            output_tokens: Some(30),
            total_tokens: Some(110),
            reasoning_output_tokens: Some(5),
            estimated_cost_usd: Some(1.2),
            created_at: 1_700_000_000,
        })
        .expect("insert token stat 1");
    storage
        .insert_request_token_stat(&RequestTokenStat {
            request_log_id: 2,
            key_id: Some("key-b".to_string()),
            account_id: Some("acc-b".to_string()),
            model: Some("gpt-4o".to_string()),
            input_tokens: Some(50),
            cached_input_tokens: Some(0),
            output_tokens: Some(25),
            total_tokens: Some(75),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.4),
            created_at: 1_700_086_400,
        })
        .expect("insert token stat 2");

    let total = storage
        .summarize_cost_usage_between(1_699_999_000, 1_700_172_800)
        .expect("summarize total");
    assert_eq!(total.request_count, 2);
    assert_eq!(total.total_tokens, 185);
    assert!((total.estimated_cost_usd - 1.6).abs() < 0.0001);

    let by_key = storage
        .summarize_cost_usage_by_key_between(1_699_999_000, 1_700_172_800)
        .expect("summarize by key");
    assert_eq!(by_key.len(), 2);
    assert_eq!(by_key[0].key_id, "key-a");

    let by_model = storage
        .summarize_cost_usage_by_model_between(1_699_999_000, 1_700_172_800)
        .expect("summarize by model");
    assert_eq!(by_model.len(), 2);
    assert_eq!(by_model[0].model, "o3");

    let by_day = storage
        .summarize_cost_usage_by_day_between(1_699_999_000, 1_700_172_800)
        .expect("summarize by day");
    assert!(!by_day.is_empty());
    assert_eq!(by_day.iter().map(|item| item.request_count).sum::<i64>(), 2);
}

#[test]
fn storage_can_summarize_request_trends_models_and_heatmap() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    for (id, model, status_code, created_at) in [
        (1, "o3", 200, 1_700_000_000),
        (2, "o3", 500, 1_700_000_600),
        (3, "gpt-4o", 200, 1_700_086_400),
    ] {
        storage
            .insert_request_log(&RequestLog {
                trace_id: Some(format!("trend-{id}")),
                key_id: Some("gk-trend".to_string()),
                account_id: Some("acc-trend".to_string()),
                initial_account_id: Some("acc-trend".to_string()),
                attempted_account_ids_json: Some(r#"["acc-trend"]"#.to_string()),
                candidate_count: None,
                attempted_count: None,
                skipped_count: None,
                skipped_cooldown_count: None,
                skipped_inflight_count: None,
                route_strategy: Some("balanced".to_string()),
                requested_model: Some(model.to_string()),
                model_fallback_path_json: Some(format!(r#"["{model}"]"#)),
                request_path: "/v1/responses".to_string(),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some(model.to_string()),
                reasoning_effort: Some("medium".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
                status_code: Some(status_code),
                duration_ms: Some(120),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: if status_code >= 400 {
                    Some("upstream error".to_string())
                } else {
                    None
                },
                created_at,
            })
            .expect("insert request log");
    }

    let request_trends = storage
        .summarize_request_trends_between(1_699_999_000, 1_700_172_800, "day")
        .expect("request trends");
    assert!(!request_trends.is_empty());
    assert_eq!(
        request_trends
            .iter()
            .map(|item| item.request_count)
            .sum::<i64>(),
        3
    );
    assert_eq!(
        request_trends
            .iter()
            .map(|item| item.success_count)
            .sum::<i64>(),
        2
    );

    let model_trends = storage
        .summarize_request_model_trends_between(1_699_999_000, 1_700_172_800)
        .expect("model trends");
    assert_eq!(model_trends.len(), 2);
    assert_eq!(model_trends[0].model, "o3");
    assert_eq!(model_trends[0].request_count, 2);

    let heatmap = storage
        .summarize_request_heatmap_between(1_699_999_000, 1_700_172_800)
        .expect("heatmap");
    assert!(!heatmap.is_empty());
    assert_eq!(
        heatmap.iter().map(|item| item.request_count).sum::<i64>(),
        3
    );
}
