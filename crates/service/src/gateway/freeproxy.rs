use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use url::Url;

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
    pub sync_register_proxy_pool: Option<bool>,
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
    pub register_proxy_sync_enabled: bool,
    pub register_proxy_created_count: usize,
    pub register_proxy_updated_count: usize,
    pub register_proxy_total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FreeProxyClearResult {
    pub previous_proxy_list_value: String,
    pub previous_proxy_list_count: usize,
    pub cleared_gateway_proxy_count: usize,
    pub deleted_register_proxy_count: usize,
    pub failed_register_proxy_count: usize,
    pub remaining_register_proxy_count: usize,
}

#[derive(Debug, Clone)]
struct FreeProxySyncOptions {
    protocol: FreeProxyProtocol,
    anonymity: FreeProxyAnonymityPolicy,
    countries: Vec<String>,
    limit: usize,
    source_url: String,
    clear_upstream_proxy_url: bool,
    sync_register_proxy_pool: bool,
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

pub(crate) fn clear_proxy_pools() -> Result<FreeProxyClearResult, String> {
    crate::initialize_storage_if_needed()?;

    let previous_proxy_list_value = crate::app_settings::current_env_overrides()
        .get(PROXY_LIST_ENV_KEY)
        .cloned()
        .unwrap_or_default();
    let previous_proxy_list_count = previous_proxy_list_value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .count();

    let mut overrides = HashMap::new();
    overrides.insert(PROXY_LIST_ENV_KEY.to_string(), String::new());
    let _ = crate::app_settings::set_env_overrides(overrides)?;

    let existing = crate::account_register::list_register_proxies(None)?;
    let mut deleted_register_proxy_count = 0usize;
    let mut failed_register_proxy_count = 0usize;
    for item in existing.iter() {
        match crate::account_register::delete_register_proxy(item.id) {
            Ok(_) => deleted_register_proxy_count += 1,
            Err(_) => failed_register_proxy_count += 1,
        }
    }

    let remaining_register_proxy_count = crate::account_register::list_register_proxies(None)
        .map(|items| items.len())
        .unwrap_or(failed_register_proxy_count);

    let result = FreeProxyClearResult {
        previous_proxy_list_value,
        previous_proxy_list_count,
        cleared_gateway_proxy_count: previous_proxy_list_count,
        deleted_register_proxy_count,
        failed_register_proxy_count,
        remaining_register_proxy_count,
    };

    crate::operation_audit::record_operation_audit(
        "freeproxy_clear",
        "清空代理池",
        format!(
            "网关代理池清空 {} 个，注册代理池删除 {} 个，失败 {} 个，剩余 {} 个",
            result.cleared_gateway_proxy_count,
            result.deleted_register_proxy_count,
            result.failed_register_proxy_count,
            result.remaining_register_proxy_count
        ),
    );

    Ok(result)
}

fn apply_freeproxy_catalog(
    options: FreeProxySyncOptions,
    catalog: FreeProxyCatalog,
) -> Result<FreeProxySyncResult, String> {
    let proxies = select_freeproxy_proxies(&catalog.data, &options);
    if proxies.is_empty() {
        return Err("freeproxy 未找到符合条件的可用代理".to_string());
    }

    let register_proxy_sync = if options.sync_register_proxy_pool {
        sync_register_proxy_pool(&proxies)?
    } else {
        RegisterProxySyncSummary::default()
    };

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

    let result = FreeProxySyncResult {
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
        register_proxy_sync_enabled: options.sync_register_proxy_pool,
        register_proxy_created_count: register_proxy_sync.created_count,
        register_proxy_updated_count: register_proxy_sync.updated_count,
        register_proxy_total_count: register_proxy_sync.total_count,
    };
    crate::operation_audit::record_operation_audit(
        "freeproxy_sync",
        "同步 freeproxy 代理池",
        format!(
            "抓取 {} 个，匹配 {} 个，应用 {} 个，注册代理池总数 {}",
            result.fetched_count,
            result.matched_count,
            result.applied_count,
            result.register_proxy_total_count
        ),
    );
    Ok(result)
}

#[derive(Debug, Clone, Default)]
struct RegisterProxySyncSummary {
    created_count: usize,
    updated_count: usize,
    total_count: usize,
}

#[derive(Debug, Clone)]
struct RegisterProxyCandidate {
    key: String,
    name: String,
    proxy_type: String,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    priority: i64,
}

fn sync_register_proxy_pool(proxies: &[String]) -> Result<RegisterProxySyncSummary, String> {
    let candidates = build_register_proxy_candidates(proxies)?;
    if candidates.is_empty() {
        return Ok(RegisterProxySyncSummary::default());
    }

    let existing = crate::account_register::list_register_proxies(None)?;
    let mut existing_by_key = HashMap::new();
    for item in existing {
        existing_by_key
            .entry(register_proxy_key_from_item(&item))
            .or_insert(item);
    }

    let mut created_count = 0;
    let mut updated_count = 0;
    for candidate in candidates.iter() {
        if let Some(existing) = existing_by_key.get(candidate.key.as_str()) {
            let needs_update = !existing.enabled
                || existing.name != candidate.name
                || existing.priority != candidate.priority;
            if needs_update {
                let _ = crate::account_register::update_register_proxy(
                    existing.id,
                    Some(candidate.name.as_str()),
                    Some(true),
                    Some(candidate.priority),
                )?;
                updated_count += 1;
            }
            continue;
        }

        let _ = crate::account_register::create_register_proxy(
            crate::account_register::CreateRegisterProxyInput {
                name: candidate.name.as_str(),
                proxy_type: candidate.proxy_type.as_str(),
                host: candidate.host.as_str(),
                port: candidate.port,
                username: candidate.username.as_deref(),
                password: candidate.password.as_deref(),
                enabled: true,
                priority: candidate.priority,
            },
        )?;
        created_count += 1;
    }

    Ok(RegisterProxySyncSummary {
        created_count,
        updated_count,
        total_count: candidates.len(),
    })
}

fn build_register_proxy_candidates(
    proxies: &[String],
) -> Result<Vec<RegisterProxyCandidate>, String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let total = proxies.len();
    for (index, proxy_url) in proxies.iter().enumerate() {
        let candidate = parse_register_proxy_candidate(proxy_url, index, total)?;
        if seen.insert(candidate.key.clone()) {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

fn parse_register_proxy_candidate(
    proxy_url: &str,
    index: usize,
    total: usize,
) -> Result<RegisterProxyCandidate, String> {
    let parsed = Url::parse(proxy_url)
        .map_err(|err| format!("解析 freeproxy 代理 URL 失败: {proxy_url}: {err}"))?;
    let host = parsed
        .host_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("freeproxy 代理缺少 host: {proxy_url}"))?
        .to_string();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| format!("freeproxy 代理缺少端口: {proxy_url}"))?;
    let username = (!parsed.username().trim().is_empty()).then(|| parsed.username().to_string());
    let password = parsed.password().map(ToString::to_string);
    let proxy_type = normalize_register_proxy_type(parsed.scheme());
    let key = register_proxy_key(
        proxy_type.as_str(),
        host.as_str(),
        port,
        username.as_deref(),
    );

    Ok(RegisterProxyCandidate {
        key,
        name: format!("freeproxy-{}-{}-{}:{}", index + 1, proxy_type, host, port),
        proxy_type,
        host,
        port,
        username,
        password,
        priority: (total.saturating_sub(index)) as i64,
    })
}

fn normalize_register_proxy_type(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "socks" | "socks5h" => "socks5".to_string(),
        other => other.to_string(),
    }
}

fn register_proxy_key(proxy_type: &str, host: &str, port: u16, username: Option<&str>) -> String {
    format!(
        "{}://{}@{}:{}",
        normalize_register_proxy_type(proxy_type),
        username.unwrap_or("").trim(),
        host.trim().to_ascii_lowercase(),
        port
    )
}

fn register_proxy_key_from_item(item: &crate::account_register::RegisterProxyItem) -> String {
    register_proxy_key(
        item.proxy_type.as_str(),
        item.host.as_str(),
        item.port,
        item.username.as_deref(),
    )
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
    let sync_register_proxy_pool = input.sync_register_proxy_pool.unwrap_or(true);

    Ok(FreeProxySyncOptions {
        protocol,
        anonymity,
        countries,
        limit,
        source_url,
        clear_upstream_proxy_url,
        sync_register_proxy_pool,
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
        .split([',', ';', '|', ' ', '\n', '\r', '\t'])
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

    matched.sort_by(compare_entry_rank);

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
            sync_register_proxy_pool: false,
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
            sync_register_proxy_pool: false,
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
            sync_register_proxy_pool: false,
        };

        let error = apply_freeproxy_catalog(options, sample_catalog()).expect_err("empty matches");
        assert!(error.contains("未找到符合条件"));
    }

    #[test]
    fn build_register_proxy_candidates_normalizes_and_deduplicates() {
        let candidates = build_register_proxy_candidates(&[
            "socks5h://1.1.1.1:1080".to_string(),
            "socks5://1.1.1.1:1080".to_string(),
            "https://user:pass@2.2.2.2:443".to_string(),
        ])
        .expect("candidates");

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].proxy_type, "socks5");
        assert_eq!(candidates[0].port, 1080);
        assert_eq!(candidates[1].username.as_deref(), Some("user"));
        assert_eq!(candidates[1].password.as_deref(), Some("pass"));
    }

    #[test]
    fn freeproxy_clear_result_serializes_camel_case_keys() {
        let value = serde_json::to_value(FreeProxyClearResult {
            previous_proxy_list_value: "a,b".to_string(),
            previous_proxy_list_count: 2,
            cleared_gateway_proxy_count: 2,
            deleted_register_proxy_count: 5,
            failed_register_proxy_count: 1,
            remaining_register_proxy_count: 0,
        })
        .expect("serialize clear result");

        assert_eq!(
            value.get("clearedGatewayProxyCount").and_then(serde_json::Value::as_u64),
            Some(2)
        );
        assert_eq!(
            value.get("remainingRegisterProxyCount").and_then(serde_json::Value::as_u64),
            Some(0)
        );
    }
}
