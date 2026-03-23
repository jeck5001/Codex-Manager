use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse, PluginListResult};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "plugin/list" => super::value_or_error(
            crate::plugin::list_plugins().map(|items| PluginListResult { items }),
        ),
        "plugin/upsert" => {
            let id = super::string_param(req, "id");
            let name = super::string_param(req, "name");
            let description = super::string_param(req, "description");
            let runtime = super::string_param(req, "runtime");
            let enabled = super::bool_param(req, "enabled");
            let timeout_ms = super::i64_param(req, "timeoutMs");
            let hook_points = req
                .params
                .as_ref()
                .and_then(|params| params.get("hookPoints"))
                .cloned();
            let script_content = super::string_param(req, "scriptContent");
            super::value_or_error(crate::plugin::upsert_plugin(
                id,
                name,
                description,
                runtime,
                hook_points,
                script_content,
                enabled,
                timeout_ms,
            ))
        }
        "plugin/delete" => {
            let plugin_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(crate::plugin::delete_plugin(plugin_id))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
