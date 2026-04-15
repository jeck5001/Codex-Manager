use super::{
    extract_id_token_claims_for_test, generate_register_oauth_start_for_test,
    parse_register_callback_for_test,
};

#[test]
fn register_http_parses_callback_query_and_fragment() {
    let parsed =
        parse_register_callback_for_test("http://localhost:1455/auth/callback?code=abc&state=xyz");
    assert_eq!(parsed.code, "abc");
    assert_eq!(parsed.state, "xyz");
}

#[test]
fn register_http_builds_oauth_start_with_pkce() {
    let start = generate_register_oauth_start_for_test();
    assert!(start.auth_url.contains("code_challenge="));
    assert!(!start.state.is_empty());
    assert!(!start.code_verifier.is_empty());
}

#[test]
fn register_http_extracts_auth_claims_from_id_token_payload() {
    let claims = extract_id_token_claims_for_test(
        "header.eyJlbWFpbCI6InVzZXJAZXhhbXBsZS5jb20iLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjLTEifX0.sig",
    );
    assert_eq!(claims.email.as_deref(), Some("user@example.com"));
    assert_eq!(claims.account_id.as_deref(), Some("acc-1"));
}
