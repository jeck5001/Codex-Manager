use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, COOKIE, USER_AGENT};
use reqwest::Proxy;
use std::time::Duration;

use super::{RegisterEmailProvider, RegisterMailboxLease};

const DEFAULT_GENERATOR_EMAIL_BASE_URL: &str = "https://generator.email";
const GENERATOR_EMAIL_TIMEOUT_SECS: u64 = 30;
const GENERATOR_EMAIL_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36";
const GENERATOR_EMAIL_ACCEPT: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";

pub(crate) struct GeneratorEmailProvider {
    client: Client,
    base_url: String,
}

impl GeneratorEmailProvider {
    pub(crate) fn new() -> Result<Self, String> {
        Self::new_with_proxy(None)
    }

    pub(crate) fn new_with_proxy(proxy: Option<&str>) -> Result<Self, String> {
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(GENERATOR_EMAIL_TIMEOUT_SECS));
        if let Some(proxy_url) = normalize_proxy_url(proxy) {
            builder = builder.proxy(
                Proxy::all(proxy_url.as_str())
                    .map_err(|err| format!("build generator.email proxy failed: {err}"))?,
            );
        }
        let client = builder
            .build()
            .map_err(|err| format!("build generator.email client failed: {err}"))?;
        Ok(Self {
            client,
            base_url: DEFAULT_GENERATOR_EMAIL_BASE_URL.to_string(),
        })
    }
}

impl RegisterEmailProvider for GeneratorEmailProvider {
    fn create_mailbox(&self) -> Result<RegisterMailboxLease, String> {
        let html = self
            .client
            .get(self.base_url.as_str())
            .header(USER_AGENT, GENERATOR_EMAIL_USER_AGENT)
            .header(ACCEPT, GENERATOR_EMAIL_ACCEPT)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|err| format!("request generator.email homepage failed: {err}"))?
            .text()
            .map_err(|err| format!("read generator.email homepage failed: {err}"))?;
        let email = parse_generator_email_address(html.as_str())
            .ok_or_else(|| "parse generator.email address failed".to_string())?;
        let credential = build_generator_email_surl(email.as_str())
            .ok_or_else(|| "build generator.email surl failed".to_string())?;
        Ok(RegisterMailboxLease { email, credential })
    }

    fn fetch_code(&self, credential: &str) -> Result<Option<String>, String> {
        if credential.trim().is_empty() {
            return Ok(None);
        }

        let mailbox_url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            credential.trim_start_matches('/')
        );
        let html = self
            .client
            .get(mailbox_url)
            .header(USER_AGENT, GENERATOR_EMAIL_USER_AGENT)
            .header(ACCEPT, GENERATOR_EMAIL_ACCEPT)
            .header(COOKIE, format!("surl={}", credential.trim()))
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|err| format!("request generator.email mailbox failed: {err}"))?
            .text()
            .map_err(|err| format!("read generator.email mailbox failed: {err}"))?;
        Ok(extract_generator_email_code(html.as_str()))
    }
}

pub(crate) fn parse_generator_email_address(html: &str) -> Option<String> {
    if html.trim().is_empty() {
        return None;
    }

    extract_text_by_id(html, "email_ch_text").or_else(|| {
        let username = extract_input_value_by_id(html, "userName")?;
        let domain = extract_input_value_by_id(html, "domainName2")?;
        Some(format!("{}@{}", username.trim(), domain.trim()))
    })
}

pub(crate) fn build_generator_email_surl(email: &str) -> Option<String> {
    let (username, domain) = email.trim().split_once('@')?;
    let safe_user = username
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase();
    let safe_domain = domain.trim().to_ascii_lowercase();
    if safe_user.is_empty() || safe_domain.is_empty() {
        return None;
    }
    Some(format!("{safe_domain}/{safe_user}"))
}

pub(crate) fn extract_generator_email_code(html: &str) -> Option<String> {
    if html.trim().is_empty() {
        return None;
    }

    let lower = html.to_ascii_lowercase();
    if let Some(start) = lower.find("your chatgpt code is") {
        if let Some(code) = first_six_digit_code(&html[start..]) {
            return Some(code);
        }
    }

    for keyword in ["openai", "chatgpt"] {
        let mut cursor = 0usize;
        while let Some(offset) = lower[cursor..].find(keyword) {
            let start = cursor + offset;
            let end = (start + 200).min(html.len());
            if let Some(code) = first_six_digit_code(&html[start..end]) {
                return Some(code);
            }
            cursor = start + keyword.len();
        }
    }

    if lower.contains("openai") || lower.contains("chatgpt") {
        return first_six_digit_code(html);
    }

    None
}

fn extract_text_by_id(html: &str, id: &str) -> Option<String> {
    let marker = format!("id=\"{id}\"");
    let start = html.find(marker.as_str())?;
    let fragment = &html[start..];
    let gt = fragment.find('>')?;
    let content = &fragment[gt + 1..];
    let end = content.find('<')?;
    let value = content[..end].trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn extract_input_value_by_id(html: &str, id: &str) -> Option<String> {
    let marker = format!("id=\"{id}\"");
    let start = html.find(marker.as_str())?;
    let fragment = &html[start..];
    let value_marker = "value=\"";
    let value_start = fragment.find(value_marker)? + value_marker.len();
    let remainder = &fragment[value_start..];
    let value_end = remainder.find('"')?;
    let value = remainder[..value_end].trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn first_six_digit_code(text: &str) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    for window in chars.windows(6) {
        if window.iter().all(|ch| ch.is_ascii_digit()) {
            return Some(window.iter().collect::<String>());
        }
    }
    None
}

fn normalize_proxy_url(proxy: Option<&str>) -> Option<String> {
    let raw = proxy?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains("://") {
        return Some(raw.to_string());
    }
    Some(format!("http://{raw}"))
}

#[cfg(test)]
pub(crate) fn normalize_proxy_url_for_test(proxy: Option<&str>) -> Option<String> {
    normalize_proxy_url(proxy)
}
