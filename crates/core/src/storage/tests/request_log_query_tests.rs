use super::{parse_request_log_query, RequestLogQuery};

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
