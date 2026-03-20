use codexmanager_core::rpc::types::{AccountListParams, JsonRpcRequest, JsonRpcResponse};

use crate::{
    account_cleanup, account_delete, account_delete_many, account_export, account_import,
    account_list, account_payment, account_register, account_update, account_update_many, auth_account,
    auth_login, auth_tokens,
};

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "account/list" => {
            let pagination_requested = req
                .params
                .as_ref()
                .map(|params| params.get("page").is_some() || params.get("pageSize").is_some())
                .unwrap_or(false);
            let params = req
                .params
                .clone()
                .map(serde_json::from_value::<AccountListParams>)
                .transpose()
                .map(|params| params.unwrap_or_default())
                .map(AccountListParams::normalized)
                .map_err(|err| format!("invalid account/list params: {err}"));
            super::value_or_error(
                params.and_then(|params| account_list::read_accounts(params, pagination_requested)),
            )
        }
        "account/delete" => {
            let account_id = super::str_param(req, "accountId").unwrap_or("");
            super::ok_or_error(account_delete::delete_account(account_id))
        }
        "account/deleteMany" => {
            let account_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("accountIds"))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            super::value_or_error(account_delete_many::delete_accounts(account_ids))
        }
        "account/deleteUnavailableFree" => {
            super::value_or_error(account_cleanup::delete_unavailable_free_accounts())
        }
        "account/update" => {
            let account_id = super::str_param(req, "accountId").unwrap_or("");
            let sort = super::i64_param(req, "sort");
            let status = super::string_param(req, "status");
            super::ok_or_error(account_update::update_account(
                account_id,
                sort,
                status.as_deref(),
            ))
        }
        "account/updateMany" => {
            let account_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("accountIds").or_else(|| params.get("account_ids")))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let status = super::str_param(req, "status").unwrap_or("");
            super::value_or_error(account_update_many::update_accounts_status(account_ids, status))
        }
        "account/import" => {
            let mut contents = req
                .params
                .as_ref()
                .and_then(|params| params.get("contents"))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some(content) = super::string_param(req, "content") {
                if !content.trim().is_empty() {
                    contents.push(content);
                }
            }
            super::value_or_error(account_import::import_account_auth_json(contents))
        }
        "account/payment/generateLink" => {
            let account_id = first_str_param(req, &["accountId", "account_id"]).unwrap_or("");
            let plan_type = first_str_param(req, &["planType", "plan_type"]).unwrap_or("");
            let workspace_name =
                first_str_param(req, &["workspaceName", "workspace_name"]);
            let price_interval =
                first_str_param(req, &["priceInterval", "price_interval"]);
            let seat_quantity = req
                .params
                .as_ref()
                .and_then(|params| params.get("seatQuantity").or_else(|| params.get("seat_quantity")))
                .and_then(|value| value.as_i64());
            let country = first_str_param(req, &["country"]);
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            super::value_or_error(account_payment::generate_payment_link(
                account_id,
                plan_type,
                workspace_name,
                price_interval,
                seat_quantity,
                country,
                proxy.as_deref(),
            ))
        }
        "account/subscription/check" => {
            let account_id = first_str_param(req, &["accountId", "account_id"]).unwrap_or("");
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            super::value_or_error(account_payment::check_account_subscription(
                account_id,
                proxy.as_deref(),
            ))
        }
        "account/subscription/checkMany" => {
            let account_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("accountIds").or_else(|| params.get("account_ids")))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            super::value_or_error(account_payment::check_many_accounts_subscription(
                account_ids,
                proxy.as_deref(),
            ))
        }
        "account/subscription/mark" => {
            let account_id = first_str_param(req, &["accountId", "account_id"]).unwrap_or("");
            let plan_type = first_str_param(req, &["planType", "plan_type"]).unwrap_or("");
            super::value_or_error(account_payment::mark_account_subscription(
                account_id,
                plan_type,
            ))
        }
        "account/teamManager/upload" => {
            let account_id = first_str_param(req, &["accountId", "account_id"]).unwrap_or("");
            super::value_or_error(account_payment::upload_account_to_team_manager(
                account_id,
            ))
        }
        "account/teamManager/uploadMany" => {
            let account_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("accountIds").or_else(|| params.get("account_ids")))
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            super::value_or_error(account_payment::upload_many_accounts_to_team_manager(
                account_ids,
            ))
        }
        "account/teamManager/test" => {
            super::value_or_error(account_payment::test_team_manager_connection(
                req.params.as_ref(),
            ))
        }
        "account/register/availableServices" => {
            super::value_or_error(account_register::available_register_services())
        }
        "account/register/start" => {
            let email_service_type = first_str_param(
                req,
                &["emailServiceType", "email_service_type", "type"],
            )
            .unwrap_or("");
            let email_service_id = req
                .params
                .as_ref()
                .and_then(|params| {
                    params
                        .get("emailServiceId")
                        .or_else(|| params.get("email_service_id"))
                })
                .and_then(|value| value.as_i64());
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            super::value_or_error(account_register::start_register_task(
                email_service_type,
                email_service_id,
                proxy,
            ))
        }
        "account/register/batch/start" => {
            let email_service_type = first_str_param(
                req,
                &["emailServiceType", "email_service_type", "type"],
            )
            .unwrap_or("");
            let email_service_id = req
                .params
                .as_ref()
                .and_then(|params| {
                    params
                        .get("emailServiceId")
                        .or_else(|| params.get("email_service_id"))
                })
                .and_then(|value| value.as_i64());
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            let count = req
                .params
                .as_ref()
                .and_then(|params| params.get("count"))
                .and_then(|value| value.as_i64())
                .unwrap_or(1);
            let interval_min = req
                .params
                .as_ref()
                .and_then(|params| params.get("intervalMin").or_else(|| params.get("interval_min")))
                .and_then(|value| value.as_i64())
                .unwrap_or(5);
            let interval_max = req
                .params
                .as_ref()
                .and_then(|params| params.get("intervalMax").or_else(|| params.get("interval_max")))
                .and_then(|value| value.as_i64())
                .unwrap_or(30);
            let concurrency = req
                .params
                .as_ref()
                .and_then(|params| params.get("concurrency"))
                .and_then(|value| value.as_i64())
                .unwrap_or(1);
            let mode = first_str_param(req, &["mode"]).unwrap_or("pipeline");
            super::value_or_error(account_register::start_register_batch(
                email_service_type,
                email_service_id,
                proxy,
                count,
                interval_min,
                interval_max,
                concurrency,
                mode,
            ))
        }
        "account/register/batch/read" => {
            let batch_id = first_str_param(req, &["batchId", "batch_id"]).unwrap_or("");
            super::value_or_error(account_register::read_register_batch(batch_id))
        }
        "account/register/batch/cancel" => {
            let batch_id = first_str_param(req, &["batchId", "batch_id"]).unwrap_or("");
            super::value_or_error(account_register::cancel_register_batch(batch_id))
        }
        "account/register/tasks/list" => {
            let page = req
                .params
                .as_ref()
                .and_then(|params| params.get("page"))
                .and_then(|value| value.as_i64())
                .unwrap_or(1);
            let page_size = req
                .params
                .as_ref()
                .and_then(|params| params.get("pageSize").or_else(|| params.get("page_size")))
                .and_then(|value| value.as_i64())
                .unwrap_or(20);
            let status = first_str_param(req, &["status"]);
            super::value_or_error(account_register::list_register_tasks(
                page,
                page_size,
                status,
            ))
        }
        "account/register/stats" => {
            super::value_or_error(account_register::register_stats())
        }
        "account/register/task/cancel" => {
            let task_uuid = first_str_param(req, &["taskUuid", "task_uuid"]).unwrap_or("");
            super::value_or_error(account_register::cancel_register_task(task_uuid))
        }
        "account/register/task/delete" => {
            let task_uuid = first_str_param(req, &["taskUuid", "task_uuid"]).unwrap_or("");
            super::value_or_error(account_register::delete_register_task(task_uuid))
        }
        "account/register/outlookAccounts" => {
            super::value_or_error(account_register::list_register_outlook_accounts())
        }
        "account/register/outlookBatch/start" => {
            let service_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceIds").or_else(|| params.get("service_ids")))
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_i64()).collect::<Vec<_>>())
                .unwrap_or_default();
            let skip_registered =
                first_bool_param(req, &["skipRegistered", "skip_registered"]).unwrap_or(true);
            let proxy = first_string_param(req, &["proxy", "proxyUrl", "proxy_url"]);
            let interval_min = req
                .params
                .as_ref()
                .and_then(|params| params.get("intervalMin").or_else(|| params.get("interval_min")))
                .and_then(|value| value.as_i64())
                .unwrap_or(5);
            let interval_max = req
                .params
                .as_ref()
                .and_then(|params| params.get("intervalMax").or_else(|| params.get("interval_max")))
                .and_then(|value| value.as_i64())
                .unwrap_or(30);
            let concurrency = req
                .params
                .as_ref()
                .and_then(|params| params.get("concurrency"))
                .and_then(|value| value.as_i64())
                .unwrap_or(1);
            let mode = first_str_param(req, &["mode"]).unwrap_or("pipeline");
            super::value_or_error(account_register::start_register_outlook_batch(
                service_ids,
                skip_registered,
                proxy,
                interval_min,
                interval_max,
                concurrency,
                mode,
            ))
        }
        "account/register/outlookBatch/read" => {
            let batch_id = first_str_param(req, &["batchId", "batch_id"]).unwrap_or("");
            super::value_or_error(account_register::read_register_outlook_batch(batch_id))
        }
        "account/register/outlookBatch/cancel" => {
            let batch_id = first_str_param(req, &["batchId", "batch_id"]).unwrap_or("");
            super::value_or_error(account_register::cancel_register_outlook_batch(batch_id))
        }
        "account/register/emailServices/types" => {
            super::value_or_error(account_register::register_email_service_types())
        }
        "account/register/emailServices/list" => {
            let service_type = first_str_param(req, &["serviceType", "service_type"]);
            let enabled_only =
                first_bool_param(req, &["enabledOnly", "enabled_only"]).unwrap_or(false);
            super::value_or_error(account_register::list_register_email_services(
                service_type,
                enabled_only,
            ))
        }
        "account/register/emailServices/stats" => {
            super::value_or_error(account_register::register_email_service_stats())
        }
        "account/register/emailServices/readFull" => {
            let service_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceId").or_else(|| params.get("service_id")))
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            super::value_or_error(account_register::read_register_email_service_full(service_id))
        }
        "account/register/emailServices/create" => {
            let service_type = first_str_param(
                req,
                &["serviceType", "service_type", "type"],
            )
            .unwrap_or("");
            let name = first_str_param(req, &["name"]).unwrap_or("");
            let enabled = first_bool_param(req, &["enabled"]).unwrap_or(true);
            let priority = req
                .params
                .as_ref()
                .and_then(|params| params.get("priority"))
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let config = req
                .params
                .as_ref()
                .and_then(|params| params.get("config"))
                .cloned()
                .unwrap_or(serde_json::json!({}));
            super::value_or_error(account_register::create_register_email_service(
                service_type,
                name,
                enabled,
                priority,
                config,
            ))
        }
        "account/register/emailServices/update" => {
            let service_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceId").or_else(|| params.get("service_id")))
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let enabled = first_bool_param(req, &["enabled"]);
            let priority = req
                .params
                .as_ref()
                .and_then(|params| params.get("priority"))
                .and_then(|value| value.as_i64());
            let config = req
                .params
                .as_ref()
                .and_then(|params| params.get("config"))
                .cloned();
            super::value_or_error(account_register::update_register_email_service(
                service_id,
                first_str_param(req, &["name"]),
                enabled,
                priority,
                config,
            ))
        }
        "account/register/emailServices/delete" => {
            let service_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceId").or_else(|| params.get("service_id")))
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            super::value_or_error(account_register::delete_register_email_service(service_id))
        }
        "account/register/emailServices/test" => {
            let service_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceId").or_else(|| params.get("service_id")))
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            super::value_or_error(account_register::test_register_email_service(service_id))
        }
        "account/register/emailServices/setEnabled" => {
            let service_id = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceId").or_else(|| params.get("service_id")))
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let enabled = first_bool_param(req, &["enabled"]).unwrap_or(true);
            super::value_or_error(account_register::set_register_email_service_enabled(
                service_id, enabled,
            ))
        }
        "account/register/emailServices/outlookBatchImport" => {
            let data = first_str_param(req, &["data"]).unwrap_or("");
            let enabled = first_bool_param(req, &["enabled"]).unwrap_or(true);
            let priority = req
                .params
                .as_ref()
                .and_then(|params| params.get("priority"))
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            super::value_or_error(account_register::batch_import_register_outlook(
                data, enabled, priority,
            ))
        }
        "account/register/emailServices/outlookBatchDelete" => {
            let service_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceIds").or_else(|| params.get("service_ids")))
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_i64()).collect::<Vec<_>>())
                .unwrap_or_default();
            super::value_or_error(account_register::batch_delete_register_outlook(service_ids))
        }
        "account/register/emailServices/reorder" => {
            let service_ids = req
                .params
                .as_ref()
                .and_then(|params| params.get("serviceIds").or_else(|| params.get("service_ids")))
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_i64()).collect::<Vec<_>>())
                .unwrap_or_default();
            super::value_or_error(account_register::reorder_register_email_services(service_ids))
        }
        "account/register/emailServices/testTempmail" => {
            let api_url = first_str_param(req, &["apiUrl", "api_url"]);
            super::value_or_error(account_register::test_register_tempmail(api_url))
        }
        "account/register/task" => {
            let task_uuid = first_str_param(req, &["taskUuid", "task_uuid"]).unwrap_or("");
            super::value_or_error(account_register::read_register_task(task_uuid))
        }
        "account/register/import" => {
            let task_uuid = first_str_param(req, &["taskUuid", "task_uuid"]).unwrap_or("");
            super::value_or_error(account_register::import_register_task(task_uuid))
        }
        "account/register/importByEmail" => {
            let email = first_str_param(req, &["email"]).unwrap_or("");
            super::value_or_error(account_register::import_register_account_by_email(email))
        }
        "account/export" => {
            let output_dir = super::str_param(req, "outputDir").unwrap_or("");
            super::value_or_error(account_export::export_accounts_to_directory(output_dir))
        }
        "account/exportData" => super::value_or_error(account_export::export_accounts_data()),
        "account/login/start" => {
            let login_type = super::str_param(req, "type").unwrap_or("chatgpt");
            if login_type.eq_ignore_ascii_case("chatgptAuthTokens") {
                let params = auth_account::ChatgptAuthTokensLoginInput {
                    access_token: first_string_param(req, &["accessToken", "access_token"])
                        .unwrap_or_default(),
                    refresh_token: first_string_param(req, &["refreshToken", "refresh_token"]),
                    id_token: first_string_param(req, &["idToken", "id_token"]),
                    chatgpt_account_id: first_string_param(
                        req,
                        &["chatgptAccountId", "chatgpt_account_id", "accountId"],
                    ),
                    workspace_id: first_string_param(req, &["workspaceId", "workspace_id"]),
                    chatgpt_plan_type: first_string_param(
                        req,
                        &["chatgptPlanType", "chatgpt_plan_type", "planType"],
                    ),
                };
                super::value_or_error(auth_account::login_with_chatgpt_auth_tokens(params))
            } else {
                let open_browser = super::bool_param(req, "openBrowser").unwrap_or(true);
                let note = super::string_param(req, "note");
                let tags = super::string_param(req, "tags");
                let group_name = super::string_param(req, "groupName");
                let workspace_id = super::string_param(req, "workspaceId").and_then(|v| {
                    if v.trim().is_empty() {
                        None
                    } else {
                        Some(v)
                    }
                });
                super::value_or_error(auth_login::login_start(
                    login_type,
                    open_browser,
                    note,
                    tags,
                    group_name,
                    workspace_id,
                ))
            }
        }
        "account/login/status" => {
            let login_id = super::str_param(req, "loginId").unwrap_or("");
            super::as_json(auth_login::login_status(login_id))
        }
        "account/login/complete" => {
            let state = super::str_param(req, "state").unwrap_or("");
            let code = super::str_param(req, "code").unwrap_or("");
            let redirect_uri = super::str_param(req, "redirectUri");
            if state.is_empty() || code.is_empty() {
                serde_json::json!({"ok": false, "error": "missing code/state"})
            } else {
                super::ok_or_error(auth_tokens::complete_login_with_redirect(
                    state,
                    code,
                    redirect_uri,
                ))
            }
        }
        "account/chatgptAuthTokens/refresh" => {
            let previous_account_id =
                first_str_param(req, &["previousAccountId", "previous_account_id"]);
            super::value_or_error(auth_account::refresh_current_chatgpt_auth_tokens(
                previous_account_id,
            ))
        }
        "account/read" => {
            let refresh_token =
                first_bool_param(req, &["refreshToken", "refresh_token"]).unwrap_or(false);
            super::value_or_error(auth_account::read_current_account(refresh_token))
        }
        "account/logout" => super::value_or_error(auth_account::logout_current_account()),
        _ => return None,
    };

    Some(super::response(req, result))
}

fn first_str_param<'a>(req: &'a JsonRpcRequest, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| super::str_param(req, key))
}

fn first_string_param(req: &JsonRpcRequest, keys: &[&str]) -> Option<String> {
    first_str_param(req, keys).map(|value| value.to_string())
}

fn first_bool_param(req: &JsonRpcRequest, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| super::bool_param(req, key))
}
