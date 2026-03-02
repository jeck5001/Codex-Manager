use super::*;

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
