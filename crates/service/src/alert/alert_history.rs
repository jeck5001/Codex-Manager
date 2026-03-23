use codexmanager_core::rpc::types::AlertHistoryItem;

use crate::storage_helpers::open_storage;

const DEFAULT_ALERT_HISTORY_LIMIT: i64 = 50;
const MAX_ALERT_HISTORY_LIMIT: i64 = 200;

pub(crate) fn list_alert_history(limit: Option<i64>) -> Result<Vec<AlertHistoryItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let normalized_limit = match limit.unwrap_or(DEFAULT_ALERT_HISTORY_LIMIT) {
        value if value <= 0 => DEFAULT_ALERT_HISTORY_LIMIT,
        value if value > MAX_ALERT_HISTORY_LIMIT => MAX_ALERT_HISTORY_LIMIT,
        value => value,
    };
    storage
        .list_alert_history(normalized_limit)
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|entry| {
            Ok(AlertHistoryItem {
                id: entry.id,
                rule_id: entry.rule_id,
                rule_name: entry.rule_name,
                channel_id: entry.channel_id,
                channel_name: entry.channel_name,
                status: entry.status,
                message: entry.message,
                created_at: entry.created_at,
            })
        })
        .collect()
}
