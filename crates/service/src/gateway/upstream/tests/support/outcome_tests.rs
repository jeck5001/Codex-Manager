use super::*;
use reqwest::header::HeaderValue;
use std::thread;
use std::time::Duration;

#[test]
fn status_404_with_more_candidates_triggers_failover() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-404",
        reqwest::StatusCode::NOT_FOUND,
        None,
        "https://chatgpt.com/backend-api/codex/chat/completions",
        true,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::Failover));
}

#[test]
fn status_404_on_last_candidate_keeps_upstream_response() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-404",
        reqwest::StatusCode::NOT_FOUND,
        None,
        "https://chatgpt.com/backend-api/codex/chat/completions",
        false,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::RespondUpstream));
}

#[test]
fn status_429_with_more_candidates_triggers_failover() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-429",
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        None,
        "https://api.openai.com/v1/responses",
        true,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::Failover));
}

#[test]
fn status_429_on_last_candidate_keeps_upstream_response() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-429",
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        None,
        "https://api.openai.com/v1/responses",
        false,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::RespondUpstream));
}

#[test]
fn status_429_respects_retry_policy_status_list() {
    let _retry_guard = crate::gateway::retry_policy_test_guard();
    crate::gateway::reset_retry_policy_for_tests();
    crate::gateway::set_retry_policy(3, "immediate", vec![502]).expect("set retry policy");

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-429-disabled",
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        None,
        "https://api.openai.com/v1/responses",
        true,
        |_, _, _| {},
    );

    assert!(matches!(decision, UpstreamOutcomeDecision::RespondUpstream));
    crate::gateway::reset_retry_policy_for_tests();
}

#[test]
fn status_500_with_more_candidates_triggers_failover() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-500",
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        None,
        "https://api.openai.com/v1/responses",
        true,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::Failover));
}

#[test]
fn challenge_with_more_candidates_triggers_failover() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let content_type = HeaderValue::from_static("text/html; charset=utf-8");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-challenge",
        reqwest::StatusCode::FORBIDDEN,
        Some(&content_type),
        "https://chatgpt.com/backend-api/codex/responses",
        true,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::Failover));
}

#[test]
fn auth_401_with_more_candidates_ignores_retry_policy_status_list() {
    let _retry_guard = crate::gateway::retry_policy_test_guard();
    crate::gateway::reset_retry_policy_for_tests();
    crate::gateway::set_retry_policy(3, "immediate", vec![429, 502]).expect("set retry policy");

    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-auth-401",
        reqwest::StatusCode::UNAUTHORIZED,
        None,
        "https://chatgpt.com/backend-api/codex/responses",
        true,
        |_, _, _| {},
    );

    assert!(matches!(decision, UpstreamOutcomeDecision::Failover));
    crate::gateway::reset_retry_policy_for_tests();
}

#[test]
fn challenge_on_last_candidate_keeps_upstream_response() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let content_type = HeaderValue::from_static("text/html; charset=utf-8");
    let decision = decide_upstream_outcome(
        &storage,
        "acc-challenge",
        reqwest::StatusCode::FORBIDDEN,
        Some(&content_type),
        "https://chatgpt.com/backend-api/codex/responses",
        false,
        |_, _, _| {},
    );
    assert!(matches!(decision, UpstreamOutcomeDecision::RespondUpstream));
}

#[test]
fn success_response_enqueues_usage_refresh_for_account() {
    crate::usage_refresh::clear_pending_usage_refresh_tasks_for_tests();
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    let decision = decide_upstream_outcome(
        &storage,
        "acc-success",
        reqwest::StatusCode::OK,
        None,
        "https://chatgpt.com/backend-api/codex/responses",
        false,
        |_, _, _| {},
    );

    assert!(matches!(decision, UpstreamOutcomeDecision::RespondUpstream));
    assert!(crate::usage_refresh::is_usage_refresh_task_pending_for_tests("acc-success"));

    thread::sleep(Duration::from_millis(20));
    crate::usage_refresh::clear_pending_usage_refresh_tasks_for_tests();
}
