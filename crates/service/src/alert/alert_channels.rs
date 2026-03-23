use codexmanager_core::rpc::types::{AlertChannelItem, AlertChannelTestResult};
use codexmanager_core::storage::{now_ts, AlertChannel};
use rand::RngCore;
use serde_json::Value;

use crate::alert_sender::send_test_alert;
use crate::storage_helpers::open_storage;

const ALLOWED_CHANNEL_TYPES: &[&str] = &["webhook", "bark", "telegram", "wecom"];

pub(crate) fn list_alert_channels() -> Result<Vec<AlertChannelItem>, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .list_alert_channels()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(to_alert_channel_item)
        .collect()
}

pub(crate) fn upsert_alert_channel(
    id: Option<String>,
    name: Option<String>,
    channel_type: Option<String>,
    config: Option<Value>,
    enabled: Option<bool>,
) -> Result<AlertChannelItem, String> {
    let normalized_name = name
        .unwrap_or_default()
        .trim()
        .to_string();
    if normalized_name.is_empty() {
        return Err("alert channel name required".to_string());
    }

    let normalized_type = channel_type
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !ALLOWED_CHANNEL_TYPES.contains(&normalized_type.as_str()) {
        return Err(format!("unsupported alert channel type: {normalized_type}"));
    }

    let config_value = normalize_config(config)?;
    let now = now_ts();
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let channel_id = normalize_id(id, "ac");
    let created_at = storage
        .find_alert_channel_by_id(channel_id.as_str())
        .map_err(|err| err.to_string())?
        .map(|item| item.created_at)
        .unwrap_or(now);

    let record = AlertChannel {
        id: channel_id,
        name: normalized_name,
        channel_type: normalized_type,
        config_json: serde_json::to_string(&config_value)
            .map_err(|err| format!("serialize alert channel config failed: {err}"))?,
        enabled: enabled.unwrap_or(true),
        created_at,
        updated_at: now,
    };
    storage
        .upsert_alert_channel(&record)
        .map_err(|err| err.to_string())?;
    to_alert_channel_item(record)
}

pub(crate) fn delete_alert_channel(channel_id: &str) -> Result<(), String> {
    if channel_id.trim().is_empty() {
        return Err("alert channel id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .delete_alert_channel(channel_id.trim())
        .map_err(|err| err.to_string())
}

pub(crate) fn test_alert_channel(channel_id: &str) -> Result<AlertChannelTestResult, String> {
    if channel_id.trim().is_empty() {
        return Err("alert channel id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let channel = storage
        .find_alert_channel_by_id(channel_id.trim())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "alert channel not found".to_string())?;
    let sent_at = now_ts();
    let payload = serde_json::json!({
        "event": "codexmanager.alert.test",
        "channelId": channel.id,
        "channelType": channel.channel_type,
        "name": channel.name,
        "sentAt": sent_at,
    });
    let result = send_test_alert(&channel, &payload);
    let history_status = if result.is_ok() {
        "test_success"
    } else {
        "test_failure"
    };
    let history_message = match &result {
        Ok(_) => format!("Test alert delivered via {}", channel.channel_type),
        Err(err) => format!("Test alert failed via {}: {err}", channel.channel_type),
    };
    storage
        .insert_alert_history(None, Some(channel.id.as_str()), history_status, &history_message)
        .map_err(|err| err.to_string())?;
    result?;

    Ok(AlertChannelTestResult {
        channel_id: channel.id,
        status: "sent".to_string(),
        sent_at,
    })
}

pub(crate) fn to_alert_channel_item(channel: AlertChannel) -> Result<AlertChannelItem, String> {
    Ok(AlertChannelItem {
        id: channel.id,
        name: channel.name,
        channel_type: channel.channel_type,
        config: serde_json::from_str(channel.config_json.as_str()).unwrap_or(Value::Object(Default::default())),
        enabled: channel.enabled,
        created_at: channel.created_at,
        updated_at: channel.updated_at,
    })
}

fn normalize_config(config: Option<Value>) -> Result<Value, String> {
    match config.unwrap_or_else(|| Value::Object(Default::default())) {
        Value::Object(map) => Ok(Value::Object(map)),
        _ => Err("alert channel config must be an object".to_string()),
    }
}

fn normalize_id(id: Option<String>, prefix: &str) -> String {
    let trimmed = id.unwrap_or_default().trim().to_string();
    if !trimmed.is_empty() {
        return trimmed;
    }

    let mut bytes = [0_u8; 6];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = format!("{prefix}_");
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
