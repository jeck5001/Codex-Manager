use super::*;

#[test]
fn gateway_response_cache_hits_second_non_stream_request() {
    let _lock = lock_env();
    let dir = new_test_dir("codexmanager-gateway-response-cache");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let _cache_enabled_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_ENABLED", "1");
    let _cache_ttl_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_TTL_SECS", "3600");
    let _cache_entries_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_MAX_ENTRIES", "8");
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let response_body = serde_json::json!({
        "id": "resp_cache_hit_1",
        "model": "gpt-5.3-codex",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "cache-ok" }]
        }],
        "usage": { "input_tokens": 14, "output_tokens": 5, "total_tokens": 19 }
    })
    .to_string();
    let (upstream_addr, upstream_rx, upstream_join) = start_mock_upstream_once(&response_body);
    let upstream_base = format!("http://{upstream_addr}/api.openai.com/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init db");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc_response_cache".to_string(),
            label: "response-cache".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_response_cache".to_string()),
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
            account_id: "acc_response_cache".to_string(),
            id_token: String::new(),
            access_token: "access_token_response_cache".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_response_cache".to_string()),
            last_refresh: now,
        })
        .expect("insert token");

    let platform_key = "pk_response_cache";
    storage
        .insert_api_key(&ApiKey {
            id: "gk_response_cache".to_string(),
            name: Some("response-cache".to_string()),
            model_slug: Some("gpt-5.3-codex".to_string()),
            reasoning_effort: Some("medium".to_string()),
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert api key");
    storage
        .upsert_api_key_response_cache_config("gk_response_cache", true)
        .expect("enable api key response cache");

    let request_body = serde_json::json!({
        "model": "gpt-5.3-codex",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": format!("cache-{}-{}", now, dir.to_string_lossy())
            }]
        }],
        "stream": false
    });
    let request_body = serde_json::to_string(&request_body).expect("serialize request");

    let first_server = codexmanager_service::start_one_shot_server().expect("start first server");
    let first_response = post_http_response(
        &first_server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    first_server.join();
    assert_eq!(first_response.status, 200, "{}", first_response.body);
    assert_eq!(
        first_response
            .headers
            .get("x-codexmanager-cache")
            .map(String::as_str),
        Some("miss")
    );

    let captured = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive upstream request");
    upstream_join.join().expect("join upstream");
    let captured_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured))
            .expect("parse upstream body");
    assert_eq!(
        captured_body.get("model").and_then(|value| value.as_str()),
        Some("gpt-5.3-codex")
    );

    let second_server = codexmanager_service::start_one_shot_server().expect("start second server");
    let second_response = post_http_response(
        &second_server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    second_server.join();
    assert_eq!(second_response.status, 200, "{}", second_response.body);
    assert_eq!(
        second_response
            .headers
            .get("x-codexmanager-cache")
            .map(String::as_str),
        Some("hit")
    );
    assert_eq!(second_response.body, first_response.body);
}

#[test]
fn gateway_response_cache_skips_requests_when_api_key_cache_disabled() {
    let _lock = lock_env();
    let dir = new_test_dir("codexmanager-gateway-response-cache-disabled");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let _cache_enabled_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_ENABLED", "1");
    let _cache_ttl_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_TTL_SECS", "3600");
    let _cache_entries_guard = EnvGuard::set("CODEXMANAGER_RESPONSE_CACHE_MAX_ENTRIES", "8");
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let response_body = serde_json::json!({
        "id": "resp_cache_disabled_1",
        "model": "gpt-5.3-codex",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "cache-disabled" }]
        }],
        "usage": { "input_tokens": 14, "output_tokens": 5, "total_tokens": 19 }
    })
    .to_string();
    let (upstream_addr, upstream_rx, upstream_join) = start_mock_upstream_sequence(vec![
        (200, response_body.clone()),
        (200, response_body.clone()),
    ]);
    let upstream_base = format!("http://{upstream_addr}/api.openai.com/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init db");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc_response_cache_disabled".to_string(),
            label: "response-cache-disabled".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_response_cache_disabled".to_string()),
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
            account_id: "acc_response_cache_disabled".to_string(),
            id_token: String::new(),
            access_token: "access_token_response_cache_disabled".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_response_cache_disabled".to_string()),
            last_refresh: now,
        })
        .expect("insert token");

    let platform_key = "pk_response_cache_disabled";
    storage
        .insert_api_key(&ApiKey {
            id: "gk_response_cache_disabled".to_string(),
            name: Some("response-cache-disabled".to_string()),
            model_slug: Some("gpt-5.3-codex".to_string()),
            reasoning_effort: Some("medium".to_string()),
            client_type: "codex".to_string(),
            protocol_type: "openai_compat".to_string(),
            auth_scheme: "authorization_bearer".to_string(),
            upstream_base_url: None,
            static_headers_json: None,
            key_hash: hash_platform_key_for_test(platform_key),
            status: "active".to_string(),
            created_at: now,
            last_used_at: None,
            expires_at: None,
        })
        .expect("insert api key");
    storage
        .upsert_api_key_response_cache_config("gk_response_cache_disabled", false)
        .expect("disable api key response cache");

    let request_body = serde_json::json!({
        "model": "gpt-5.3-codex",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": format!("cache-disabled-{}-{}", now, dir.to_string_lossy())
            }]
        }],
        "stream": false
    });
    let request_body = serde_json::to_string(&request_body).expect("serialize request");

    let first_server = codexmanager_service::start_one_shot_server().expect("start first server");
    let first_response = post_http_response(
        &first_server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    first_server.join();
    assert_eq!(first_response.status, 200, "{}", first_response.body);
    assert!(
        first_response.headers.get("x-codexmanager-cache").is_none(),
        "disabled key should not emit cache header on first response"
    );

    let second_server = codexmanager_service::start_one_shot_server().expect("start second server");
    let second_response = post_http_response(
        &second_server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    second_server.join();
    assert_eq!(second_response.status, 200, "{}", second_response.body);
    assert!(
        second_response
            .headers
            .get("x-codexmanager-cache")
            .is_none(),
        "disabled key should bypass cache on second response"
    );

    let captured_first = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive first upstream request");
    let captured_second = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive second upstream request");
    upstream_join.join().expect("join upstream");

    let first_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured_first))
            .expect("parse first upstream body");
    let second_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured_second))
            .expect("parse second upstream body");
    assert_eq!(
        first_body.get("model").and_then(|value| value.as_str()),
        Some("gpt-5.3-codex")
    );
    assert_eq!(
        second_body.get("model").and_then(|value| value.as_str()),
        Some("gpt-5.3-codex")
    );
}
