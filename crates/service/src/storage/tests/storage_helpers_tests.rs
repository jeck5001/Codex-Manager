use super::{
    clear_storage_cache_for_tests, clear_storage_open_count_for_tests, open_storage_at_path,
    storage_open_count_for_tests,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// 函数 `unique_db_path`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - prefix: 参数 prefix
///
/// # 返回
/// 返回函数执行结果
fn unique_db_path(prefix: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{prefix}-{nonce}.db"))
        .to_string_lossy()
        .to_string()
}

/// 函数 `open_storage_reuses_cached_connection_in_same_thread`
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
fn open_storage_reuses_cached_connection_in_same_thread() {
    let db_path = unique_db_path("codexmanager-open-storage-reuse");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);

    let storage = open_storage_at_path(&db_path).expect("open storage 1");
    storage.init().expect("init");
    drop(storage);

    let storage = open_storage_at_path(&db_path).expect("open storage 2");
    drop(storage);

    assert_eq!(storage_open_count_for_tests(&db_path), 1);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);
    let _ = std::fs::remove_file(&db_path);
}

/// 函数 `open_storage_reopens_when_db_path_changes`
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
fn open_storage_reopens_when_db_path_changes() {
    let db_path_1 = unique_db_path("codexmanager-open-storage-path-1");
    let db_path_2 = unique_db_path("codexmanager-open-storage-path-2");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path_1);
    clear_storage_open_count_for_tests(&db_path_2);

    let storage = open_storage_at_path(&db_path_1).expect("open storage path 1");
    storage.init().expect("init 1");
    drop(storage);

    let storage = open_storage_at_path(&db_path_2).expect("open storage path 2");
    storage.init().expect("init 2");
    drop(storage);

    assert_eq!(storage_open_count_for_tests(&db_path_1), 1);
    assert_eq!(storage_open_count_for_tests(&db_path_2), 1);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path_1);
    clear_storage_open_count_for_tests(&db_path_2);
    let _ = std::fs::remove_file(&db_path_1);
    let _ = std::fs::remove_file(&db_path_2);
}
