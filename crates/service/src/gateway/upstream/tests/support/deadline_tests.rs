use super::*;

/// 函数 `effective_request_timeout_non_stream_uses_total_only`
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
fn effective_request_timeout_non_stream_uses_total_only() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            false
        ),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), false),
        None
    );
}

/// 函数 `effective_request_timeout_stream_uses_max_total_and_stream`
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
fn effective_request_timeout_stream_uses_max_total_and_stream() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            true
        ),
        Some(Duration::from_secs(300))
    );
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(300)),
            Some(Duration::from_secs(120)),
            true
        ),
        Some(Duration::from_secs(300))
    );
}

/// 函数 `effective_request_timeout_stream_falls_back_when_one_side_missing`
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
fn effective_request_timeout_stream_falls_back_when_one_side_missing() {
    assert_eq!(
        effective_request_timeout(Some(Duration::from_secs(120)), None, true),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), true),
        Some(Duration::from_secs(300))
    );
    assert_eq!(effective_request_timeout(None, None, true), None);
}
