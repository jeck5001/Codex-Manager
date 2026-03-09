use codexmanager_core::storage::Storage;
use rfd::FileDialog;

use crate::app_storage::{
    read_account_import_contents_from_directory, resolve_db_path_with_legacy_migration,
};
use crate::commands::shared::rpc_call_in_background;
use crate::rpc_client::rpc_call;

#[tauri::command]
pub async fn service_account_list(
    addr: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
    query: Option<String>,
    filter: Option<String>,
    group_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut params = serde_json::Map::new();
    if let Some(value) = page {
        params.insert("page".to_string(), serde_json::json!(value));
    }
    if let Some(value) = page_size {
        params.insert("pageSize".to_string(), serde_json::json!(value));
    }
    if let Some(value) = query {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            params.insert("query".to_string(), serde_json::json!(trimmed));
        }
    }
    if let Some(value) = filter {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            params.insert("filter".to_string(), serde_json::json!(trimmed));
        }
    }
    if let Some(value) = group_filter {
        let trimmed = value.trim();
        if !trimmed.is_empty() && trimmed != "all" {
            params.insert("groupFilter".to_string(), serde_json::json!(trimmed));
        }
    }
    let payload = if params.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(params))
    };
    rpc_call_in_background("account/list", addr, payload).await
}

#[tauri::command]
pub async fn service_account_delete(
    addr: Option<String>,
    account_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountId": account_id });
    rpc_call_in_background("account/delete", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_delete_many(
    addr: Option<String>,
    account_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountIds": account_ids });
    rpc_call_in_background("account/deleteMany", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_delete_unavailable_free(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("account/deleteUnavailableFree", addr, None).await
}

#[tauri::command]
pub async fn service_account_update(
    addr: Option<String>,
    account_id: String,
    sort: i64,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountId": account_id, "sort": sort });
    rpc_call_in_background("account/update", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_import(
    addr: Option<String>,
    contents: Option<Vec<String>>,
    content: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut payload_contents = contents.unwrap_or_default();
    if let Some(single) = content {
        if !single.trim().is_empty() {
            payload_contents.push(single);
        }
    }
    let params = serde_json::json!({ "contents": payload_contents });
    rpc_call_in_background("account/import", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_account_import_by_directory(
    _addr: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected_dir = FileDialog::new()
            .set_title("选择账号导入目录")
            .pick_folder();
        let Some(dir_path) = selected_dir else {
            return Ok(serde_json::json!({
              "result": {
                "ok": true,
                "canceled": true
              }
            }));
        };

        let (json_files, contents) = read_account_import_contents_from_directory(&dir_path)?;
        Ok(serde_json::json!({
          "result": {
            "ok": true,
            "canceled": false,
            "directoryPath": dir_path.to_string_lossy().to_string(),
            "fileCount": json_files.len(),
            "contents": contents
          }
        }))
    })
    .await
    .map_err(|err| format!("service_account_import_by_directory task failed: {err}"))?
}

#[tauri::command]
pub async fn service_account_export_by_account_files(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected_dir = FileDialog::new()
            .set_title("选择账号导出目录")
            .pick_folder();
        let Some(dir_path) = selected_dir else {
            return Ok(serde_json::json!({
              "result": {
                "ok": true,
                "canceled": true
              }
            }));
        };
        let params = serde_json::json!({
          "outputDir": dir_path.to_string_lossy().to_string()
        });
        rpc_call("account/export", addr, Some(params))
    })
    .await
    .map_err(|err| format!("service_account_export_by_account_files task failed: {err}"))?
}

#[tauri::command]
pub async fn local_account_delete(
    app: tauri::AppHandle,
    account_id: String,
) -> Result<serde_json::Value, String> {
    let db_path = resolve_db_path_with_legacy_migration(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut storage = Storage::open(db_path).map_err(|e| e.to_string())?;
        storage
            .delete_account(&account_id)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "ok": true }))
    })
    .await
    .map_err(|err| format!("local_account_delete task failed: {err}"))?
}
