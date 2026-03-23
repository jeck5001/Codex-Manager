use codexmanager_core::rpc::types::{
    AlertChannelListResult, AlertHistoryListResult, AlertRuleListResult, JsonRpcRequest,
    JsonRpcResponse,
};

use crate::{alert_channels, alert_history, alert_rules};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "alert/rules/list" => super::value_or_error(
            alert_rules::list_alert_rules().map(|items| AlertRuleListResult { items }),
        ),
        "alert/rules/upsert" => {
            let id = super::string_param(req, "id");
            let name = super::string_param(req, "name");
            let rule_type = super::string_param(req, "type");
            let enabled = super::bool_param(req, "enabled");
            let config = req
                .params
                .as_ref()
                .and_then(|params| params.get("config"))
                .cloned();
            super::value_or_error(alert_rules::upsert_alert_rule(
                id, name, rule_type, config, enabled,
            ))
        }
        "alert/rules/delete" => {
            let rule_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(alert_rules::delete_alert_rule(rule_id))
        }
        "alert/channels/list" => super::value_or_error(
            alert_channels::list_alert_channels().map(|items| AlertChannelListResult { items }),
        ),
        "alert/channels/upsert" => {
            let id = super::string_param(req, "id");
            let name = super::string_param(req, "name");
            let channel_type = super::string_param(req, "type");
            let enabled = super::bool_param(req, "enabled");
            let config = req
                .params
                .as_ref()
                .and_then(|params| params.get("config"))
                .cloned();
            super::value_or_error(alert_channels::upsert_alert_channel(
                id,
                name,
                channel_type,
                config,
                enabled,
            ))
        }
        "alert/channels/delete" => {
            let channel_id = super::str_param(req, "id").unwrap_or("");
            super::ok_or_error(alert_channels::delete_alert_channel(channel_id))
        }
        "alert/channels/test" => {
            let channel_id = super::str_param(req, "id").unwrap_or("");
            super::value_or_error(alert_channels::test_alert_channel(channel_id))
        }
        "alert/history/list" => super::value_or_error(
            alert_history::list_alert_history(super::i64_param(req, "limit"))
                .map(|items| AlertHistoryListResult { items }),
        ),
        _ => return None,
    };

    Some(super::response(req, result))
}
