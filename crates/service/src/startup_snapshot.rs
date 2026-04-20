use codexmanager_core::rpc::types::{AccountListParams, StartupAccountSummary, StartupSnapshotResult};
use std::collections::HashMap;

use crate::{
    account_list, apikey_list, apikey_models, gateway, requestlog_list, requestlog_today_summary,
    usage_aggregate, usage_list,
};

pub(crate) fn read_startup_snapshot(
    request_log_limit: Option<i64>,
) -> Result<StartupSnapshotResult, String> {
    let usage_snapshots = usage_list::read_usage_snapshots()?;
    let usage_by_account_id: HashMap<String, _> = usage_snapshots
        .into_iter()
        .filter_map(|snapshot| {
            snapshot
                .account_id
                .clone()
                .map(|account_id| (account_id, snapshot))
        })
        .collect();
    let accounts = account_list::read_accounts(AccountListParams::default(), false)?
        .items
        .into_iter()
        .map(|account| StartupAccountSummary {
            usage: usage_by_account_id.get(&account.id).cloned(),
            account,
        })
        .collect();
    let usage_aggregate_summary = usage_aggregate::read_usage_aggregate_summary()?;
    let usage_prediction_summary = crate::usage_prediction::read_usage_prediction_summary()?;
    let failure_reason_summary = crate::failure_summary::read_failure_reason_summary()?;
    let governance_summary = crate::governance_summary::read_governance_summary()?;
    let operation_audits = crate::operation_audit_summary::read_recent_operation_audits()?;
    let api_keys = apikey_list::read_api_keys()?;
    let api_model_options = apikey_models::read_model_options(false)?.items;
    let manual_preferred_account_id = gateway::manual_preferred_account();
    let request_log_today_summary = requestlog_today_summary::read_requestlog_today_summary()?;
    let request_logs = requestlog_list::read_request_logs(None, request_log_limit)?;
    let recent_request_log_count = request_logs.len() as i64;
    let latest_request_account_id = request_logs
        .iter()
        .filter_map(|item| {
            item.account_id
                .as_ref()
                .map(|account_id| (item.created_at, account_id.as_str()))
        })
        .max_by_key(|(created_at, _)| *created_at)
        .map(|(_, account_id)| account_id.to_string());

    Ok(StartupSnapshotResult {
        accounts,
        usage_aggregate_summary,
        usage_prediction_summary,
        failure_reason_summary,
        governance_summary,
        operation_audits,
        api_keys,
        api_model_options,
        manual_preferred_account_id,
        request_log_today_summary,
        recent_request_log_count,
        latest_request_account_id,
    })
}
