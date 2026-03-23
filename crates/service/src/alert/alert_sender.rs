use codexmanager_core::storage::AlertChannel;
use reqwest::blocking::Client;
use serde_json::{json, Value};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::mpsc::Sender;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};
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

#[cfg(test)]
fn test_webhook_registry() -> &'static Mutex<HashMap<String, Sender<String>>> {
    static TEST_WEBHOOKS: OnceLock<Mutex<HashMap<String, Sender<String>>>> = OnceLock::new();
    TEST_WEBHOOKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
pub(crate) fn alert_sender_test_guard() -> MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    crate::lock_utils::lock_recover(
        TEST_MUTEX.get_or_init(|| Mutex::new(())),
        "alert sender test mutex",
    )
}

#[cfg(test)]
pub(crate) fn register_test_webhook(url: &str, sender: Sender<String>) {
    register_test_http("POST", url, sender);
}

#[cfg(test)]
pub(crate) fn unregister_test_webhook(url: &str) {
    unregister_test_http("POST", url);
}

#[cfg(test)]
pub(crate) fn register_test_http(method: &str, url: &str, sender: Sender<String>) {
    let mut registry =
        crate::lock_utils::lock_recover(test_webhook_registry(), "test webhook registry");
    registry.insert(test_http_registry_key(method, url), sender);
}

#[cfg(test)]
pub(crate) fn unregister_test_http(method: &str, url: &str) {
    let mut registry =
        crate::lock_utils::lock_recover(test_webhook_registry(), "test webhook registry");
    registry.remove(&test_http_registry_key(method, url));
}

#[cfg(test)]
fn test_http_registry_key(method: &str, url: &str) -> String {
    format!("{} {}", method.trim().to_ascii_uppercase(), url)
}

#[cfg(test)]
fn dispatch_test_http(method: &str, url: &str, body: Option<&Value>) -> Result<bool, String> {
    let sender = {
        let registry =
            crate::lock_utils::lock_recover(test_webhook_registry(), "test webhook registry");
        registry.get(&test_http_registry_key(method, url)).cloned()
    };
    match sender {
        Some(sender) => sender
            .send(
                json!({
                    "method": method.trim().to_ascii_uppercase(),
                    "url": url,
                    "body": body.cloned(),
                })
                .to_string(),
            )
            .map(|_| true)
            .map_err(|err| format!("test webhook send failed: {err}")),
        None => Ok(false),
    }
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
        #[cfg(test)]
        if dispatch_test_http("POST", &self.url, Some(&body))? {
            return Ok(());
        }
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
        #[cfg(test)]
        if dispatch_test_http("GET", &target, None)? {
            return Ok(());
        }
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
        #[cfg(test)]
        if dispatch_test_http("POST", &url, Some(&body))? {
            return Ok(());
        }
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
        #[cfg(test)]
        if dispatch_test_http("POST", &self.webhook_url, Some(&body))? {
            return Ok(());
        }
        self.client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .and_then(|response| response.error_for_status())
            .map(|_| ())
            .map_err(|err| format!("wecom send failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codexmanager_core::storage::{now_ts, AlertChannel};
    use std::sync::mpsc::{channel, RecvTimeoutError};

    struct TestHttpRegistration {
        method: &'static str,
        url: String,
    }

    impl TestHttpRegistration {
        fn new(method: &'static str, url: impl Into<String>, sender: Sender<String>) -> Self {
            let url = url.into();
            register_test_http(method, &url, sender);
            Self { method, url }
        }
    }

    impl Drop for TestHttpRegistration {
        fn drop(&mut self) {
            unregister_test_http(self.method, &self.url);
        }
    }

    fn test_channel(channel_type: &str, config: Value) -> AlertChannel {
        let now = now_ts();
        AlertChannel {
            id: format!("channel-{channel_type}"),
            name: format!("{channel_type}-channel"),
            channel_type: channel_type.to_string(),
            config_json: config.to_string(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn recv_request_json(receiver: &std::sync::mpsc::Receiver<String>) -> Value {
        let payload = receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_else(|err| match err {
                RecvTimeoutError::Timeout => panic!("timed out waiting for mocked alert request"),
                RecvTimeoutError::Disconnected => {
                    panic!("mocked alert request channel disconnected")
                }
            });
        serde_json::from_str(&payload).expect("parse mocked request payload")
    }

    #[test]
    fn send_test_alert_supports_bark_telegram_and_wecom_mock_transports() {
        let _guard = alert_sender_test_guard();

        let (bark_tx, bark_rx) = channel::<String>();
        let bark_url = format!(
            "mock://bark/{}/{}",
            urlencoding::encode("CodexManager 告警测试"),
            urlencoding::encode("这是一条测试通知，用于验证当前告警渠道配置。")
        );
        let _bark_registration = TestHttpRegistration::new("GET", bark_url.clone(), bark_tx);
        send_test_alert(
            &test_channel("bark", json!({ "url": "mock://bark" })),
            &json!({ "kind": "bark" }),
        )
        .expect("send bark test alert");
        let bark_request = recv_request_json(&bark_rx);
        assert_eq!(
            bark_request.get("method").and_then(Value::as_str),
            Some("GET")
        );
        assert_eq!(
            bark_request.get("url").and_then(Value::as_str),
            Some(bark_url.as_str())
        );
        assert!(bark_request.get("body").is_some_and(Value::is_null));

        let (telegram_tx, telegram_rx) = channel::<String>();
        let telegram_url = "https://api.telegram.org/botbot-token/sendMessage";
        let _telegram_registration = TestHttpRegistration::new("POST", telegram_url, telegram_tx);
        send_test_alert(
            &test_channel(
                "telegram",
                json!({
                    "botToken": "bot-token",
                    "chatId": "chat-42",
                }),
            ),
            &json!({ "kind": "telegram" }),
        )
        .expect("send telegram test alert");
        let telegram_request = recv_request_json(&telegram_rx);
        assert_eq!(
            telegram_request.get("method").and_then(Value::as_str),
            Some("POST")
        );
        assert_eq!(
            telegram_request.get("url").and_then(Value::as_str),
            Some(telegram_url)
        );
        assert_eq!(
            telegram_request
                .get("body")
                .and_then(|body| body.get("chat_id"))
                .and_then(Value::as_str),
            Some("chat-42")
        );
        assert!(telegram_request
            .get("body")
            .and_then(|body| body.get("text"))
            .and_then(Value::as_str)
            .is_some_and(|text| {
                text.contains("CodexManager 告警测试")
                    && text.contains("这是一条测试通知，用于验证当前告警渠道配置。")
            }));

        let (wecom_tx, wecom_rx) = channel::<String>();
        let wecom_url = "mock://wecom-bot";
        let _wecom_registration = TestHttpRegistration::new("POST", wecom_url, wecom_tx);
        send_test_alert(
            &test_channel("wecom", json!({ "webhookUrl": wecom_url })),
            &json!({ "kind": "wecom" }),
        )
        .expect("send wecom test alert");
        let wecom_request = recv_request_json(&wecom_rx);
        assert_eq!(
            wecom_request.get("method").and_then(Value::as_str),
            Some("POST")
        );
        assert_eq!(
            wecom_request.get("url").and_then(Value::as_str),
            Some(wecom_url)
        );
        assert_eq!(
            wecom_request
                .get("body")
                .and_then(|body| body.get("msgtype"))
                .and_then(Value::as_str),
            Some("text")
        );
        assert!(wecom_request
            .get("body")
            .and_then(|body| body.get("text"))
            .and_then(|text| text.get("content"))
            .and_then(Value::as_str)
            .is_some_and(|content| {
                content.contains("CodexManager 告警测试")
                    && content.contains("这是一条测试通知，用于验证当前告警渠道配置。")
            }));
    }
}
