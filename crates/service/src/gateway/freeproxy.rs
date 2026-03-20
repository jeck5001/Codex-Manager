use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_FREEPROXY_SOURCE_URL: &str =
    "https://raw.githubusercontent.com/CharlesPikachu/freeproxy/master/proxies.json";
const DEFAULT_FREEPROXY_LIMIT: usize = 20;
const MAX_FREEPROXY_LIMIT: usize = 100;
const FREEPROXY_FETCH_TIMEOUT_SECS: u64 = 20;
const PROXY_LIST_ENV_KEY: &str = "CODEXMANAGER_PROXY_LIST";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FreeProxySyncInput {
    pub protocol: Option<String>,
    pub anonymity: Option<String>,
    pub country: Option<String>,
    pub limit: Option<usize>,
    pub source_url: Option<String>,
    pub clear_upstream_proxy_url: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FreeProxySyncResult {
    pub source_url: String,
    pub source_updated_at: Option<String>,
    pub fetched_count: usize,
    pub matched_count: usize,
    pub applied_count: usize,
    pub protocol: String,
    pub anonymity: String,
    pub country_filter: Vec<String>,
    pub limit: usize,
    pub cleared_upstream_proxy_url: bool,
    pub single_proxy_still_configured: bool,
    pub previous_upstream_proxy_url: Option<String>,
    pub proxy_list_value: String,
    pub proxies: Vec<String>,
}

#[derive(Debug, Clone)]
struct FreeProxySyncOptions {
    protocol: FreeProxyProtocol,
    anonymity: FreeProxyAnonymityPolicy,
    countries: Vec<String>,
    limit: usize,
    source_url: String,
    clear_upstream_proxy_url: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreeProxyProtocol {
    Auto,
    Socks5,
    Https,
    Http,
}

impl FreeProxyProtocol {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Socks5 => "socks5",
            Self::Https => "https",
            Self::Http => "http",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreeProxyAnonymityPolicy {
    Elite,
    AnonymousOrElite,
    All,
}

impl FreeProxyAnonymityPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Elite => "elite",
            Self::AnonymousOrElite => "anonymous_or_elite",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct FreeProxyCatalog {
    updated_at: Option<String>,
    #[allow(dead_code)]
    count: Option<usize>,
    #[serde(default)]
    data: Vec<FreeProxyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct FreeProxyEntry {
    ip: String,
    port: u16,
    protocol: String,
    country: Option<String>,
    anonymity: Option<String>,
    speed: Option<f64>,
}

pub(crate) fn sync_proxy_pool_from_freeproxy(
    input: FreeProxySyncInput,
) -> Result<FreeProxySyncResult, String> {
    crate::initialize_storage_if_needed()?;
    let options = normalize_sync_options(input)?;
    let catalog = fetch_freeproxy_catalog(options.source_url.as_str())?;
    apply_freeproxy_catalog(options, catalog)
}

fn apply_freeproxy_catalog(
    options: FreeProxySyncOptions,
    catalog: FreeProxyCatalog,
) -> Result<FreeProxySyncResult, String> {
    let proxies = select_freeproxy_proxies(&catalog.data, &options);
    if proxies.is_empty() {
        return Err("freeproxy 未找到符合条件的可用代理".to_string());
    }

    let proxy_list_value = proxies.join(",");
    let previous_upstream_proxy_url = crate::gateway::current_upstream_proxy_url();
    let mut overrides = HashMap::new();
    overrides.insert(PROXY_LIST_ENV_KEY.to_string(), proxy_list_value.clone());
    let _ = crate::app_settings::set_env_overrides(overrides)?;

    let cleared_upstream_proxy_url = if options.clear_upstream_proxy_url {
        let _ = crate::set_gateway_upstream_proxy_url(None)?;
        true
    } else {
        false
    };

    Ok(FreeProxySyncResult {
        source_url: options.source_url,
        source_updated_at: catalog.updated_at,
        fetched_count: catalog.data.len(),
        matched_count: proxies.len(),
        applied_count: proxies.len(),
        protocol: options.protocol.as_str().to_string(),
        anonymity: options.anonymity.as_str().to_string(),
        country_filter: options.countries,
        limit: options.limit,
        cleared_upstream_proxy_url,
        single_proxy_still_configured: previous_upstream_proxy_url.is_some()
            && !cleared_upstream_proxy_url,
        previous_upstream_proxy_url,
        proxy_list_value,
        proxies,
    })
}

fn fetch_freeproxy_catalog(source_url: &str) -> Result<FreeProxyCatalog, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(FREEPROXY_FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("创建 freeproxy 客户端失败: {err}"))?;
    let response = client
        .get(source_url)
        .send()
        .map_err(|err| format!("拉取 freeproxy 代理列表失败: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "拉取 freeproxy 代理列表失败: HTTP {}",
            response.status()
        ));
    }
    response
        .json::<FreeProxyCatalog>()
        .map_err(|err| format!("解析 freeproxy 代理列表失败: {err}"))
}

fn normalize_sync_options(input: FreeProxySyncInput) -> Result<FreeProxySyncOptions, String> {
    let protocol = parse_sync_protocol(input.protocol.as_deref())?;
    let anonymity = parse_anonymity_policy(input.anonymity.as_deref())?;
    let countries = parse_country_filters(input.country.as_deref());
    let limit = input
        .limit
        .unwrap_or(DEFAULT_FREEPROXY_LIMIT)
        .clamp(1, MAX_FREEPROXY_LIMIT);
    let source_url = input
        .source_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_FREEPROXY_SOURCE_URL.to_string());
    let clear_upstream_proxy_url = input.clear_upstream_proxy_url.unwrap_or(true);

    Ok(FreeProxySyncOptions {
        protocol,
        anonymity,
        countries,
        limit,
        source_url,
        clear_upstream_proxy_url,
    })
}

fn parse_sync_protocol(raw: Option<&str>) -> Result<FreeProxyProtocol, String> {
    match raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("socks5")
        .to_ascii_lowercase()
        .as_str()
    {
        "auto" => Ok(FreeProxyProtocol::Auto),
        "socks5" | "socks5h" | "socks" => Ok(FreeProxyProtocol::Socks5),
        "https" => Ok(FreeProxyProtocol::Https),
        "http" => Ok(FreeProxyProtocol::Http),
        _ => Err("protocol 只支持 auto/http/https/socks5".to_string()),
    }
}

fn parse_anonymity_policy(raw: Option<&str>) -> Result<FreeProxyAnonymityPolicy, String> {
    match raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("elite")
        .to_ascii_lowercase()
        .as_str()
    {
        "elite" => Ok(FreeProxyAnonymityPolicy::Elite),
        "anonymous_or_elite" | "anonymous-elite" | "anonymousorelites" | "anonymous" => {
            Ok(FreeProxyAnonymityPolicy::AnonymousOrElite)
        }
        "all" => Ok(FreeProxyAnonymityPolicy::All),
        _ => Err("anonymity 只支持 elite/anonymous_or_elite/all".to_string()),
    }
}

fn parse_country_filters(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(|ch: char| matches!(ch, ',' | ';' | '|' | ' ' | '\n' | '\r' | '\t'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
        .collect::<Vec<_>>()
}

fn select_freeproxy_proxies(
    entries: &[FreeProxyEntry],
    options: &FreeProxySyncOptions,
) -> Vec<String> {
    let mut matched = entries
        .iter()
        .filter_map(|entry| match_freeproxy_entry(entry, options))
        .collect::<Vec<_>>();

    matched.sort_by(|left, right| compare_entry_rank(left, right));

    let mut deduped = Vec::new();
    for candidate in matched {
        if deduped
            .iter()
            .any(|item: &MatchedProxy| item.proxy_url == candidate.proxy_url)
        {
            continue;
        }
        deduped.push(candidate);
        if deduped.len() >= options.limit {
            break;
        }
    }

    deduped
        .into_iter()
        .map(|item| item.proxy_url)
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone)]
struct MatchedProxy {
    proxy_url: String,
    speed: Option<f64>,
    protocol_rank: u8,
}

fn match_freeproxy_entry(
    entry: &FreeProxyEntry,
    options: &FreeProxySyncOptions,
) -> Option<MatchedProxy> {
    if !country_matches(entry, options) || !anonymity_matches(entry, options) {
        return None;
    }

    let (scheme, protocol_rank) = resolve_entry_scheme(entry.protocol.as_str(), options.protocol)?;
    Some(MatchedProxy {
        proxy_url: format!("{scheme}://{}:{}", entry.ip.trim(), entry.port),
        speed: entry.speed,
        protocol_rank,
    })
}

fn country_matches(entry: &FreeProxyEntry, options: &FreeProxySyncOptions) -> bool {
    if options.countries.is_empty() {
        return true;
    }
    let country = entry
        .country
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());
    country
        .as_deref()
        .is_some_and(|value| options.countries.iter().any(|country| country == value))
}

fn anonymity_matches(entry: &FreeProxyEntry, options: &FreeProxySyncOptions) -> bool {
    let normalized = entry
        .anonymity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    match options.anonymity {
        FreeProxyAnonymityPolicy::Elite => normalized == "elite",
        FreeProxyAnonymityPolicy::AnonymousOrElite => {
            normalized == "elite" || normalized == "anonymous"
        }
        FreeProxyAnonymityPolicy::All => true,
    }
}

fn resolve_entry_scheme(
    raw_protocols: &str,
    preferred: FreeProxyProtocol,
) -> Option<(&'static str, u8)> {
    let protocols = raw_protocols
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let supports = |target: &str| protocols.iter().any(|item| item == target);

    match preferred {
        FreeProxyProtocol::Auto => {
            if supports("socks5") {
                Some(("socks5", 0))
            } else if supports("https") {
                Some(("https", 1))
            } else if supports("http") {
                Some(("http", 2))
            } else {
                None
            }
        }
        FreeProxyProtocol::Socks5 => supports("socks5").then_some(("socks5", 0)),
        FreeProxyProtocol::Https => supports("https").then_some(("https", 1)),
        FreeProxyProtocol::Http => supports("http").then_some(("http", 2)),
    }
}

fn compare_entry_rank(left: &MatchedProxy, right: &MatchedProxy) -> Ordering {
    match compare_speed(left.speed, right.speed) {
        Ordering::Equal => match left.protocol_rank.cmp(&right.protocol_rank) {
            Ordering::Equal => left.proxy_url.cmp(&right.proxy_url),
            other => other,
        },
        other => other,
    }
}

fn compare_speed(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> FreeProxyCatalog {
        FreeProxyCatalog {
            updated_at: Some("2026-03-20 04:51:48 UTC".to_string()),
            count: Some(4),
            data: vec![
                FreeProxyEntry {
                    ip: "1.1.1.1".to_string(),
                    port: 1080,
                    protocol: "Http, Https, Socks5".to_string(),
                    country: Some("FR".to_string()),
                    anonymity: Some("Elite".to_string()),
                    speed: Some(39.0),
                },
                FreeProxyEntry {
                    ip: "2.2.2.2".to_string(),
                    port: 443,
                    protocol: "Https".to_string(),
                    country: Some("US".to_string()),
                    anonymity: Some("Anonymous".to_string()),
                    speed: Some(20.0),
                },
                FreeProxyEntry {
                    ip: "3.3.3.3".to_string(),
                    port: 8080,
                    protocol: "Http".to_string(),
                    country: Some("US".to_string()),
                    anonymity: Some("Transparent".to_string()),
                    speed: Some(10.0),
                },
                FreeProxyEntry {
                    ip: "4.4.4.4".to_string(),
                    port: 2080,
                    protocol: "Http, Socks5".to_string(),
                    country: Some("US".to_string()),
                    anonymity: Some("Elite".to_string()),
                    speed: Some(15.0),
                },
            ],
        }
    }

    #[test]
    fn select_freeproxy_proxies_prefers_requested_protocol_and_filters_fields() {
        let options = FreeProxySyncOptions {
            protocol: FreeProxyProtocol::Socks5,
            anonymity: FreeProxyAnonymityPolicy::Elite,
            countries: vec!["US".to_string()],
            limit: 10,
            source_url: DEFAULT_FREEPROXY_SOURCE_URL.to_string(),
            clear_upstream_proxy_url: true,
        };

        let proxies = select_freeproxy_proxies(&sample_catalog().data, &options);

        assert_eq!(proxies, vec!["socks5://4.4.4.4:2080"]);
    }

    #[test]
    fn select_freeproxy_proxies_auto_prefers_socks5_then_https_then_http() {
        let options = FreeProxySyncOptions {
            protocol: FreeProxyProtocol::Auto,
            anonymity: FreeProxyAnonymityPolicy::AnonymousOrElite,
            countries: Vec::new(),
            limit: 10,
            source_url: DEFAULT_FREEPROXY_SOURCE_URL.to_string(),
            clear_upstream_proxy_url: true,
        };

        let proxies = select_freeproxy_proxies(&sample_catalog().data, &options);

        assert_eq!(
            proxies,
            vec![
                "socks5://4.4.4.4:2080",
                "https://2.2.2.2:443",
                "socks5://1.1.1.1:1080",
            ]
        );
    }

    #[test]
    fn apply_freeproxy_catalog_rejects_empty_matches() {
        let options = FreeProxySyncOptions {
            protocol: FreeProxyProtocol::Socks5,
            anonymity: FreeProxyAnonymityPolicy::Elite,
            countries: vec!["CN".to_string()],
            limit: 10,
            source_url: DEFAULT_FREEPROXY_SOURCE_URL.to_string(),
            clear_upstream_proxy_url: true,
        };

        let error = apply_freeproxy_catalog(options, sample_catalog()).expect_err("empty matches");
        assert!(error.contains("未找到符合条件"));
    }
}
