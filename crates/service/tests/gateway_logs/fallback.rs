use super::*;

#[test]
fn gateway_model_fallback_uses_next_models_and_logs_path() {
    let _lock = lock_env();
    let dir = new_test_dir("codexmanager-gateway-model-fallback");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let responses = vec![
        (
            500,
            serde_json::json!({
                "error": { "message": "model o3 unavailable", "type": "server_error" }
            })
            .to_string(),
        ),
        (
            500,
            serde_json::json!({
                "error": { "message": "model o4-mini unavailable", "type": "server_error" }
            })
            .to_string(),
        ),
        (
            200,
            serde_json::json!({
                "id": "resp_fallback_final",
                "model": "gpt-4o",
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "ok" }]
                }],
                "usage": { "input_tokens": 12, "output_tokens": 4, "total_tokens": 16 }
            })
            .to_string(),
        ),
    ];
    let (upstream_addr, upstream_rx, upstream_join) = start_mock_upstream_sequence(responses);
    let upstream_base = format!("http://{upstream_addr}/api.openai.com/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init db");
    let now = now_ts();

    storage
        .insert_account(&Account {
            id: "acc_model_fallback".to_string(),
            label: "model-fallback".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_model_fallback".to_string()),
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
            account_id: "acc_model_fallback".to_string(),
            id_token: String::new(),
            access_token: "access_token_model_fallback".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_model_fallback".to_string()),
            last_refresh: now,
        })
        .expect("insert token");

    let platform_key = "pk_model_fallback";
    storage
        .insert_api_key(&ApiKey {
            id: "gk_model_fallback".to_string(),
            name: Some("model-fallback".to_string()),
            model_slug: Some("o3".to_string()),
            reasoning_effort: Some("high".to_string()),
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
        .upsert_api_key_model_fallback(
            "gk_model_fallback",
            &[
                "o3".to_string(),
                "o4-mini".to_string(),
                "gpt-4o".to_string(),
            ],
        )
        .expect("upsert fallback chain");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let request_body = serde_json::json!({
        "model": "o3",
        "input": "hello",
        "stream": false
    });
    let request_body = serde_json::to_string(&request_body).expect("serialize request");
    let response = post_http_response(
        &server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();
    assert_eq!(response.status, 200, "gateway response: {}", response.body);
    assert_eq!(
        response
            .headers
            .get("x-codexmanager-actual-model")
            .map(String::as_str),
        Some("gpt-4o")
    );

    let captured_first = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive first upstream request");
    let captured_second = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive second upstream request");
    let captured_third = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive third upstream request");
    upstream_join.join().expect("join upstream");

    let first_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured_first))
            .expect("parse first upstream body");
    let second_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured_second))
            .expect("parse second upstream body");
    let third_body: serde_json::Value =
        serde_json::from_slice(&decode_upstream_request_body(&captured_third))
            .expect("parse third upstream body");
    assert_eq!(
        first_body.get("model").and_then(|value| value.as_str()),
        Some("o3")
    );
    assert_eq!(
        second_body.get("model").and_then(|value| value.as_str()),
        Some("o4-mini")
    );
    assert_eq!(
        third_body.get("model").and_then(|value| value.as_str()),
        Some("gpt-4o")
    );

    let mut matched = None;
    for _ in 0..40 {
        let logs = storage
            .list_request_logs(Some("key:=gk_model_fallback"), 20)
            .expect("list request logs");
        matched = logs
            .into_iter()
            .find(|item| item.request_path == "/v1/responses");
        if matched.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let log = matched.expect("model fallback request log");
    assert_eq!(log.status_code, Some(200));
    assert_eq!(log.model.as_deref(), Some("gpt-4o"));
    assert_eq!(log.requested_model.as_deref(), Some("o3"));
    assert_eq!(
        log.model_fallback_path_json.as_deref(),
        Some(r#"["o3","o4-mini","gpt-4o"]"#)
    );
}

#[test]
fn gateway_model_fallback_keeps_primary_model_when_first_attempt_succeeds() {
    let _lock = lock_env();
    let dir = new_test_dir("codexmanager-gateway-model-fallback-primary");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let responses = vec![(
        200,
        serde_json::json!({
            "id": "resp_primary_model",
            "model": "o3",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "ok" }]
            }],
            "usage": { "input_tokens": 10, "output_tokens": 3, "total_tokens": 13 }
        })
        .to_string(),
    )];
    let (upstream_addr, upstream_rx, upstream_join) = start_mock_upstream_sequence(responses);
    let upstream_base = format!("http://{upstream_addr}/api.openai.com/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init db");
    let now = now_ts();

    storage
        .insert_account(&Account {
            id: "acc_model_primary".to_string(),
            label: "model-primary".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_model_primary".to_string()),
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
            account_id: "acc_model_primary".to_string(),
            id_token: String::new(),
            access_token: "access_token_model_primary".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_model_primary".to_string()),
            last_refresh: now,
        })
        .expect("insert token");

    let platform_key = "pk_model_primary";
    storage
        .insert_api_key(&ApiKey {
            id: "gk_model_primary".to_string(),
            name: Some("model-primary".to_string()),
            model_slug: Some("o3".to_string()),
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
        .upsert_api_key_model_fallback(
            "gk_model_primary",
            &[
                "o3".to_string(),
                "o4-mini".to_string(),
                "gpt-4o".to_string(),
            ],
        )
        .expect("upsert fallback chain");

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let request_body = serde_json::json!({
        "model": "o3",
        "input": "hello",
        "stream": false
    });
    let request_body = serde_json::to_string(&request_body).expect("serialize request");
    let response = post_http_response(
        &server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();

    assert_eq!(response.status, 200, "gateway response: {}", response.body);
    assert_eq!(response.headers.get("x-codexmanager-actual-model"), None);

    let captured = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive upstream request");
    assert!(
        matches!(
            upstream_rx.recv_timeout(Duration::from_millis(300)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ),
        "unexpected second upstream request"
    );
    upstream_join.join().expect("join upstream");

    let body: serde_json::Value = serde_json::from_slice(&decode_upstream_request_body(&captured))
        .expect("parse upstream body");
    assert_eq!(
        body.get("model").and_then(|value| value.as_str()),
        Some("o3")
    );

    let mut matched = None;
    for _ in 0..40 {
        let logs = storage
            .list_request_logs(Some("key:=gk_model_primary"), 20)
            .expect("list request logs");
        matched = logs
            .into_iter()
            .find(|item| item.request_path == "/v1/responses");
        if matched.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let log = matched.expect("primary model request log");
    assert_eq!(log.status_code, Some(200));
    assert_eq!(log.model.as_deref(), Some("o3"));
    assert_eq!(log.requested_model.as_deref(), Some("o3"));
    assert_eq!(log.model_fallback_path_json, None);
}

#[test]
fn gateway_model_fallback_is_disabled_when_key_has_no_chain() {
    let _lock = lock_env();
    let dir = new_test_dir("codexmanager-gateway-model-fallback-disabled");
    let db_path: PathBuf = dir.join("codexmanager.db");

    let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.to_string_lossy().as_ref());
    let _no_proxy_guard = EnvGuard::set("NO_PROXY", "127.0.0.1,localhost");
    let _no_proxy_lower_guard = EnvGuard::set("no_proxy", "127.0.0.1,localhost");

    let responses = vec![
        (
            500,
            serde_json::json!({
                "error": { "message": "model o3 unavailable", "type": "server_error" }
            })
            .to_string(),
        ),
        (
            200,
            serde_json::json!({
                "id": "resp_should_not_be_used",
                "model": "o4-mini",
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "unexpected" }]
                }],
                "usage": { "input_tokens": 9, "output_tokens": 2, "total_tokens": 11 }
            })
            .to_string(),
        ),
    ];
    let (upstream_addr, upstream_rx, upstream_join) = start_mock_upstream_sequence(responses);
    let upstream_base = format!("http://{upstream_addr}/api.openai.com/v1");
    let _upstream_guard = EnvGuard::set("CODEXMANAGER_UPSTREAM_BASE_URL", &upstream_base);

    let storage = Storage::open(&db_path).expect("open db");
    storage.init().expect("init db");
    let now = now_ts();

    storage
        .insert_account(&Account {
            id: "acc_model_no_chain".to_string(),
            label: "model-no-chain".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("chatgpt_model_no_chain".to_string()),
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
            account_id: "acc_model_no_chain".to_string(),
            id_token: String::new(),
            access_token: "access_token_model_no_chain".to_string(),
            refresh_token: String::new(),
            api_key_access_token: Some("api_access_token_model_no_chain".to_string()),
            last_refresh: now,
        })
        .expect("insert token");

    let platform_key = "pk_model_no_chain";
    storage
        .insert_api_key(&ApiKey {
            id: "gk_model_no_chain".to_string(),
            name: Some("model-no-chain".to_string()),
            model_slug: Some("o3".to_string()),
            reasoning_effort: Some("high".to_string()),
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

    let server = codexmanager_service::start_one_shot_server().expect("start server");
    let request_body = serde_json::json!({
        "model": "o3",
        "input": "hello",
        "stream": false
    });
    let request_body = serde_json::to_string(&request_body).expect("serialize request");
    let response = post_http_response(
        &server.addr,
        "/v1/responses",
        &request_body,
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {platform_key}")),
        ],
    );
    server.join();

    assert_eq!(response.status, 500, "gateway response: {}", response.body);
    assert_eq!(response.headers.get("x-codexmanager-actual-model"), None);

    let captured = upstream_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("receive upstream request");
    assert!(
        matches!(
            upstream_rx.recv_timeout(Duration::from_millis(300)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ),
        "unexpected fallback upstream request"
    );
    upstream_join.join().expect("join upstream");

    let body: serde_json::Value = serde_json::from_slice(&decode_upstream_request_body(&captured))
        .expect("parse upstream body");
    assert_eq!(
        body.get("model").and_then(|value| value.as_str()),
        Some("o3")
    );

    let mut matched = None;
    for _ in 0..40 {
        let logs = storage
            .list_request_logs(Some("key:=gk_model_no_chain"), 20)
            .expect("list request logs");
        matched = logs
            .into_iter()
            .find(|item| item.request_path == "/v1/responses");
        if matched.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let log = matched.expect("no-chain request log");
    assert_eq!(log.status_code, Some(500));
    assert_eq!(log.model.as_deref(), Some("o3"));
    assert_eq!(log.requested_model.as_deref(), Some("o3"));
    assert_eq!(log.model_fallback_path_json, None);
}
