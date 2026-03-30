use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

mod catalog;
mod runtime;
mod scheduler;
mod store;

static PLUGIN_SCHEDULER_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) fn ensure_plugin_scheduler() {
    PLUGIN_SCHEDULER_STARTED.get_or_init(|| {
        catalog::sync_builtin_cleanup_task_schedule();
        let _ = thread::Builder::new()
            .name("plugin-scheduler".to_string())
            .spawn(plugin_scheduler_loop);
    });
}

pub(crate) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "plugin/catalog/list" | "plugin/catalog/refresh" => Some(catalog::handle_catalog_list(req)),
        "plugin/install" => Some(catalog::handle_install(req)),
        "plugin/update" => Some(catalog::handle_update(req)),
        "plugin/uninstall" => Some(catalog::handle_uninstall(req)),
        "plugin/list" => Some(store::handle_list_installed(req)),
        "plugin/enable" => Some(store::handle_enable(req, true)),
        "plugin/disable" => Some(store::handle_enable(req, false)),
        "plugin/tasks/update" => Some(store::handle_task_update(req)),
        "plugin/tasks/list" => Some(store::handle_task_list(req)),
        "plugin/tasks/run" => Some(runtime::handle_task_run(req)),
        "plugin/logs/list" => Some(store::handle_log_list(req)),
        _ => None,
    }?;
    Some(result)
}

pub(crate) fn json_response(req: &JsonRpcRequest, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        id: req.id.clone(),
        result,
    }
}

fn plugin_scheduler_loop() {
    loop {
        let sleep_secs = scheduler::run_due_tasks_once();
        thread::sleep(Duration::from_secs(sleep_secs));
    }
}
