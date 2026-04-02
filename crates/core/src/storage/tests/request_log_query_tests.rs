use super::{parse_request_log_query, RequestLogQuery};

/// 函数 `prefixed_field_query_supports_exact_mode`
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
fn prefixed_field_query_supports_exact_mode() {
    let query = parse_request_log_query(Some("method:=POST"));
    assert!(matches!(
        query,
        RequestLogQuery::FieldExact {
            column: "method",
            value
        } if value == "POST"
    ));
}

/// 函数 `prefixed_field_query_keeps_like_mode_by_default`
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
fn prefixed_field_query_keeps_like_mode_by_default() {
    let query = parse_request_log_query(Some("key:key-alpha"));
    assert!(matches!(
        query,
        RequestLogQuery::FieldLike {
            column: "key_id",
            pattern
        } if pattern == "%key-alpha%"
    ));
}

/// 函数 `prefixed_account_query_supports_alias`
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
fn prefixed_account_query_supports_alias() {
    let query = parse_request_log_query(Some("account:acc-1"));
    assert!(matches!(
        query,
        RequestLogQuery::FieldLike {
            column: "account_id",
            pattern
        } if pattern == "%acc-1%"
    ));
}
