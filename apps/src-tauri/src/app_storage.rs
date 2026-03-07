use codexmanager_core::storage::Storage;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

fn collect_json_files_recursively(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|err| format!("read dir failed ({}): {err}", root.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("read dir entry failed ({}): {err}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files_recursively(&path, output)?;
            continue;
        }
        let is_json = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if is_json {
            output.push(path);
        }
    }
    Ok(())
}

pub(crate) fn read_account_import_contents_from_directory(
    root: &Path,
) -> Result<(Vec<PathBuf>, Vec<String>), String> {
    let mut json_files = Vec::new();
    collect_json_files_recursively(root, &mut json_files)?;
    json_files.sort();

    let mut contents = Vec::with_capacity(json_files.len());
    for path in &json_files {
        let text = fs::read_to_string(path)
            .map_err(|err| format!("read json file failed ({}): {err}", path.display()))?;
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            contents.push(trimmed.to_string());
        }
    }
    Ok((json_files, contents))
}

pub(crate) fn resolve_rpc_token_path_for_db(db_path: &Path) -> PathBuf {
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join("codexmanager.rpc-token")
}

pub(crate) fn apply_runtime_storage_env(app: &tauri::AppHandle) {
    if let Ok(data_path) = resolve_db_path_with_legacy_migration(app) {
        std::env::set_var("CODEXMANAGER_DB_PATH", &data_path);
        let token_path = resolve_rpc_token_path_for_db(&data_path);
        std::env::set_var("CODEXMANAGER_RPC_TOKEN_FILE", &token_path);
        log::info!("db path: {}", data_path.display());
        log::info!("rpc token path: {}", token_path.display());
    }
}

pub(crate) fn resolve_db_path_with_legacy_migration(
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    let mut data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "app data dir not found".to_string())?;
    if let Err(err) = fs::create_dir_all(&data_dir) {
        log::warn!("Failed to create app data dir: {}", err);
    }
    data_dir.push("codexmanager.db");
    maybe_migrate_legacy_db(&data_dir);
    Ok(data_dir)
}

fn maybe_migrate_legacy_db(current_db: &Path) {
    let current_has_data = db_has_user_data(current_db);
    if current_has_data {
        return;
    }

    let needs_bootstrap = !current_db.is_file() || !current_has_data;
    if !needs_bootstrap {
        return;
    }

    for legacy_db in legacy_db_candidates(current_db) {
        if !legacy_db.is_file() {
            continue;
        }
        if !db_has_user_data(&legacy_db) {
            continue;
        }

        if let Some(parent) = current_db.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if current_db.is_file() {
            let backup = current_db.with_extension("db.empty.bak");
            if let Err(err) = fs::copy(current_db, &backup) {
                log::warn!(
                    "Failed to backup empty current db {} -> {}: {}",
                    current_db.display(),
                    backup.display(),
                    err
                );
            }
        }

        match fs::copy(&legacy_db, current_db) {
            Ok(_) => {
                log::info!(
                    "Migrated legacy db {} -> {}",
                    legacy_db.display(),
                    current_db.display()
                );
                return;
            }
            Err(err) => {
                log::warn!(
                    "Failed to migrate legacy db {} -> {}: {}",
                    legacy_db.display(),
                    current_db.display(),
                    err
                );
            }
        }
    }
}

fn db_has_user_data(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let storage = match Storage::open(path) {
        Ok(storage) => storage,
        Err(_) => return false,
    };
    let _ = storage.init();
    storage
        .list_accounts()
        .map(|items| !items.is_empty())
        .unwrap_or(false)
        || storage
            .list_tokens()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
        || storage
            .list_api_keys()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
}

fn legacy_db_candidates(current_db: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Some(parent) = current_db.parent() {
        out.push(parent.join("gpttools.db"));
        if parent
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("com.codexmanager.desktop"))
        {
            if let Some(root) = parent.parent() {
                out.push(root.join("com.gpttools.desktop").join("gpttools.db"));
            }
        }
    }

    out.retain(|candidate| candidate != current_db);
    let mut dedup = Vec::new();
    for candidate in out {
        if !dedup.iter().any(|item| item == &candidate) {
            dedup.push(candidate);
        }
    }
    dedup
}
