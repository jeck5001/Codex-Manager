use codexmanager_core::rpc::types::{
    ApiKeyListResult, ApiKeyUsageStatListResult, JsonRpcRequest, JsonRpcResponse,
};

use crate::{
    apikey_allowed_models, apikey_create, apikey_delete, apikey_disable, apikey_enable,
    apikey_list, apikey_model_fallback, apikey_models, apikey_rate_limit, apikey_read_secret,
    apikey_renew, apikey_response_cache, apikey_update_model, apikey_usage_stats,
};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "apikey/list" => super::value_or_error(
            apikey_list::read_api_keys().map(|items| ApiKeyListResult { items }),
        ),
        "apikey/create" => {
            let name = super::string_param(req, "name");
            let model_slug = super::string_param(req, "modelSlug");
            let reasoning_effort = super::string_param(req, "reasoningEffort");
            let protocol_type = super::string_param(req, "protocolType");
            let upstream_base_url = super::string_param(req, "upstreamBaseUrl");
            let static_headers_json = super::string_param(req, "staticHeadersJson");
            let expires_at = super::i64_param(req, "expiresAt");
            super::value_or_error(apikey_create::create_api_key(
                name,
                model_slug,
                reasoning_effort,
                protocol_type,
                upstream_base_url,
                static_headers_json,
                expires_at,
            ))
        }
        "apikey/readSecret" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(apikey_read_secret::read_api_key_secret(key_id))
        }
        "apikey/rateLimit/get" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(apikey_rate_limit::read_api_key_rate_limit(key_id))
        }
        "apikey/rateLimit/set" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let rpm = super::i64_param(req, "rpm");
            let tpm = super::i64_param(req, "tpm");
            let daily_limit = super::i64_param(req, "dailyLimit");
            super::ok_or_error(apikey_rate_limit::update_api_key_rate_limit(
                key_id,
                rpm,
                tpm,
                daily_limit,
            ))
        }
        "apikey/modelFallback/get" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(apikey_model_fallback::read_api_key_model_fallback(key_id))
        }
        "apikey/modelFallback/set" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let model_chain = req
                .params
                .as_ref()
                .and_then(|params| params.get("modelChain"))
                .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
                .unwrap_or_default();
            super::ok_or_error(apikey_model_fallback::update_api_key_model_fallback(
                key_id,
                model_chain,
            ))
        }
        "apikey/allowedModels/get" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(apikey_allowed_models::read_api_key_allowed_models(key_id))
        }
        "apikey/allowedModels/set" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let allowed_models = req
                .params
                .as_ref()
                .and_then(|params| params.get("allowedModels"))
                .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
                .unwrap_or_default();
            super::ok_or_error(apikey_allowed_models::update_api_key_allowed_models(
                key_id,
                allowed_models,
            ))
        }
        "apikey/responseCache/get" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(apikey_response_cache::read_api_key_response_cache(key_id))
        }
        "apikey/responseCache/set" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let enabled = super::bool_param(req, "enabled").unwrap_or(false);
            super::ok_or_error(apikey_response_cache::update_api_key_response_cache(
                key_id, enabled,
            ))
        }
        "apikey/models" => {
            let refresh_remote = super::bool_param(req, "refreshRemote").unwrap_or(false);
            super::value_or_error(apikey_models::read_model_options(refresh_remote))
        }
        "apikey/usageStats" => super::value_or_error(
            apikey_usage_stats::read_api_key_usage_stats()
                .map(|items| ApiKeyUsageStatListResult { items }),
        ),
        "apikey/updateModel" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let model_slug = super::string_param(req, "modelSlug");
            let reasoning_effort = super::string_param(req, "reasoningEffort");
            let protocol_type = super::string_param(req, "protocolType");
            let upstream_base_url = super::string_param(req, "upstreamBaseUrl");
            let static_headers_json = super::string_param(req, "staticHeadersJson");
            super::ok_or_error(apikey_update_model::update_api_key_model(
                key_id,
                model_slug,
                reasoning_effort,
                protocol_type,
                upstream_base_url,
                static_headers_json,
            ))
        }
        "apikey/delete" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(apikey_delete::delete_api_key(key_id))
        }
        "apikey/renew" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            let expires_at = super::i64_param(req, "expiresAt");
            super::ok_or_error(apikey_renew::renew_api_key(key_id, expires_at))
        }
        "apikey/disable" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(apikey_disable::disable_api_key(key_id))
        }
        "apikey/enable" => {
            let key_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(apikey_enable::enable_api_key(key_id))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
