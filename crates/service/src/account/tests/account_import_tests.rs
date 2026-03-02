use super::{resolve_logical_account_id, ExistingAccountIndex, ImportTokenPayload};
use codexmanager_core::storage::{now_ts, Account, Storage};

fn payload() -> ImportTokenPayload {
    ImportTokenPayload {
        access_token: "access".to_string(),
        id_token: "id".to_string(),
        refresh_token: "refresh".to_string(),
        account_id_hint: None,
    }
}

#[test]
fn resolve_logical_account_id_distinguishes_workspace_under_same_chatgpt() {
    let input = payload();
    let a = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("same-fp"),
    )
    .expect("resolve ws-a");
    let b = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-b"),
        Some("same-fp"),
    )
    .expect("resolve ws-b");

    assert_ne!(a, b);
}

#[test]
fn resolve_logical_account_id_is_stable_when_scope_is_stable() {
    let input = payload();
    let first = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("fp-1"),
    )
    .expect("resolve first");
    let second = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("fp-2"),
    )
    .expect("resolve second");

    assert_eq!(first, second);
}

#[test]
fn existing_account_index_next_sort_uses_step_five() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-1".to_string(),
            label: "acc-1".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-1".to_string()),
            workspace_id: Some("ws-1".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert acc-1");
    storage
        .insert_account(&Account {
            id: "acc-2".to_string(),
            label: "acc-2".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-2".to_string()),
            workspace_id: Some("ws-2".to_string()),
            group_name: None,
            sort: 9,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert acc-2");

    let idx = ExistingAccountIndex::build(&storage).expect("build index");
    assert_eq!(idx.next_sort, 14);
}
