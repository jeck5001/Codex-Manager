use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub(crate) struct LocalRegisterTaskInput {
    pub email_service_type: String,
    pub register_mode: String,
    pub proxy: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalRegisterBatchInput {
    pub email_service_type: String,
    pub register_mode: String,
    pub proxy: Option<String>,
    pub count: usize,
    pub interval_min: i64,
    pub interval_max: i64,
    pub concurrency: usize,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRegisterTaskSnapshot {
    pub task_uuid: String,
    pub batch_id: Option<String>,
    pub status: String,
    pub email_service_type: String,
    pub register_mode: String,
    pub proxy: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub failure_code: Option<String>,
    pub error_message: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub result: Value,
    #[serde(skip_serializing)]
    pub payload: Option<String>,
    pub imported_account_id: Option<String>,
    pub is_imported: bool,
    pub canceled: bool,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRegisterBatchSnapshot {
    pub batch_id: String,
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub success: usize,
    pub failed: usize,
    pub current_index: usize,
    pub task_uuids: Vec<String>,
    pub cancelled: bool,
    pub finished: bool,
    pub progress: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone)]
struct LocalRegisterBatchRecord {
    batch_id: String,
    email_service_type: String,
    register_mode: String,
    proxy: Option<String>,
    total: usize,
    interval_min: i64,
    interval_max: i64,
    concurrency: usize,
    mode: String,
    task_uuids: Vec<String>,
    cancelled: bool,
    logs: Vec<String>,
}

#[derive(Default)]
struct LocalRegisterRuntime {
    tasks: HashMap<String, LocalRegisterTaskSnapshot>,
    batches: HashMap<String, LocalRegisterBatchRecord>,
    seq: u64,
    batch_seq: u64,
}

static LOCAL_REGISTER_RUNTIME: OnceLock<Mutex<LocalRegisterRuntime>> = OnceLock::new();

fn runtime() -> &'static Mutex<LocalRegisterRuntime> {
    LOCAL_REGISTER_RUNTIME.get_or_init(|| Mutex::new(LocalRegisterRuntime::default()))
}

pub(crate) fn create_local_register_task(
    input: LocalRegisterTaskInput,
) -> LocalRegisterTaskSnapshot {
    create_local_register_task_with_batch(input, None)
}

fn create_local_register_task_with_batch(
    input: LocalRegisterTaskInput,
    batch_id: Option<String>,
) -> LocalRegisterTaskSnapshot {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    runtime.seq += 1;
    let snapshot = LocalRegisterTaskSnapshot {
        task_uuid: format!("reg-{}", runtime.seq),
        batch_id,
        status: "queued".to_string(),
        email_service_type: input.email_service_type,
        register_mode: input.register_mode,
        proxy: input.proxy,
        created_at: Some(now_rfc3339()),
        started_at: None,
        completed_at: None,
        failure_code: None,
        error_message: None,
        email: None,
        result: Value::Null,
        payload: None,
        imported_account_id: None,
        is_imported: false,
        canceled: false,
        logs: Vec::new(),
    };
    runtime
        .tasks
        .insert(snapshot.task_uuid.clone(), snapshot.clone());
    snapshot
}

pub(crate) fn create_local_register_batch(
    input: LocalRegisterBatchInput,
) -> Result<LocalRegisterBatchSnapshot, String> {
    if input.count == 0 {
        return Err("count must be greater than 0".to_string());
    }
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    runtime.batch_seq += 1;
    let batch_id = format!("batch-{}", runtime.batch_seq);
    let mut task_uuids = Vec::with_capacity(input.count);
    for _ in 0..input.count {
        runtime.seq += 1;
        let task_uuid = format!("reg-{}", runtime.seq);
        let snapshot = LocalRegisterTaskSnapshot {
            task_uuid: task_uuid.clone(),
            batch_id: Some(batch_id.clone()),
            status: "queued".to_string(),
            email_service_type: input.email_service_type.clone(),
            register_mode: input.register_mode.clone(),
            proxy: input.proxy.clone(),
            created_at: Some(now_rfc3339()),
            started_at: None,
            completed_at: None,
            failure_code: None,
            error_message: None,
            email: None,
            result: Value::Null,
            payload: None,
            imported_account_id: None,
            is_imported: false,
            canceled: false,
            logs: Vec::new(),
        };
        runtime.tasks.insert(task_uuid.clone(), snapshot);
        task_uuids.push(task_uuid);
    }
    let record = LocalRegisterBatchRecord {
        batch_id: batch_id.clone(),
        email_service_type: input.email_service_type,
        register_mode: input.register_mode,
        proxy: input.proxy,
        total: input.count,
        interval_min: input.interval_min,
        interval_max: input.interval_max,
        concurrency: input.concurrency,
        mode: input.mode,
        task_uuids: task_uuids.clone(),
        cancelled: false,
        logs: vec!["batch created locally".to_string()],
    };
    runtime.batches.insert(batch_id.clone(), record);
    Ok(build_batch_snapshot(
        runtime.batches.get(&batch_id).expect("batch record"),
        &runtime.tasks,
    ))
}

pub(crate) fn read_local_register_task(task_uuid: &str) -> Option<LocalRegisterTaskSnapshot> {
    let runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    runtime.tasks.get(task_uuid).cloned()
}

pub(crate) fn list_local_register_tasks() -> Vec<LocalRegisterTaskSnapshot> {
    let runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut tasks = runtime.tasks.values().cloned().collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.task_uuid.cmp(&right.task_uuid));
    tasks
}

pub(crate) fn read_local_register_batch(batch_id: &str) -> Option<LocalRegisterBatchSnapshot> {
    let runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let batch = runtime.batches.get(batch_id)?;
    Some(build_batch_snapshot(batch, &runtime.tasks))
}

pub(crate) fn cancel_local_register_batch(batch_id: &str) -> Result<(), String> {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(batch) = runtime.batches.get_mut(batch_id) else {
        return Err("register batch not found".to_string());
    };
    batch.cancelled = true;
    batch.logs.push("batch cancelled locally".to_string());
    for task_uuid in batch.task_uuids.clone() {
        if let Some(task) = runtime.tasks.get_mut(&task_uuid) {
            if !is_terminal_status(task.status.as_str()) {
                task.canceled = true;
                task.status = "cancelled".to_string();
                task.completed_at = Some(now_rfc3339());
            }
        }
    }
    Ok(())
}

pub(crate) fn append_register_task_log(task_uuid: &str, line: &str) {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(task) = runtime.tasks.get_mut(task_uuid) {
        task.logs.push(line.to_string());
    }
}

pub(crate) fn set_register_task_status(
    task_uuid: &str,
    status: &str,
    failure_code: Option<&str>,
    error_message: Option<&str>,
) {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(task) = runtime.tasks.get_mut(task_uuid) {
        task.status = status.to_string();
        task.failure_code = failure_code.map(ToString::to_string);
        task.error_message = error_message.map(ToString::to_string);
        if task.started_at.is_none() && !matches!(status, "queued") {
            task.started_at = Some(now_rfc3339());
        }
        if is_terminal_status(status) {
            task.completed_at = Some(now_rfc3339());
        }
    }
}

pub(crate) fn set_register_task_result(
    task_uuid: &str,
    email: Option<String>,
    payload: Option<String>,
    result: Value,
) {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(task) = runtime.tasks.get_mut(task_uuid) {
        task.email = email;
        task.payload = payload;
        task.result = result;
    }
}

pub(crate) fn read_local_register_task_payload(task_uuid: &str) -> Option<String> {
    let runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    runtime.tasks.get(task_uuid).and_then(|task| task.payload.clone())
}

pub(crate) fn mark_local_register_task_imported(task_uuid: &str, account_id: &str) {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(task) = runtime.tasks.get_mut(task_uuid) {
        task.is_imported = true;
        task.imported_account_id = Some(account_id.to_string());
    }
}

pub(crate) fn task_batch_id(task_uuid: &str) -> Option<String> {
    let runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    runtime
        .tasks
        .get(task_uuid)
        .and_then(|task| task.batch_id.clone())
}

pub(crate) fn cancel_local_register_task(task_uuid: &str) -> Result<(), String> {
    let mut runtime = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(task) = runtime.tasks.get_mut(task_uuid) else {
        return Err("register task not found".to_string());
    };
    task.canceled = true;
    task.status = "cancelled".to_string();
    task.completed_at = Some(now_rfc3339());
    Ok(())
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled" | "succeeded"
    )
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn build_batch_snapshot(
    batch: &LocalRegisterBatchRecord,
    tasks: &HashMap<String, LocalRegisterTaskSnapshot>,
) -> LocalRegisterBatchSnapshot {
    let completed = batch
        .task_uuids
        .iter()
        .filter_map(|task_uuid| tasks.get(task_uuid))
        .filter(|task| matches!(task.status.trim().to_ascii_lowercase().as_str(), "completed" | "succeeded"))
        .count();
    let failed = batch
        .task_uuids
        .iter()
        .filter_map(|task_uuid| tasks.get(task_uuid))
        .filter(|task| matches!(task.status.trim().to_ascii_lowercase().as_str(), "failed" | "cancelled" | "canceled"))
        .count();
    let current_index = completed + failed;
    let finished = batch.cancelled || current_index >= batch.total;
    let status = if batch.cancelled {
        "cancelled".to_string()
    } else if finished {
        "completed".to_string()
    } else {
        "running".to_string()
    };
    LocalRegisterBatchSnapshot {
        batch_id: batch.batch_id.clone(),
        status,
        total: batch.total,
        completed: current_index,
        success: completed,
        failed,
        current_index,
        task_uuids: batch.task_uuids.clone(),
        cancelled: batch.cancelled,
        finished,
        progress: format!("{}/{}", current_index, batch.total),
        logs: batch.logs.clone(),
    }
}

#[cfg(test)]
pub(crate) fn create_local_register_task_for_test(
    input: LocalRegisterTaskInput,
) -> LocalRegisterTaskSnapshot {
    create_local_register_task(input)
}

#[cfg(test)]
pub(crate) fn read_local_register_task_for_test(
    task_uuid: &str,
) -> Option<LocalRegisterTaskSnapshot> {
    read_local_register_task(task_uuid)
}

#[cfg(test)]
pub(crate) fn append_register_task_log_for_test(task_uuid: &str, line: &str) {
    append_register_task_log(task_uuid, line);
}

#[cfg(test)]
pub(crate) fn set_register_task_status_for_test(
    task_uuid: &str,
    status: &str,
    failure_code: Option<&str>,
) {
    set_register_task_status(task_uuid, status, failure_code, None);
}

#[cfg(test)]
pub(crate) fn set_register_task_result_for_test(
    task_uuid: &str,
    email: Option<String>,
    payload: Option<String>,
    result: Value,
) {
    set_register_task_result(task_uuid, email, payload, result);
}

#[cfg(test)]
pub(crate) fn start_local_register_batch_for_test(
    input: LocalRegisterBatchInput,
) -> Result<LocalRegisterBatchSnapshot, String> {
    create_local_register_batch(input)
}

#[cfg(test)]
pub(crate) fn read_local_register_batch_for_test(
    batch_id: &str,
) -> Option<LocalRegisterBatchSnapshot> {
    read_local_register_batch(batch_id)
}

#[cfg(test)]
pub(crate) fn cancel_local_register_batch_for_test(batch_id: &str) -> Result<(), String> {
    cancel_local_register_batch(batch_id)
}

#[cfg(test)]
#[path = "tests/register_runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/register_batch_runtime_tests.rs"]
mod register_batch_runtime_tests;
