use super::{
    import_register_task, read_register_task, seed_completed_local_register_task_for_test,
};
use codexmanager_core::storage::Storage;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvRestore(Option<OsString>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            std::env::set_var("CODEXMANAGER_DB_PATH", value);
        } else {
            std::env::remove_var("CODEXMANAGER_DB_PATH");
        }
    }
}

fn override_db_path() -> EnvRestore {
    let previous = std::env::var_os("CODEXMANAGER_DB_PATH");
    let path = std::env::temp_dir().join(format!(
        "codexmanager-register-task-test-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::env::set_var("CODEXMANAGER_DB_PATH", &path);
    let storage = Storage::open(&path).expect("open test db");
    storage.init().expect("init test db");
    EnvRestore(previous)
}

#[test]
fn read_register_task_returns_local_runtime_snapshot() {
    let task = seed_completed_local_register_task_for_test("user@example.com");
    let snapshot = read_register_task(&task.task_uuid).expect("task snapshot");

    assert_eq!(snapshot.status(), "completed");
    assert_eq!(snapshot.email(), Some("user@example.com"));
    assert!(snapshot.can_import());
}

#[test]
fn import_register_task_uses_local_payload_and_marks_imported() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _db = override_db_path();
    let task = seed_completed_local_register_task_for_test("import@example.com");
    let imported = import_register_task(&task.task_uuid).expect("import task");
    assert!(
        imported.get("created").is_some() || imported.get("updated").is_some(),
        "unexpected import result: {imported}"
    );
}
