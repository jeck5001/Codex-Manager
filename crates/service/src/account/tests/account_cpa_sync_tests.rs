use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use tiny_http::{Header, Response, Server, StatusCode};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn cpa_test_connection_rejects_missing_url() {
    let _guard = setup_test_storage();

    let err = super::test_cpa_connection(Some(&serde_json::json!({
        "managementKey": "key-1"
    })))
    .expect_err("missing url should fail");

    assert!(err.contains("CPA API URL 未配置"));
}

#[test]
fn cpa_test_connection_rejects_missing_management_key() {
    let _guard = setup_test_storage();

    let err = super::test_cpa_connection(Some(&serde_json::json!({
        "apiUrl": "https://cpa.example.com"
    })))
    .expect_err("missing key should fail");

    assert!(err.contains("CPA Management Key 未配置"));
}

#[test]
fn cpa_test_connection_reports_unauthorized_key() {
    let _guard = setup_test_storage();
    let (api_url, headers_rx, handle) = start_mock_cpa_server(
        401,
        r#"{"message":"bad management key"}"#.to_string(),
        "application/json",
    );

    let err = super::test_cpa_connection(Some(&serde_json::json!({
        "apiUrl": api_url,
        "managementKey": "key-401"
    })))
    .expect_err("401 should fail");

    let headers = headers_rx.recv().expect("receive headers");
    handle.join().expect("join mock server");

    assert!(headers.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("authorization") && value == "Bearer key-401"
    }));
    assert!(headers.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("x-management-key") && value == "key-401"
    }));
    assert!(err.contains("无效") || err.contains("没有权限"));
}

#[test]
fn cpa_test_connection_rejects_invalid_json_response() {
    let _guard = setup_test_storage();
    let (api_url, _headers_rx, handle) =
        start_mock_cpa_server(200, "not-json".to_string(), "text/plain");

    let err = super::test_cpa_connection(Some(&serde_json::json!({
        "apiUrl": api_url,
        "managementKey": "key-ok"
    })))
    .expect_err("invalid json should fail");

    handle.join().expect("join mock server");
    assert!(err.contains("invalid CPA auth-files response"));
}

#[test]
fn cpa_settings_use_saved_key_sentinel_falls_back_to_saved_value() {
    let _guard = setup_test_storage();
    crate::app_settings_set(Some(&serde_json::json!({
        "cpaSyncApiUrl": "https://saved.example.com/root/",
        "cpaSyncManagementKey": "saved-key"
    })))
    .expect("seed settings");

    let (api_url, management_key) = super::resolve_cpa_settings_for_test(Some(&serde_json::json!({
        "apiUrl": "https://override.example.com/base/",
        "managementKey": "use_saved_key"
    })))
    .expect("resolve settings");

    assert_eq!(api_url, "https://override.example.com/base");
    assert_eq!(management_key, "saved-key");
}

#[test]
fn cpa_auth_files_accepts_nested_files_array() {
    let names = super::auth_files_from_test_payload(serde_json::json!({
        "data": {
            "files": [
                { "name": "openai-main.json" },
                { "filename": "codex-backup.json" }
            ]
        }
    }))
    .expect("parse files");

    assert_eq!(
        names,
        vec![
            "openai-main.json".to_string(),
            "codex-backup.json".to_string()
        ]
    );
}

#[test]
fn cpa_filter_skips_non_target_payload_without_metadata_match() {
    let count = super::filter_import_items_for_test(r#"{"access_token":"abc123"}"#, false)
        .expect("filter");
    assert_eq!(count, 0);
}

#[test]
fn cpa_filter_accepts_target_payload_when_metadata_matches() {
    let count =
        super::filter_import_items_for_test(r#"{"access_token":"abc123"}"#, true).expect("filter");
    assert_eq!(count, 1);
}

#[test]
fn cpa_sync_reuses_account_import_pipeline() {
    let _guard = setup_test_storage();

    let payloads = vec![
        r#"{"access_token":"a","id_token":"i","refresh_token":"r","account_id":"acc-test-1"}"#
            .to_string(),
    ];
    let result = super::import_cpa_payloads_for_test(payloads).expect("import");
    assert_eq!(result.created, 1);
    assert_eq!(result.failed, 0);
}

fn setup_test_storage() -> EnvGuard {
    let lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "codexmanager_cpa_sync_test_{}_{}.db",
        std::process::id(),
        stamp
    ));
    let value = path.to_string_lossy().to_string();
    let previous = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var("CODEXMANAGER_DB_PATH", &value);
    crate::storage_helpers::clear_storage_cache_for_tests();
    let storage = codexmanager_core::storage::Storage::open(
        std::env::var("CODEXMANAGER_DB_PATH")
            .expect("db path should be present for CPA sync tests"),
    )
    .expect("open storage");
    storage.init().expect("init storage");
    EnvGuard {
        key: "CODEXMANAGER_DB_PATH",
        previous,
        _lock: Some(lock),
    }
}

struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
    _lock: Option<MutexGuard<'static, ()>>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        crate::storage_helpers::clear_storage_cache_for_tests();
        if let Some(value) = self.previous.take() {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn start_mock_cpa_server(
    status: u16,
    response_body: String,
    content_type: &str,
) -> (
    String,
    std::sync::mpsc::Receiver<Vec<(String, String)>>,
    thread::JoinHandle<()>,
) {
    let server = Server::http("127.0.0.1:0").expect("start mock cpa server");
    let addr = format!("http://{}", server.server_addr());
    let content_type = content_type.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let request = server.recv().expect("receive cpa request");
        let headers = request
            .headers()
            .iter()
            .map(|header| {
                (
                    header.field.as_str().to_string(),
                    header.value.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        tx.send(headers).expect("send cpa headers");
        let response = Response::from_string(response_body)
            .with_status_code(StatusCode(status))
            .with_header(
                Header::from_bytes("Content-Type", content_type.as_bytes())
                    .expect("content-type header"),
            );
        request.respond(response).expect("respond cpa request");
    });
    (addr, rx, handle)
}
