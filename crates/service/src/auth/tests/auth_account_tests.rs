use super::{resolve_plan_type, resolve_plan_type_raw};
use codexmanager_core::auth::parse_id_token_claims;
use codexmanager_core::storage::Token;

fn jwt_with_claims(payload: &str) -> String {
    format!("eyJhbGciOiJIUzI1NiJ9.{payload}.sig")
}

fn build_token(access_token: &str, id_token: &str) -> Token {
    Token {
        account_id: "acc-1".to_string(),
        id_token: id_token.to_string(),
        access_token: access_token.to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: 0,
    }
}

#[test]
fn resolve_plan_type_prefers_latest_access_token_claims() {
    let access_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJnbyJ9fQ",
    );
    let id_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJwcm8ifX0",
    );
    let token = build_token(&access_token, &id_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("go"));
}

#[test]
fn resolve_plan_type_falls_back_to_id_token_when_access_claims_missing() {
    let access_token = jwt_with_claims("eyJzdWIiOiJ1c2VyLTEifQ");
    let id_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJ0ZWFtIn19",
    );
    let token = build_token(&access_token, &id_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("team"));
}

#[test]
fn resolve_plan_type_preserves_unknown_raw_value_for_diagnostics() {
    let access_token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9wbGFuX3R5cGUiOiJyZXNlYXJjaGVyX2JldGEifX0",
    );
    let token = build_token(&access_token, &access_token);
    let claims = parse_id_token_claims(&access_token).expect("access claims");

    let resolved = resolve_plan_type(&token, Some(&claims));
    let raw = resolve_plan_type_raw(&token, Some(&claims));

    assert_eq!(resolved.as_deref(), Some("unknown"));
    assert_eq!(raw.as_deref(), Some("researcher_beta"));
}
