use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "appSettings/get" => super::value_or_error(crate::app_settings_get()),
        "appSettings/set" => super::value_or_error(crate::app_settings_set(req.params.as_ref())),
        "webAuth/status" => super::value_or_error(crate::web_auth_status_value()),
        "webAuth/password/set" => {
            let password = super::str_param(req, "password").unwrap_or("");
            super::value_or_error(
                crate::set_web_access_password(Some(password))
                    .map(|configured| serde_json::json!({ "passwordConfigured": configured })),
            )
        }
        "webAuth/password/clear" => super::value_or_error(
            crate::set_web_access_password(None)
                .map(|configured| serde_json::json!({ "passwordConfigured": configured })),
        ),
        "webAuth/2fa/setup" => super::value_or_error(crate::web_auth_two_factor_setup()),
        "webAuth/2fa/verify" => {
            let setup_token = super::str_param(req, "setupToken").unwrap_or("");
            let code = super::str_param(req, "code")
                .or_else(|| super::str_param(req, "recoveryCode"))
                .unwrap_or("");
            let result = if !setup_token.trim().is_empty() {
                crate::web_auth_two_factor_verify(setup_token, code)
            } else {
                crate::web_auth_two_factor_verify_current(code)
            };
            super::value_or_error(result)
        }
        "webAuth/2fa/disable" => {
            let code = super::str_param(req, "code")
                .or_else(|| super::str_param(req, "recoveryCode"))
                .unwrap_or("");
            super::value_or_error(crate::web_auth_two_factor_disable(code))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}
