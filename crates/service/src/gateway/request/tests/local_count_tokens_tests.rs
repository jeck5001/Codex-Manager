use super::*;

#[test]
fn estimate_input_tokens_uses_messages_and_system_text() {
    let body = br#"{
        "model":"gpt-5.3-codex",
        "system":"abcdabcd",
        "messages":[
            {"role":"user","content":"abcd"},
            {"role":"assistant","content":[{"type":"text","text":"abcdabcd"}]}
        ]
    }"#;
    let count = estimate_input_tokens_from_anthropic_messages(body).expect("estimate failed");
    assert_eq!(count, 5);
}
