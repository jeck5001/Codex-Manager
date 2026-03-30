use crate::storage_helpers::open_storage;

use super::runtime::run_plugin_task;

pub(crate) fn run_due_tasks_once() {
    let Some(storage) = open_storage() else {
        return;
    };
    let now = codexmanager_core::storage::now_ts();
    let tasks = match storage.list_due_plugin_tasks(now, 100) {
        Ok(items) => items,
        Err(err) => {
            log::warn!("list due plugin tasks failed: {err}");
            return;
        }
    };
    for task in tasks {
        let _ = run_plugin_task(&task.id, None);
    }
}
