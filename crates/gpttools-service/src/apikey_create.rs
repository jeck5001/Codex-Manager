use gpttools_core::rpc::types::ApiKeyCreateResult;
use gpttools_core::storage::{now_ts, ApiKey};

use crate::storage_helpers::{generate_key_id, generate_platform_key, hash_platform_key, open_storage};

pub(crate) fn create_api_key(name: Option<String>) -> Result<ApiKeyCreateResult, String> {
    // 创建平台 Key 并写入存储
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let key = generate_platform_key();
    let key_hash = hash_platform_key(&key);
    let key_id = generate_key_id();
    let record = ApiKey {
        id: key_id.clone(),
        name,
        key_hash,
        status: "active".to_string(),
        created_at: now_ts(),
        last_used_at: None,
    };
    storage.insert_api_key(&record).map_err(|e| e.to_string())?;
    Ok(ApiKeyCreateResult { id: key_id, key })
}
