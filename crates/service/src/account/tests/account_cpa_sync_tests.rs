use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use tiny_http::{Header, Response, Server, StatusCode};

static TEST_DB_SEQ: AtomicUsize = AtomicUsize::new(0);

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
fn cpa_download_falls_back_to_filename_query_param_on_404() {
    let _guard = setup_test_storage();
    let server = Server::http("127.0.0.1:0").expect("start fallback server");
    let api_url = format!("http://{}", server.server_addr());
    let handle = thread::spawn(move || {
        let first = server.recv().expect("receive first request");
        let first_url = first.url().to_string();
        let first_response = Response::from_string("404 page not found")
            .with_status_code(StatusCode(404))
            .with_header(
                Header::from_bytes("Content-Type", "text/plain").expect("content-type header"),
            );
        first.respond(first_response).expect("respond first request");

        let second = server.recv().expect("receive second request");
        let second_url = second.url().to_string();
        let second_response = Response::from_string(r#"{"access_token":"a","id_token":"i","account_id":"acc-fallback"}"#)
            .with_status_code(StatusCode(200))
            .with_header(
                Header::from_bytes("Content-Type", "application/json")
                    .expect("content-type header"),
            );
        second.respond(second_response).expect("respond second request");

        (first_url, second_url)
    });

    let content = super::download_auth_file_for_test(
        &api_url,
        "key-fallback",
        serde_json::json!({
            "files": [
                { "name": "demo.json", "source": "file" }
            ]
        }),
    )
    .expect("download auth file");

    let (first_url, second_url) = handle.join().expect("join fallback server");
    assert!(first_url.contains("/v0/management/auth-files/download?name=demo.json"));
    assert!(second_url.contains("/v0/management/auth-files/download?filename=demo.json"));
    assert!(content.contains("\"acc-fallback\""));
}

#[test]
fn cpa_download_prefers_canonical_name_endpoint_over_broken_path_field() {
    let _guard = setup_test_storage();
    let server = Server::http("127.0.0.1:0").expect("start canonical server");
    let api_url = format!("http://{}", server.server_addr());
    let handle = thread::spawn(move || {
        let request = server.recv().expect("receive canonical request");
        let request_url = request.url().to_string();
        let response = Response::from_string(
            r#"{"access_token":"a","id_token":"i","account_id":"acc-canonical"}"#,
        )
        .with_status_code(StatusCode(200))
        .with_header(
            Header::from_bytes("Content-Type", "application/json").expect("content-type header"),
        );
        request.respond(response).expect("respond canonical request");
        request_url
    });

    let content = super::download_auth_file_for_test(
        &api_url,
        "key-canonical",
        serde_json::json!({
            "files": [
                {
                    "name": "demo.json",
                    "source": "file",
                    "path": "/wrong/internal/path"
                }
            ]
        }),
    )
    .expect("download auth file");

    let request_url = handle.join().expect("join canonical server");
    assert!(request_url.contains("/v0/management/auth-files/download?name=demo.json"));
    assert!(content.contains("\"acc-canonical\""));
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
fn cpa_auth_files_extracts_source_and_runtime_only_flags() {
    let files = super::auth_file_flags_for_test(serde_json::json!({
        "files": [
            { "name": "disk.json", "source": "file", "runtime_only": false },
            { "name": "memory.json", "source": "memory", "runtimeOnly": true }
        ]
    }))
    .expect("parse file flags");

    assert_eq!(
        files,
        vec![
            ("disk.json".to_string(), Some("file".to_string()), false),
            ("memory.json".to_string(), Some("memory".to_string()), true),
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

#[test]
fn cpa_sync_status_defaults_to_disabled_snapshot() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    super::reset_cpa_sync_runtime_for_test();
    let status = super::cpa_sync_status_for_test();

    assert_eq!(status.status, "disabled");
    assert!(!status.is_running);
    assert_eq!(status.interval_minutes, 30);
}

#[test]
fn cpa_sync_run_guard_rejects_overlapping_runs() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    super::reset_cpa_sync_runtime_for_test();
    let _guard = super::begin_cpa_sync_run_for_test("manual").expect("first lock");
    let err = super::begin_cpa_sync_run_for_test("scheduled")
        .expect_err("second run should fail");

    assert!(err.contains("正在执行中"));
}

#[test]
fn cpa_schedule_status_marks_misconfigured_when_enabled_without_credentials() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    super::reset_cpa_sync_runtime_for_test();

    super::refresh_cpa_sync_schedule_for_test(Some(false), true, 15, "", false);
    let status = super::cpa_sync_status_for_test();

    assert_eq!(status.status, "misconfigured");
    assert_eq!(status.last_error, "CPA API URL 或 Management Key 未配置");
}

#[test]
fn cpa_schedule_status_sets_next_run_when_enabled_and_configured() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    super::reset_cpa_sync_runtime_for_test();

    super::refresh_cpa_sync_schedule_for_test(
        Some(true),
        true,
        15,
        "https://cpa.example.com",
        true,
    );
    let status = super::cpa_sync_status_for_test();

    assert_eq!(status.status, "idle");
    assert_eq!(status.interval_minutes, 15);
    assert!(status.next_run_at.is_some());
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
        "codexmanager_cpa_sync_test_{}_{}_{}.db",
        std::process::id(),
        stamp,
        TEST_DB_SEQ.fetch_add(1, Ordering::Relaxed)
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
