use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

use crate::{dashboard_health, dashboard_trend};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "dashboard/health" => super::value_or_error(dashboard_health::read_dashboard_health()),
        "dashboard/trend" => super::value_or_error(dashboard_trend::read_dashboard_trend()),
        _ => return None,
    };

    Some(super::response(req, result))
}
