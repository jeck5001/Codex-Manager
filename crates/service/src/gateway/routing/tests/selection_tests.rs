use super::{clear_candidate_cache_for_tests, collect_gateway_candidates, CANDIDATE_CACHE_TTL_ENV};
use codexmanager_core::storage::{now_ts, Account, Storage, Token, UsageSnapshotRecord};
use serde_json::json;
use std::sync::Mutex;

static CANDIDATE_CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn candidate_snapshot_cache_reuses_recent_snapshot() {
    let _guard = CANDIDATE_CACHE_TEST_LOCK.lock().expect("lock");
    let previous_ttl = std::env::var(CANDIDATE_CACHE_TTL_ENV).ok();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var(CANDIDATE_CACHE_TTL_ENV, "2000");
    std::env::set_var("CODEXMANAGER_DB_PATH", "selection-cache-test-1");
    super::reload_from_env();
    clear_candidate_cache_for_tests();

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    storage
        .insert_account(&Account {
            id: "acc-cache-1".to_string(),
            label: "cached".to_string(),
            issuer: "issuer".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now_ts(),
            updated_at: now_ts(),
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "acc-cache-1".to_string(),
            id_token: "id".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            api_key_access_token: None,
            last_refresh: now_ts(),
        })
        .expect("insert token");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-cache-1".to_string(),
            used_percent: Some(10.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now_ts(),
        })
        .expect("insert snapshot");

    let first = collect_gateway_candidates(&storage).expect("first candidates");
    assert_eq!(first.len(), 1);

    storage
        .update_account_status("acc-cache-1", "inactive")
        .expect("mark inactive");
    let second = collect_gateway_candidates(&storage).expect("second candidates");
    assert_eq!(second.len(), 1);

    clear_candidate_cache_for_tests();
    if let Some(value) = previous_ttl {
        std::env::set_var(CANDIDATE_CACHE_TTL_ENV, value);
    } else {
        std::env::remove_var(CANDIDATE_CACHE_TTL_ENV);
    }
    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    super::reload_from_env();
}

#[test]
fn candidates_follow_account_sort_order() {
    let _guard = CANDIDATE_CACHE_TEST_LOCK.lock().expect("lock");
    let previous_ttl = std::env::var(CANDIDATE_CACHE_TTL_ENV).ok();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var(CANDIDATE_CACHE_TTL_ENV, "0");
    std::env::set_var("CODEXMANAGER_DB_PATH", "selection-cache-test-2");
    super::reload_from_env();
    clear_candidate_cache_for_tests();

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    let now = now_ts();
    let accounts = vec![
        ("acc-sort-10", 10_i64),
        ("acc-sort-0", 0_i64),
        ("acc-sort-1", 1_i64),
    ];
    for (id, sort) in &accounts {
        storage
            .insert_account(&Account {
                id: (*id).to_string(),
                label: (*id).to_string(),
                issuer: "issuer".to_string(),
                chatgpt_account_id: None,
                workspace_id: None,
                group_name: None,
                sort: *sort,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            })
            .expect("insert account");
        storage
            .insert_token(&Token {
                account_id: (*id).to_string(),
                id_token: "id".to_string(),
                access_token: "access".to_string(),
                refresh_token: "refresh".to_string(),
                api_key_access_token: None,
                last_refresh: now,
            })
            .expect("insert token");
        storage
            .insert_usage_snapshot(&UsageSnapshotRecord {
                account_id: (*id).to_string(),
                used_percent: Some(10.0),
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

    let candidates = collect_gateway_candidates(&storage).expect("collect candidates");
    let ordered_ids = candidates
        .iter()
        .map(|(account, _)| account.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ordered_ids, vec!["acc-sort-0", "acc-sort-1", "acc-sort-10"]);

    clear_candidate_cache_for_tests();
    if let Some(value) = previous_ttl {
        std::env::set_var(CANDIDATE_CACHE_TTL_ENV, value);
    } else {
        std::env::remove_var(CANDIDATE_CACHE_TTL_ENV);
    }
    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    super::reload_from_env();
}

#[test]
fn usage_snapshot_write_invalidates_cached_candidates() {
    let _guard = CANDIDATE_CACHE_TEST_LOCK.lock().expect("lock");
    let previous_ttl = std::env::var(CANDIDATE_CACHE_TTL_ENV).ok();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    let previous_enabled =
        std::env::var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED).ok();
    let previous_threshold =
        std::env::var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT)
            .ok();
    std::env::set_var(CANDIDATE_CACHE_TTL_ENV, "2000");
    std::env::set_var("CODEXMANAGER_DB_PATH", "selection-cache-test-3");
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
        "1",
    );
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        "20",
    );
    super::reload_from_env();
    clear_candidate_cache_for_tests();

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-cache-quota".to_string(),
            label: "cached".to_string(),
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
            account_id: "acc-cache-quota".to_string(),
            id_token: "id".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-cache-quota".to_string(),
            used_percent: Some(70.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert snapshot");

    let first = collect_gateway_candidates(&storage).expect("first candidates");
    assert_eq!(first.len(), 1);

    crate::usage_snapshot_store::store_usage_snapshot(
        &storage,
        "acc-cache-quota",
        json!({
            "rate_limit": {
                "primary_window": {
                    "used_percent": 95.0,
                    "limit_window_seconds": 18_000
                }
            }
        }),
    )
    .expect("store updated snapshot");

    let second = collect_gateway_candidates(&storage).expect("second candidates");
    assert!(second.is_empty());

    clear_candidate_cache_for_tests();
    if let Some(value) = previous_ttl {
        std::env::set_var(CANDIDATE_CACHE_TTL_ENV, value);
    } else {
        std::env::remove_var(CANDIDATE_CACHE_TTL_ENV);
    }
    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    if let Some(value) = previous_enabled {
        std::env::set_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
            value,
        );
    } else {
        std::env::remove_var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED);
    }
    if let Some(value) = previous_threshold {
        std::env::set_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
            value,
        );
    } else {
        std::env::remove_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        );
    }
    super::reload_from_env();
}

#[test]
fn quota_protection_setting_change_invalidates_cached_candidates() {
    let _guard = CANDIDATE_CACHE_TEST_LOCK.lock().expect("lock");
    let previous_ttl = std::env::var(CANDIDATE_CACHE_TTL_ENV).ok();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    let previous_enabled =
        std::env::var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED).ok();
    let previous_threshold =
        std::env::var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT)
            .ok();
    std::env::set_var(CANDIDATE_CACHE_TTL_ENV, "2000");
    std::env::set_var("CODEXMANAGER_DB_PATH", "selection-cache-test-4");
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
        "0",
    );
    std::env::set_var(
        crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        "20",
    );
    super::reload_from_env();
    clear_candidate_cache_for_tests();

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-setting-quota".to_string(),
            label: "cached".to_string(),
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
            account_id: "acc-setting-quota".to_string(),
            id_token: "id".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");
    storage
        .insert_usage_snapshot(&UsageSnapshotRecord {
            account_id: "acc-setting-quota".to_string(),
            used_percent: Some(95.0),
            window_minutes: Some(300),
            resets_at: None,
            secondary_used_percent: None,
            secondary_window_minutes: None,
            secondary_resets_at: None,
            credits_json: None,
            captured_at: now,
        })
        .expect("insert snapshot");

    let first = collect_gateway_candidates(&storage).expect("first candidates");
    assert_eq!(first.len(), 1);

    crate::app_settings::set_gateway_quota_protection_enabled(true).expect("enable protection");

    let second = collect_gateway_candidates(&storage).expect("second candidates");
    assert!(second.is_empty());

    clear_candidate_cache_for_tests();
    if let Some(value) = previous_ttl {
        std::env::set_var(CANDIDATE_CACHE_TTL_ENV, value);
    } else {
        std::env::remove_var(CANDIDATE_CACHE_TTL_ENV);
    }
    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    if let Some(value) = previous_enabled {
        std::env::set_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED,
            value,
        );
    } else {
        std::env::remove_var(crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_ENABLED);
    }
    if let Some(value) = previous_threshold {
        std::env::set_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
            value,
        );
    } else {
        std::env::remove_var(
            crate::account_availability::ENV_GATEWAY_QUOTA_PROTECTION_THRESHOLD_PERCENT,
        );
    }
    super::reload_from_env();
}
