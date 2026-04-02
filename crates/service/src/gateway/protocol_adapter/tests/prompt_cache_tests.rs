use super::{PromptCache, PromptCacheConfig};
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

/// 函数 `new_test_cache`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - now: 参数 now
/// - config: 参数 config
///
/// # 返回
/// 返回函数执行结果
fn new_test_cache(now: Instant, config: PromptCacheConfig) -> PromptCache {
    PromptCache {
        by_key: HashMap::new(),
        lru_by_tick: BTreeMap::new(),
        tick: 0,
        last_cleanup: now,
        config,
    }
}

/// 函数 `lru_capacity_evicts_least_recently_seen`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn lru_capacity_evicts_least_recently_seen() {
    let now = Instant::now();
    let mut cache = new_test_cache(
        now,
        PromptCacheConfig {
            ttl: Duration::ZERO,
            cleanup_interval: Duration::from_secs(3600),
            capacity: 2,
        },
    );

    let id1 = cache.get_or_create("k1", now);
    let id2 = cache.get_or_create("k2", now);
    assert_eq!(cache.by_key.len(), 2);

    // Touch k1 so k2 becomes the LRU.
    assert_eq!(cache.get_or_create("k1", now + Duration::from_secs(1)), id1);

    let id3 = cache.get_or_create("k3", now + Duration::from_secs(2));
    assert_eq!(cache.by_key.len(), 2);
    assert!(cache.by_key.contains_key("k1"));
    assert!(cache.by_key.contains_key("k3"));
    assert!(!cache.by_key.contains_key("k2"));

    // k2 should have been evicted.
    let id2_new = cache.get_or_create("k2", now + Duration::from_secs(3));
    assert_ne!(id2_new, id2);
    assert_eq!(cache.get_or_create("k3", now + Duration::from_secs(3)), id3);
    assert_ne!(cache.get_or_create("k1", now + Duration::from_secs(3)), id1);
}

/// 函数 `ttl_expires_after_idle_and_is_checked_on_access`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn ttl_expires_after_idle_and_is_checked_on_access() {
    let now = Instant::now();
    let mut cache = new_test_cache(
        now,
        PromptCacheConfig {
            ttl: Duration::from_secs(10),
            cleanup_interval: Duration::from_secs(3600),
            capacity: 0,
        },
    );

    let id1 = cache.get_or_create("k1", now);

    // Within TTL: hit returns same id and refreshes last_seen.
    assert_eq!(cache.get_or_create("k1", now + Duration::from_secs(9)), id1);

    // Past TTL since last_seen: miss returns new id.
    let id2 = cache.get_or_create("k1", now + Duration::from_secs(21));
    assert_ne!(id2, id1);
}
