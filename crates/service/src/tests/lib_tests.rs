use super::*;
use codexmanager_core::storage::{now_ts, Storage};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn new_test_db_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "{prefix}-{}-{}-{}.db",
        std::process::id(),
        now_ts(),
        TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    path
}

#[test]
fn login_complete_requires_params() {
    let req = JsonRpcRequest {
        id: 1,
        method: "account/login/complete".to_string(),
        params: None,
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));

    let req = JsonRpcRequest {
        id: 2,
        method: "account/login/complete".to_string(),
        params: Some(serde_json::json!({ "code": "x" })),
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));

    let req = JsonRpcRequest {
        id: 3,
        method: "account/login/complete".to_string(),
        params: Some(serde_json::json!({ "state": "y" })),
    };
    let resp = handle_request(req);
    let err = resp
        .result
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(err.contains("missing"));
}

#[test]
fn healthcheck_config_rpc_supports_get_and_set() {
    crate::usage_refresh::clear_session_probe_state_for_tests();
    let db_path = new_test_db_path("healthcheck-config-rpc");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let initial_resp = handle_request(JsonRpcRequest {
        id: 4,
        method: "healthcheck/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        initial_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 5,
        method: "healthcheck/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": true,
            "intervalSecs": 1800,
            "sampleSize": 4,
        })),
    });
    assert_eq!(
        set_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_resp
            .result
            .get("intervalSecs")
            .and_then(|value| value.as_u64()),
        Some(1800)
    );
    assert_eq!(
        set_resp
            .result
            .get("sampleSize")
            .and_then(|value| value.as_u64()),
        Some(4)
    );

    let restore_resp = handle_request(JsonRpcRequest {
        id: 6,
        method: "healthcheck/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": false,
            "intervalSecs": 300,
            "sampleSize": 2,
        })),
    });
    assert_eq!(
        restore_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let _ = fs::remove_file(db_path);
    crate::usage_refresh::clear_session_probe_state_for_tests();
}

#[test]
fn healthcheck_run_rpc_returns_empty_summary_without_probe_candidates() {
    crate::usage_refresh::clear_session_probe_state_for_tests();
    let db_path = new_test_db_path("healthcheck-run-rpc");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let run_resp = handle_request(JsonRpcRequest {
        id: 7,
        method: "healthcheck/run".to_string(),
        params: None,
    });
    assert_eq!(
        run_resp
            .result
            .get("sampledAccounts")
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert_eq!(
        run_resp
            .result
            .get("failureCount")
            .and_then(|value| value.as_i64()),
        Some(0)
    );
    assert!(run_resp
        .result
        .get("startedAt")
        .and_then(|value| value.as_i64())
        .is_some());

    let config_resp = handle_request(JsonRpcRequest {
        id: 8,
        method: "healthcheck/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        config_resp
            .result
            .get("recentRun")
            .and_then(|value| value.get("sampledAccounts"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );

    let _ = fs::remove_file(db_path);
    crate::usage_refresh::clear_session_probe_state_for_tests();
}

#[test]
fn apikey_rpc_supports_expires_at_and_renew() {
    let db_path = new_test_db_path("apikey-rpc-expires-at");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let expires_at = now_ts() + 3600;
    let create_resp = handle_request(JsonRpcRequest {
        id: 10,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "临时分享",
            "expiresAt": expires_at,
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let list_resp = handle_request(JsonRpcRequest {
        id: 11,
        method: "apikey/list".to_string(),
        params: None,
    });
    let created = list_resp
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(|value| value.as_str()) == Some(key_id.as_str())
            })
        })
        .expect("created key in list");
    assert_eq!(
        created.get("expiresAt").and_then(|value| value.as_i64()),
        Some(expires_at)
    );
    assert_eq!(
        created.get("status").and_then(|value| value.as_str()),
        Some("active")
    );

    storage
        .update_api_key_status(&key_id, "expired")
        .expect("mark key expired");

    let renewed_expires_at = now_ts() + 7200;
    let renew_resp = handle_request(JsonRpcRequest {
        id: 12,
        method: "apikey/renew".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "expiresAt": renewed_expires_at,
        })),
    });
    assert_eq!(
        renew_resp
            .result
            .get("ok")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let list_after_renew = handle_request(JsonRpcRequest {
        id: 13,
        method: "apikey/list".to_string(),
        params: None,
    });
    let renewed = list_after_renew
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(|value| value.as_str()) == Some(key_id.as_str())
            })
        })
        .expect("renewed key in list");
    assert_eq!(
        renewed.get("expiresAt").and_then(|value| value.as_i64()),
        Some(renewed_expires_at)
    );
    assert_eq!(
        renewed.get("status").and_then(|value| value.as_str()),
        Some("active")
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn apikey_rpc_supports_rate_limit_get_and_set() {
    let db_path = new_test_db_path("apikey-rpc-rate-limit");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let create_resp = handle_request(JsonRpcRequest {
        id: 20,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "限流测试",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let set_resp = handle_request(JsonRpcRequest {
        id: 21,
        method: "apikey/rateLimit/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "rpm": 10,
            "tpm": 1000,
            "dailyLimit": 50,
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 22,
        method: "apikey/rateLimit/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        get_resp.result.get("rpm").and_then(|value| value.as_i64()),
        Some(10)
    );
    assert_eq!(
        get_resp.result.get("tpm").and_then(|value| value.as_i64()),
        Some(1000)
    );
    assert_eq!(
        get_resp
            .result
            .get("dailyLimit")
            .and_then(|value| value.as_i64()),
        Some(50)
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn apikey_response_cache_rpc_supports_get_and_set() {
    let db_path = new_test_db_path("apikey-rpc-response-cache");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let create_resp = handle_request(JsonRpcRequest {
        id: 23,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "缓存测试",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let initial_get_resp = handle_request(JsonRpcRequest {
        id: 24,
        method: "apikey/responseCache/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        initial_get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 25,
        method: "apikey/responseCache/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "enabled": true,
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let enabled_get_resp = handle_request(JsonRpcRequest {
        id: 26,
        method: "apikey/responseCache/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        enabled_get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn gateway_cache_rpc_supports_get_set_stats_and_clear() {
    let db_path = new_test_db_path("gateway-cache-rpc");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    crate::gateway::clear_response_cache();
    crate::gateway::set_response_cache_enabled(false);

    let get_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "gateway/cache/config/get".to_string(),
        params: None,
    });
    assert_eq!(
        get_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let set_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "gateway/cache/config/set".to_string(),
        params: Some(serde_json::json!({
            "enabled": true,
            "ttlSecs": 120,
            "maxEntries": 12,
        })),
    });
    assert_eq!(
        set_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        set_resp
            .result
            .get("ttlSecs")
            .and_then(|value| value.as_u64()),
        Some(120)
    );
    assert_eq!(
        set_resp
            .result
            .get("maxEntries")
            .and_then(|value| value.as_u64()),
        Some(12)
    );

    let stats_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "gateway/cache/stats".to_string(),
        params: None,
    });
    assert_eq!(
        stats_resp
            .result
            .get("enabled")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        stats_resp
            .result
            .get("entryCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );

    let clear_resp = handle_request(JsonRpcRequest {
        id: 33,
        method: "gateway/cache/clear".to_string(),
        params: None,
    });
    assert_eq!(
        clear_resp
            .result
            .get("entryCount")
            .and_then(|value| value.as_u64()),
        Some(0)
    );

    crate::gateway::set_response_cache_enabled(false);
    crate::gateway::clear_response_cache();
    let _ = fs::remove_file(db_path);
}

#[test]
fn stats_cost_model_pricing_rpc_supports_get_and_set() {
    let db_path = new_test_db_path("stats-cost-model-pricing");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let set_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "stats/cost/modelPricing/set".to_string(),
        params: Some(serde_json::json!({
            "items": [
                {
                    "modelSlug": "o3",
                    "inputPricePer1k": 0.02,
                    "outputPricePer1k": 0.08
                },
                {
                    "modelSlug": "gpt-4o",
                    "inputPricePer1k": 0.005,
                    "outputPricePer1k": 0.015
                }
            ]
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "stats/cost/modelPricing/get".to_string(),
        params: None,
    });
    let items = get_resp
        .result
        .get("items")
        .and_then(|value| value.as_array())
        .expect("pricing items");
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0].get("modelSlug").and_then(|value| value.as_str()),
        Some("gpt-4o")
    );
    assert_eq!(
        items[1]
            .get("outputPricePer1k")
            .and_then(|value| value.as_f64()),
        Some(0.08)
    );

    let clear_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "stats/cost/modelPricing/set".to_string(),
        params: Some(serde_json::json!({ "items": [] })),
    });
    assert_eq!(
        clear_resp
            .result
            .get("ok")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    let cleared = handle_request(JsonRpcRequest {
        id: 33,
        method: "stats/cost/modelPricing/get".to_string(),
        params: None,
    });
    assert_eq!(
        cleared
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(0)
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn stats_cost_summary_rpc_aggregates_custom_range() {
    let db_path = new_test_db_path("stats-cost-summary");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 1,
            key_id: Some("key-a".to_string()),
            account_id: Some("acc-a".to_string()),
            model: Some("o3".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(20),
            output_tokens: Some(40),
            total_tokens: Some(140),
            reasoning_output_tokens: Some(6),
            estimated_cost_usd: Some(1.5),
            created_at: 1_700_000_000,
        })
        .expect("insert stat 1");
    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 2,
            key_id: Some("key-b".to_string()),
            account_id: Some("acc-b".to_string()),
            model: Some("gpt-4o".to_string()),
            input_tokens: Some(80),
            cached_input_tokens: Some(10),
            output_tokens: Some(30),
            total_tokens: Some(100),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.8),
            created_at: 1_700_086_400,
        })
        .expect("insert stat 2");

    let resp = handle_request(JsonRpcRequest {
        id: 40,
        method: "stats/cost/summary".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });

    assert_eq!(
        resp.result
            .get("total")
            .and_then(|value| value.get("requestCount"))
            .and_then(|value| value.as_i64()),
        Some(2)
    );
    assert_eq!(
        resp.result
            .get("byKey")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(
        resp.result
            .get("byModel")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.get("model"))
            .and_then(|value| value.as_str()),
        Some("o3")
    );
    assert_eq!(
        resp.result.get("preset").and_then(|value| value.as_str()),
        Some("custom")
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn stats_cost_export_rpc_returns_csv_content() {
    let db_path = new_test_db_path("stats-cost-export");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    storage
        .insert_request_token_stat(&codexmanager_core::storage::RequestTokenStat {
            request_log_id: 1,
            key_id: Some("key-export".to_string()),
            account_id: Some("acc-export".to_string()),
            model: Some("o3".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(20),
            output_tokens: Some(40),
            total_tokens: Some(140),
            reasoning_output_tokens: Some(6),
            estimated_cost_usd: Some(1.5),
            created_at: 1_700_000_000,
        })
        .expect("insert export stat");

    let resp = handle_request(JsonRpcRequest {
        id: 41,
        method: "stats/cost/export".to_string(),
        params: Some(serde_json::json!({
            "preset": "custom",
            "startTs": 1_699_999_000_i64,
            "endTs": 1_700_172_800_i64,
        })),
    });

    assert_eq!(
        resp.result.get("fileName").and_then(|value| value.as_str()),
        Some("codexmanager-costs-custom.csv")
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("csv content");
    assert!(content.contains("section,dimension,dimensionValue"));
    assert!(content.contains("byKey,keyId,key-export"));

    let _ = fs::remove_file(db_path);
}

#[test]
fn requestlog_export_rpc_returns_filtered_csv_content() {
    let db_path = new_test_db_path("requestlog-export");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-1".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-2".to_string()),
            key_id: Some("gk-export".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(502),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(10),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.05),
            error: Some("upstream error".to_string()),
            created_at: 1_700_000_100,
        })
        .expect("insert request log");

    let resp = handle_request(JsonRpcRequest {
        id: 42,
        method: "requestlog/export".to_string(),
        params: Some(serde_json::json!({
            "format": "csv",
            "query": "status:502",
            "statusFilter": "5xx",
        })),
    });

    assert_eq!(
        resp.result.get("fileName").and_then(|value| value.as_str()),
        Some("codexmanager-requestlogs-5xx.csv")
    );
    assert_eq!(
        resp.result
            .get("recordCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("csv content");
    assert!(content.contains("traceId,keyId,accountId"));
    assert!(content.contains("trc-export-2"));
    assert!(!content.contains("trc-export-1"));

    let _ = fs::remove_file(db_path);
}

#[test]
fn requestlog_export_rpc_supports_key_model_and_time_filters() {
    let db_path = new_test_db_path("requestlog-export-extended-filters");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-a".to_string()),
            key_id: Some("gk-export-a".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log a");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-export-b".to_string()),
            key_id: Some("gk-export-b".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("gpt-4o".to_string()),
            model_fallback_path_json: Some(r#"["gpt-4o"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-4o".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(6),
            total_tokens: Some(16),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.08),
            error: None,
            created_at: 1_700_000_200,
        })
        .expect("insert request log b");

    let resp = handle_request(JsonRpcRequest {
        id: 43,
        method: "requestlog/export".to_string(),
        params: Some(serde_json::json!({
            "format": "json",
            "keyId": "gk-export-b",
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });

    assert_eq!(
        resp.result
            .get("recordCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    let content = resp
        .result
        .get("content")
        .and_then(|value| value.as_str())
        .expect("json content");
    assert!(content.contains("trc-export-b"));
    assert!(!content.contains("trc-export-a"));

    let _ = fs::remove_file(db_path);
}

#[test]
fn requestlog_list_and_summary_support_extended_filters() {
    let db_path = new_test_db_path("requestlog-list-summary-extended-filters");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-list-a".to_string()),
            key_id: Some("gk-list-a".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path_json: Some(r#"["o3"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(200),
            duration_ms: Some(120),
            input_tokens: Some(20),
            cached_input_tokens: Some(2),
            output_tokens: Some(4),
            total_tokens: Some(24),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1_700_000_000,
        })
        .expect("insert request log a");
    storage
        .insert_request_log(&codexmanager_core::storage::RequestLog {
            trace_id: Some("trc-list-b".to_string()),
            key_id: Some("gk-list-b".to_string()),
            account_id: Some("acc-export".to_string()),
            initial_account_id: Some("acc-export".to_string()),
            attempted_account_ids_json: Some(r#"["acc-export"]"#.to_string()),
            route_strategy: Some("balanced".to_string()),
            requested_model: Some("gpt-4o".to_string()),
            model_fallback_path_json: Some(r#"["gpt-4o"]"#.to_string()),
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/responses".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-4o".to_string()),
            reasoning_effort: Some("medium".to_string()),
            response_adapter: Some("Passthrough".to_string()),
            upstream_url: Some("https://api.openai.com/v1/responses".to_string()),
            status_code: Some(502),
            duration_ms: Some(220),
            input_tokens: Some(10),
            cached_input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(10),
            reasoning_output_tokens: Some(0),
            estimated_cost_usd: Some(0.05),
            error: Some("upstream error".to_string()),
            created_at: 1_700_000_200,
        })
        .expect("insert request log b");

    let list_resp = handle_request(JsonRpcRequest {
        id: 44,
        method: "requestlog/list".to_string(),
        params: Some(serde_json::json!({
            "page": 1,
            "pageSize": 20,
            "statusFilter": "5xx",
            "keyId": "gk-list-b",
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });
    assert_eq!(
        list_resp
            .result
            .get("total")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        list_resp
            .result
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(1)
    );

    let summary_resp = handle_request(JsonRpcRequest {
        id: 45,
        method: "requestlog/summary".to_string(),
        params: Some(serde_json::json!({
            "statusFilter": "5xx",
            "keyId": "gk-list-b",
            "model": "gpt-4o",
            "timeFrom": 1_700_000_150_i64,
            "timeTo": 1_700_000_250_i64,
        })),
    });
    assert_eq!(
        summary_resp
            .result
            .get("totalCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        summary_resp
            .result
            .get("filteredCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(
        summary_resp
            .result
            .get("errorCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn apikey_rpc_supports_model_fallback_get_and_set() {
    let db_path = new_test_db_path("apikey-rpc-model-fallback");
    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init schema");

    let create_resp = handle_request(JsonRpcRequest {
        id: 30,
        method: "apikey/create".to_string(),
        params: Some(serde_json::json!({
            "name": "降级测试",
            "modelSlug": "o3",
        })),
    });
    let key_id = create_resp
        .result
        .get("id")
        .and_then(|value| value.as_str())
        .expect("created key id")
        .to_string();

    let set_resp = handle_request(JsonRpcRequest {
        id: 31,
        method: "apikey/modelFallback/set".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
            "modelChain": ["o3", "o4-mini", "gpt-4o"],
        })),
    });
    assert_eq!(
        set_resp.result.get("ok").and_then(|value| value.as_bool()),
        Some(true)
    );

    let get_resp = handle_request(JsonRpcRequest {
        id: 32,
        method: "apikey/modelFallback/get".to_string(),
        params: Some(serde_json::json!({
            "id": key_id,
        })),
    });
    assert_eq!(
        get_resp
            .result
            .get("modelChain")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            "o3".to_string(),
            "o4-mini".to_string(),
            "gpt-4o".to_string()
        ])
    );

    let _ = fs::remove_file(db_path);
}
