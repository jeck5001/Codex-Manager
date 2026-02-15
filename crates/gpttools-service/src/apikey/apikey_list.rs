use gpttools_core::rpc::types::ApiKeySummary;

use crate::storage_helpers::open_storage;

pub(crate) fn read_api_keys() -> Vec<ApiKeySummary> {
    // 读取平台 Key 列表
    let storage = match open_storage() {
        Some(storage) => storage,
        None => return Vec::new(),
    };
    let keys = match storage.list_api_keys() {
        Ok(keys) => keys,
        Err(_) => return Vec::new(),
    };
    keys.into_iter()
        .map(|key| ApiKeySummary {
            id: key.id,
            name: key.name,
            model_slug: key.model_slug,
            reasoning_effort: key.reasoning_effort,
            client_type: key.client_type,
            protocol_type: key.protocol_type,
            auth_scheme: key.auth_scheme,
            upstream_base_url: key.upstream_base_url,
            static_headers_json: key.static_headers_json,
            status: key.status,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
        })
        .collect()
}
