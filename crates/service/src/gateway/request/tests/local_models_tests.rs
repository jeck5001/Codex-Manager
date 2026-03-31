use super::*;
use serde_json::Value;

#[test]
fn build_openai_models_list_outputs_expected_shape() {
    let items = vec![
        ModelOption {
            slug: "gpt-5.3-codex".to_string(),
            display_name: "GPT-5.3 Codex".to_string(),
        },
        ModelOption {
            slug: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
        },
    ];
    let output = build_openai_models_list(&items);
    let value: Value = serde_json::from_str(&output).expect("valid json");
    assert_eq!(value.get("object").and_then(Value::as_str), Some("list"));
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .expect("data array");
    assert_eq!(data.len(), 2);
    assert_eq!(data[0].get("id").and_then(Value::as_str), Some("gpt-4o"));
    assert_eq!(
        data[1].get("id").and_then(Value::as_str),
        Some("gpt-5.3-codex")
    );
    assert_eq!(
        data[0].get("created").and_then(Value::as_i64),
        Some(1715558400)
    );
    assert_eq!(
        data[1].get("created").and_then(Value::as_i64),
        Some(1770249600)
    );
    for item in data {
        assert_eq!(item.get("object").and_then(Value::as_str), Some("model"));
        assert!(item.get("id").and_then(Value::as_str).is_some());
        assert_eq!(item.get("owned_by").and_then(Value::as_str), Some("openai"));
    }
}
