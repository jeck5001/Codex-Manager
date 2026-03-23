use codexmanager_core::storage::AlertChannel;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub(crate) trait AlertSender {
    fn send(&self, title: &str, message: &str, payload: &Value) -> Result<(), String>;
}

pub(crate) fn send_alert(
    channel: &AlertChannel,
    title: &str,
    message: &str,
    payload: &Value,
) -> Result<(), String> {
    let sender = build_sender(channel)?;
    sender.send(title, message, payload)
}

pub(crate) fn send_test_alert(channel: &AlertChannel, payload: &Value) -> Result<(), String> {
    send_alert(
        channel,
        "CodexManager 告警测试",
        "这是一条测试通知，用于验证当前告警渠道配置。",
        payload,
    )
}

fn build_sender(channel: &AlertChannel) -> Result<Box<dyn AlertSender>, String> {
    let config = serde_json::from_str::<Value>(channel.config_json.as_str())
        .map_err(|err| format!("invalid alert channel config json: {err}"))?;
    match channel.channel_type.as_str() {
        "webhook" => Ok(Box::new(WebhookSender {
            url: required_string(&config, "url")?,
            client: build_http_client()?,
        })),
        "bark" => Ok(Box::new(BarkSender {
            url: required_string(&config, "url")?,
            client: build_http_client()?,
        })),
        "telegram" => Ok(Box::new(TelegramSender {
            bot_token: required_string(&config, "botToken")?,
            chat_id: required_string(&config, "chatId")?,
            client: build_http_client()?,
        })),
        "wecom" => Ok(Box::new(WecomSender {
            webhook_url: required_string(&config, "webhookUrl")?,
            client: build_http_client()?,
        })),
        other => Err(format!("unsupported alert channel type: {other}")),
    }
}

fn build_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .no_proxy()
        .build()
        .map_err(|err| format!("build alert http client failed: {err}"))
}

fn required_string(config: &Value, key: &str) -> Result<String, String> {
    let value = config
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("alert channel config missing {key}"))?;
    Ok(value.to_string())
}

struct WebhookSender {
    url: String,
    client: Client,
}

impl AlertSender for WebhookSender {
    fn send(&self, title: &str, message: &str, payload: &Value) -> Result<(), String> {
        let body = json!({
            "title": title,
            "message": message,
            "payload": payload,
        });
        self.client
            .post(&self.url)
            .json(&body)
            .send()
            .and_then(|response| response.error_for_status())
            .map(|_| ())
            .map_err(|err| format!("webhook send failed: {err}"))
    }
}

struct BarkSender {
    url: String,
    client: Client,
}

impl AlertSender for BarkSender {
    fn send(&self, title: &str, message: &str, _payload: &Value) -> Result<(), String> {
        let title = urlencoding::encode(title);
        let message = urlencoding::encode(message);
        let target = format!("{}/{}/{}", self.url.trim_end_matches('/'), title, message);
        self.client
            .get(&target)
            .send()
            .and_then(|response| response.error_for_status())
            .map(|_| ())
            .map_err(|err| format!("bark send failed: {err}"))
    }
}

struct TelegramSender {
    bot_token: String,
    chat_id: String,
    client: Client,
}

impl AlertSender for TelegramSender {
    fn send(&self, title: &str, message: &str, _payload: &Value) -> Result<(), String> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let body = json!({
            "chat_id": self.chat_id,
            "text": format!("{title}\n{message}"),
        });
        self.client
            .post(url)
            .json(&body)
            .send()
            .and_then(|response| response.error_for_status())
            .map(|_| ())
            .map_err(|err| format!("telegram send failed: {err}"))
    }
}

struct WecomSender {
    webhook_url: String,
    client: Client,
}

impl AlertSender for WecomSender {
    fn send(&self, title: &str, message: &str, _payload: &Value) -> Result<(), String> {
        let body = json!({
            "msgtype": "text",
            "text": {
                "content": format!("{title}\n{message}")
            }
        });
        self.client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .and_then(|response| response.error_for_status())
            .map(|_| ())
            .map_err(|err| format!("wecom send failed: {err}"))
    }
}
