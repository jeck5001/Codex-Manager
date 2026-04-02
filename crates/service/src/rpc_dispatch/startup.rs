use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

use crate::startup_snapshot;

/// 函数 `try_handle`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "startup/snapshot" => {
            let request_log_limit = super::i64_param(req, "requestLogLimit");
            super::value_or_error(startup_snapshot::read_startup_snapshot(request_log_limit))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
