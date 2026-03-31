use std::collections::HashSet;

use codexmanager_core::rpc::types::ModelOption;
use serde_json::Value;

pub(super) fn parse_model_options(body: &[u8]) -> Vec<ModelOption> {
    let mut items: Vec<ModelOption> = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        parse_models_array(
            value.get("models").and_then(|v| v.as_array()),
            &mut seen,
            &mut items,
        );
        parse_data_array(
            value.get("data").and_then(|v| v.as_array()),
            &mut seen,
            &mut items,
        );
    }

    items.sort_by(|a, b| a.slug.cmp(&b.slug));
    items
}

fn parse_models_array(
    models: Option<&Vec<Value>>,
    seen: &mut HashSet<String>,
    items: &mut Vec<ModelOption>,
) {
    let Some(models) = models else {
        return;
    };
    for item in models {
        let slug = item
            .get("slug")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if let Some(slug) = slug {
            push_model_option(slug, seen, items);
        }
    }
}

fn parse_data_array(
    data: Option<&Vec<Value>>,
    seen: &mut HashSet<String>,
    items: &mut Vec<ModelOption>,
) {
    let Some(data) = data else {
        return;
    };
    for item in data {
        let slug = item
            .get("id")
            .or_else(|| item.get("slug"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if let Some(slug) = slug {
            push_model_option(slug, seen, items);
        }
    }
}

fn push_model_option(slug: &str, seen: &mut HashSet<String>, items: &mut Vec<ModelOption>) {
    if seen.insert(slug.to_string()) {
        items.push(ModelOption {
            slug: slug.to_string(),
            display_name: slug.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_model_options;

    #[test]
    fn parse_model_options_normalizes_display_name_to_slug() {
        let body = json!({
            "data": [
                { "id": "gpt-5.4-mini", "display_name": "GPT-5.4 Mini" },
                { "slug": "o3", "title": "OpenAI o3" }
            ]
        })
        .to_string();

        let items = parse_model_options(body.as_bytes());
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].slug, "gpt-5.4-mini");
        assert_eq!(items[0].display_name, "gpt-5.4-mini");
        assert_eq!(items[1].slug, "o3");
        assert_eq!(items[1].display_name, "o3");
    }
}
