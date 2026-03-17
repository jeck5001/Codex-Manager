use super::next_account_sort;
use crate::account_identity::{build_account_storage_id, pick_existing_account_id_by_identity};
use crate::auth_tokens::{ensure_workspace_allowed, parse_token_endpoint_error};
use codexmanager_core::auth::parse_id_token_claims;
use codexmanager_core::storage::{now_ts, Account, Storage};

fn build_account(
    id: &str,
    chatgpt_account_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Account {
    let now = now_ts();
    Account {
        id: id.to_string(),
        label: id.to_string(),
        issuer: "https://auth.openai.com".to_string(),
        chatgpt_account_id: chatgpt_account_id.map(|v| v.to_string()),
        workspace_id: workspace_id.map(|v| v.to_string()),
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn pick_existing_account_requires_exact_scope_when_workspace_present() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    storage
        .insert_account(&build_account("acc-ws-a", Some("cgpt-1"), Some("ws-a")))
        .expect("insert ws-a");

    let found = pick_existing_account_id_by_identity(
        storage.list_accounts().expect("list accounts").iter(),
        Some("cgpt-1"),
        Some("ws-b"),
        Some("sub-fallback"),
        None,
    );

    assert_eq!(found, None);
}

#[test]
fn pick_existing_account_matches_exact_workspace_scope() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    storage
        .insert_account(&build_account("acc-ws-a", Some("cgpt-1"), Some("ws-a")))
        .expect("insert ws-a");
    storage
        .insert_account(&build_account("acc-ws-b", Some("cgpt-1"), Some("ws-b")))
        .expect("insert ws-b");

    let found = pick_existing_account_id_by_identity(
        storage.list_accounts().expect("list accounts").iter(),
        Some("cgpt-1"),
        Some("ws-b"),
        Some("sub-fallback"),
        None,
    );

    assert_eq!(found.as_deref(), Some("acc-ws-b"));
}

#[test]
fn build_account_storage_id_keeps_login_scope_shape() {
    let account_id = build_account_storage_id("sub-1", Some("cgpt-1"), Some("ws-a"), None);
    assert_eq!(account_id, "sub-1::cgpt=cgpt-1|ws=ws-a");
}

#[test]
fn next_account_sort_uses_step_five() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    storage
        .insert_account(&build_account("acc-1", Some("cgpt-1"), Some("ws-1")))
        .expect("insert account 1");
    storage
        .update_account_sort("acc-1", 2)
        .expect("update sort 1");
    storage
        .insert_account(&build_account("acc-2", Some("cgpt-2"), Some("ws-2")))
        .expect("insert account 2");
    storage
        .update_account_sort("acc-2", 7)
        .expect("update sort 2");

    assert_eq!(next_account_sort(&storage), 12);
}

fn jwt_with_claims(payload: &str) -> String {
    format!("eyJhbGciOiJIUzI1NiJ9.{payload}.sig")
}

#[test]
fn ensure_workspace_allowed_accepts_matching_auth_chatgpt_account_id() {
    let token = jwt_with_claims(
        "eyJzdWIiOiJ1c2VyLTEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoib3JnX2FiYyJ9fQ",
    );
    let claims = parse_id_token_claims(&token).expect("claims");

    let result = ensure_workspace_allowed(Some("org_abc"), &claims, &token, &token);

    assert!(result.is_ok(), "workspace should match: {:?}", result);
}

#[test]
fn ensure_workspace_allowed_rejects_mismatched_workspace() {
    let token = jwt_with_claims("eyJzdWIiOiJ1c2VyLTEiLCJ3b3Jrc3BhY2VfaWQiOiJvcmdfYWJjIn0");
    let claims = parse_id_token_claims(&token).expect("claims");

    let result = ensure_workspace_allowed(Some("org_other"), &claims, &token, &token);

    assert_eq!(
        result.expect_err("should reject mismatch"),
        "Login is restricted to workspace id org_other."
    );
}

#[test]
fn parse_token_endpoint_error_prefers_error_description() {
    let detail = parse_token_endpoint_error(
        r#"{"error":"invalid_grant","error_description":"refresh token expired"}"#,
    );

    assert_eq!(detail.to_string(), "refresh token expired");
}

#[test]
fn parse_token_endpoint_error_reads_nested_error_message_and_code() {
    let detail = parse_token_endpoint_error(
        r#"{"error":{"code":"proxy_auth_required","message":"proxy authentication required"}}"#,
    );

    assert_eq!(detail.to_string(), "proxy authentication required");
}

#[test]
fn parse_token_endpoint_error_preserves_plain_text_for_display() {
    let detail = parse_token_endpoint_error("service unavailable");

    assert_eq!(detail.to_string(), "service unavailable");
}

#[test]
fn parse_token_endpoint_error_summarizes_challenge_html() {
    let detail =
        parse_token_endpoint_error("<html><title>Just a moment...</title><body>cf</body></html>");

    assert_eq!(
        detail.to_string(),
        "Cloudflare 安全验证页（title=Just a moment...）"
    );
}

#[test]
fn parse_token_endpoint_error_summarizes_generic_html() {
    let detail = parse_token_endpoint_error("<html><title>502 Bad Gateway</title></html>");

    assert_eq!(
        detail.to_string(),
        "上游返回 HTML 错误页（title=502 Bad Gateway）"
    );
}
