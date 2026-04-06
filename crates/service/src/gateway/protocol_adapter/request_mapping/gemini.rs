use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeminiThinkingLevel {
    Disabled,
    Auto,
    Effort(&'static str),
}

#[derive(Debug, Clone, Copy)]
struct GeminiReasoningDecision {
    effort: Option<&'static str>,
    include_reasoning: bool,
}

/// 函数 `convert_gemini_generate_content_request`
///
/// 作者: Codex
///
/// 时间: 2026-04-05
///
/// # 参数
/// - path: Gemini 原生请求路径
/// - body: Gemini 原生请求体
///
/// # 返回
/// 返回转换后的 Responses 请求体、是否流式和工具名还原映射
pub(crate) fn convert_gemini_generate_content_request(
    path: &str,
    body: &[u8],
) -> Result<(Vec<u8>, bool, super::ToolNameRestoreMap), String> {
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| "invalid gemini request json".to_string())?;
    let (source, body_model) = extract_gemini_request_source(payload)?;
    let obj = &source;

    let model = extract_model_from_path(path)
        .or(body_model)
        .ok_or_else(|| "gemini model is required".to_string())?;
    let request_stream = normalized_request_path(path).contains(":streamGenerateContent");
    let tool_names = collect_gemini_declared_tool_names(obj);
    let (tool_name_map, tool_name_restore_map) = build_gemini_cpa_tool_name_maps(tool_names);
    let mut input_items = convert_gemini_contents_to_cpa_responses_input(obj, &tool_name_map)?;
    if let Some(system_message) = build_gemini_system_instruction_message(obj) {
        input_items.insert(0, system_message);
    }

    let mut out = serde_json::Map::new();
    out.insert("model".to_string(), Value::String(model));
    out.insert("instructions".to_string(), Value::String(String::new()));
    out.insert("input".to_string(), Value::Array(input_items));
    out.insert("stream".to_string(), Value::Bool(true));
    out.insert("store".to_string(), Value::Bool(false));
    out.insert("parallel_tool_calls".to_string(), Value::Bool(true));

    if let Some(tools) = map_gemini_tools_to_cpa_responses(obj, &tool_name_map) {
        out.insert("tools".to_string(), Value::Array(tools));
        out.insert("tool_choice".to_string(), Value::String("auto".to_string()));
    }

    let effort = resolve_gemini_cpa_reasoning_effort(obj);
    out.insert(
        "reasoning".to_string(),
        json!({ "effort": effort, "summary": "auto" }),
    );
    out.insert(
        "include".to_string(),
        Value::Array(vec![Value::String(
            "reasoning.encrypted_content".to_string(),
        )]),
    );

    serde_json::to_vec(&Value::Object(out))
        .map(|bytes| (bytes, request_stream, tool_name_restore_map))
        .map_err(|err| format!("convert gemini request failed: {err}"))
}

fn extract_gemini_request_source(
    payload: Value,
) -> Result<(serde_json::Map<String, Value>, Option<String>), String> {
    let Some(root) = payload.as_object() else {
        return Err("gemini request body must be an object".to_string());
    };
    let body_model = root
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut source = root
        .get("request")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| root.clone());
    if source.contains_key("systemInstruction") && !source.contains_key("system_instruction") {
        if let Some(value) = source.remove("systemInstruction") {
            source.insert("system_instruction".to_string(), value);
        }
    }
    Ok((source, body_model))
}

fn build_gemini_system_instruction_message(
    source: &serde_json::Map<String, Value>,
) -> Option<Value> {
    let system = get_value_field(source, &["system_instruction", "systemInstruction"])?;
    let parts = system.get("parts").and_then(Value::as_array)?;
    let mut content_parts = Vec::new();
    for part in parts {
        let Some(text) = part
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        content_parts.push(json!({ "type": "input_text", "text": text }));
    }
    if content_parts.is_empty() {
        return None;
    }
    Some(json!({
        "type": "message",
        "role": "developer",
        "content": content_parts,
    }))
}

fn convert_gemini_contents_to_cpa_responses_input(
    source: &serde_json::Map<String, Value>,
    tool_name_map: &BTreeMap<String, String>,
) -> Result<Vec<Value>, String> {
    let mut items = Vec::new();
    let mut pending_call_ids: VecDeque<String> = VecDeque::new();
    let contents = source
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| "gemini contents field is required".to_string())?;
    for content in contents {
        let role = content
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let role = if role == "model" { "assistant" } else { role };
        let Some(parts) = content.get("parts").and_then(Value::as_array) else {
            continue;
        };
        for part in parts {
            if let Some(text) = part
                .get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                let part_type = if role == "assistant" {
                    "output_text"
                } else {
                    "input_text"
                };
                items.push(json!({
                    "type": "message",
                    "role": role,
                    "content": [{ "type": part_type, "text": text }],
                }));
                continue;
            }

            if let Some(function_call) = part.get("functionCall").and_then(Value::as_object) {
                let Some(name) = function_call
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    continue;
                };
                let mapped = map_gemini_cpa_tool_name(name, tool_name_map);
                let call_id = generate_gemini_call_id();
                pending_call_ids.push_back(call_id.clone());
                let arguments = function_call
                    .get("args")
                    .and_then(|value| serde_json::to_string(value).ok())
                    .unwrap_or_else(|| "{}".to_string());
                items.push(json!({
                    "type": "function_call",
                    "name": mapped,
                    "arguments": arguments,
                    "call_id": call_id,
                }));
                continue;
            }

            if let Some(function_response) = part.get("functionResponse").and_then(Value::as_object)
            {
                let output = if let Some(result) = function_response
                    .get("response")
                    .and_then(|value| value.get("result"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    result.to_string()
                } else if let Some(response) = function_response.get("response") {
                    serde_json::to_string(response).unwrap_or_default()
                } else {
                    String::new()
                };
                let call_id = pending_call_ids
                    .pop_front()
                    .unwrap_or_else(generate_gemini_call_id);
                items.push(json!({
                    "type": "function_call_output",
                    "output": output,
                    "call_id": call_id,
                }));
            }
        }
    }
    Ok(items)
}

fn generate_gemini_call_id() -> String {
    let rand_text: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    format!("call_{rand_text}")
}

fn collect_gemini_declared_tool_names(source: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut names = Vec::new();
    let Some(tools) = source.get("tools").and_then(Value::as_array) else {
        return names;
    };
    for tool in tools {
        let Some(tool_obj) = tool.as_object() else {
            continue;
        };
        let Some(function_declarations) =
            get_array_field(tool_obj, &["functionDeclarations", "function_declarations"])
        else {
            continue;
        };
        for declaration in function_declarations {
            let Some(name) = declaration
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            names.push(name.to_string());
        }
    }
    names
}

fn build_gemini_cpa_tool_name_maps(
    names: Vec<String>,
) -> (BTreeMap<String, String>, super::ToolNameRestoreMap) {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() || seen.contains(trimmed) {
            continue;
        }
        seen.insert(trimmed.to_string());
        ordered.push(trimmed.to_string());
    }

    let mut used = BTreeSet::new();
    let mut tool_name_map = BTreeMap::new();
    let mut restore_map = super::ToolNameRestoreMap::new();
    for original in ordered {
        let base = gemini_cpa_short_candidate(original.as_str());
        let unique = gemini_cpa_make_unique(&base, &mut used);
        if original != unique {
            restore_map.insert(unique.clone(), original.clone());
        }
        tool_name_map.insert(original, unique);
    }
    (tool_name_map, restore_map)
}

fn gemini_cpa_short_candidate(name: &str) -> String {
    const LIMIT: usize = 64;
    if name.len() <= LIMIT {
        return name.to_string();
    }
    if name.starts_with("mcp__") {
        if let Some(idx) = name.rfind("__") {
            if idx > 0 {
                let mut candidate = format!("mcp__{}", &name[idx + 2..]);
                if candidate.len() > LIMIT {
                    candidate.truncate(LIMIT);
                }
                return candidate;
            }
        }
    }
    name.chars().take(LIMIT).collect()
}

fn gemini_cpa_make_unique(base: &str, used: &mut BTreeSet<String>) -> String {
    const LIMIT: usize = 64;
    if !used.contains(base) {
        used.insert(base.to_string());
        return base.to_string();
    }
    for idx in 1usize.. {
        let suffix = format!("_{idx}");
        let allowed = LIMIT.saturating_sub(suffix.len());
        let mut prefix = base.to_string();
        if prefix.len() > allowed {
            prefix.truncate(allowed);
        }
        let candidate = format!("{prefix}{suffix}");
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
    }
    base.to_string()
}

fn map_gemini_cpa_tool_name(name: &str, tool_name_map: &BTreeMap<String, String>) -> String {
    tool_name_map
        .get(name)
        .cloned()
        .unwrap_or_else(|| gemini_cpa_short_candidate(name))
}

fn map_gemini_tools_to_cpa_responses(
    source: &serde_json::Map<String, Value>,
    tool_name_map: &BTreeMap<String, String>,
) -> Option<Vec<Value>> {
    let Some(tools) = source.get("tools").and_then(Value::as_array) else {
        return None;
    };
    let mut out = Vec::new();
    for tool in tools {
        let Some(tool_obj) = tool.as_object() else {
            continue;
        };
        let Some(function_declarations) =
            get_array_field(tool_obj, &["functionDeclarations", "function_declarations"])
        else {
            continue;
        };
        for declaration in function_declarations {
            let Some(name) = declaration
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let mapped_name = map_gemini_cpa_tool_name(name, tool_name_map);
            let mut mapped = serde_json::Map::new();
            mapped.insert("type".to_string(), Value::String("function".to_string()));
            mapped.insert("name".to_string(), Value::String(mapped_name));
            if let Some(description) = declaration.get("description") {
                mapped.insert("description".to_string(), description.clone());
            }
            if let Some(parameters) = get_value_field(
                declaration.as_object().expect("declaration object"),
                &[
                    "parameters",
                    "parametersJsonSchema",
                    "parameters_json_schema",
                ],
            ) {
                let cleaned = clean_gemini_tool_schema(parameters);
                mapped.insert("parameters".to_string(), cleaned);
            }
            mapped.insert("strict".to_string(), Value::Bool(false));
            let mut tool_value = Value::Object(mapped);
            lowercase_type_fields(&mut tool_value);
            out.push(tool_value);
        }
    }
    Some(out)
}

fn clean_gemini_tool_schema(value: &Value) -> Value {
    let mut schema = value.clone();
    if let Value::Object(obj) = &mut schema {
        obj.remove("$schema");
        obj.insert("additionalProperties".to_string(), Value::Bool(false));
    }
    schema
}

fn lowercase_type_fields(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                lowercase_type_fields(item);
            }
        }
        Value::Object(obj) => {
            if let Some(Value::String(text)) = obj.get_mut("type") {
                *text = text.to_ascii_lowercase();
            }
            for value in obj.values_mut() {
                lowercase_type_fields(value);
            }
        }
        _ => {}
    }
}

fn resolve_gemini_cpa_reasoning_effort(source: &serde_json::Map<String, Value>) -> String {
    let mut effort: Option<String> = None;
    if let Some(gen_config) = get_object_field(source, &["generationConfig"]) {
        if let Some(thinking_config) = get_object_field(gen_config, &["thinkingConfig"]) {
            if let Some(level) =
                get_value_field(thinking_config, &["thinkingLevel", "thinking_level"])
                    .and_then(Value::as_str)
            {
                let normalized = level.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    effort = Some(normalized);
                }
            } else if let Some(budget) =
                get_value_field(thinking_config, &["thinkingBudget", "thinking_budget"])
                    .and_then(Value::as_i64)
            {
                if let Some(mapped) = gemini_cpa_budget_to_level(budget) {
                    effort = Some(mapped.to_string());
                }
            }
        }
    }
    effort.unwrap_or_else(|| "medium".to_string())
}

fn gemini_cpa_budget_to_level(budget: i64) -> Option<&'static str> {
    match budget {
        i64::MIN..=-2 => None,
        -1 => Some("auto"),
        0 => Some("none"),
        1..=512 => Some("minimal"),
        513..=1024 => Some("low"),
        1025..=8192 => Some("medium"),
        8193..=24576 => Some("high"),
        24577..=i64::MAX => Some("xhigh"),
    }
}

fn normalize_gemini_request_source(
    mut source: serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    if let Some(contents) = source.get("contents").and_then(Value::as_array) {
        source.insert(
            "contents".to_string(),
            Value::Array(normalize_gemini_contents(contents)),
        );
    }
    source
}

fn normalize_gemini_contents(contents: &[Value]) -> Vec<Value> {
    let mut normalized = Vec::new();
    let mut previous_conversation_role: Option<&'static str> = None;
    for content in contents {
        let Some(content_obj) = content.as_object() else {
            continue;
        };
        let Some(parts) = content_obj.get("parts").and_then(Value::as_array) else {
            continue;
        };
        let filtered_parts = parts
            .iter()
            .filter(|part| is_meaningful_gemini_part(part))
            .cloned()
            .collect::<Vec<_>>();
        if filtered_parts.is_empty() {
            continue;
        }
        let normalized_role = normalize_gemini_content_role(
            content_obj.get("role").and_then(Value::as_str),
            previous_conversation_role,
        );
        if matches!(normalized_role, "user" | "model") {
            previous_conversation_role = Some(normalized_role);
        }
        let mut normalized_content = content_obj.clone();
        normalized_content.insert(
            "role".to_string(),
            Value::String(normalized_role.to_string()),
        );
        normalized_content.insert("parts".to_string(), Value::Array(filtered_parts));
        normalized.push(Value::Object(normalized_content));
    }
    normalized
}

fn normalize_gemini_content_role(
    role: Option<&str>,
    previous_role: Option<&'static str>,
) -> &'static str {
    match role.map(str::trim) {
        Some("user") => "user",
        Some("model") => "model",
        Some("function") => "function",
        _ => match previous_role {
            None => "user",
            Some("user") => "model",
            Some("model") => "user",
            Some(_) => "user",
        },
    }
}

fn is_meaningful_gemini_part(part: &Value) -> bool {
    let Some(part_obj) = part.as_object() else {
        return false;
    };
    if part_obj
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    if map_gemini_image_part_to_responses_item(part_obj).is_some() {
        return true;
    }
    if get_object_field(part_obj, &["functionCall", "function_call"]).is_some() {
        return true;
    }
    get_object_field(part_obj, &["functionResponse", "function_response"]).is_some()
}

fn normalized_request_path(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

fn extract_model_from_path(path: &str) -> Option<String> {
    let normalized = normalized_request_path(path);
    ["/v1/models/", "/v1beta/models/", "/v1alpha/models/"]
        .iter()
        .find_map(|prefix| {
            normalized.strip_prefix(prefix).and_then(|rest| {
                let (model, _) = rest.split_once(':')?;
                let trimmed = model.trim();
                if trimmed.is_empty() {
                    None
                } else if let Some(stripped) = trimmed.strip_prefix("models/") {
                    Some(stripped.to_string())
                } else {
                    Some(trimmed.to_string())
                }
            })
        })
}

fn collect_gemini_tool_names(source: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut names = Vec::new();

    if let Some(tools) = get_array_field(source, &["tools"]) {
        for tool in tools {
            let Some(tool_obj) = tool.as_object() else {
                continue;
            };
            let Some(function_declarations) =
                get_array_field(tool_obj, &["functionDeclarations", "function_declarations"])
            else {
                continue;
            };
            for declaration in function_declarations {
                let Some(name) = declaration
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    continue;
                };
                names.push(name.to_string());
            }
        }
    }

    if let Some(allowed_function_names) =
        get_tool_config_field_array(source, &["allowedFunctionNames", "allowed_function_names"])
    {
        for name in allowed_function_names {
            let Some(name) = name
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            names.push(name.to_string());
        }
    }

    if let Some(contents) = source.get("contents").and_then(Value::as_array) {
        for content in contents {
            let Some(parts) = content.get("parts").and_then(Value::as_array) else {
                continue;
            };
            for part in parts {
                if let Some(name) = get_part_object_field(part, &["functionCall", "function_call"])
                    .and_then(|value| value.get("name"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
                if let Some(name) =
                    get_part_object_field(part, &["functionResponse", "function_response"])
                        .and_then(|value| value.get("name"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
            }
        }
    }

    names
}

fn collect_gemini_allowed_function_names(
    source: &serde_json::Map<String, Value>,
) -> Option<BTreeSet<String>> {
    let allowed_items = source
        .get("toolConfig")
        .or_else(|| source.get("tool_config"))
        .and_then(Value::as_object)
        .and_then(|value| {
            get_object_field(value, &["functionCallingConfig", "function_calling_config"])
        })
        .and_then(|value| {
            get_value_field(value, &["allowedFunctionNames", "allowed_function_names"])
        })?;
    let mut names = BTreeSet::new();
    if let Some(items) = allowed_items.as_array() {
        for item in items {
            let Some(name) = item
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            names.insert(name.to_string());
        }
    }
    Some(names)
}

fn convert_gemini_contents_to_responses_input(
    source: &serde_json::Map<String, Value>,
    tool_name_map: &BTreeMap<String, String>,
) -> Result<(Option<String>, Vec<Value>), String> {
    let mut messages = Vec::new();
    let mut pending_tool_calls: BTreeMap<String, VecDeque<String>> = BTreeMap::new();
    let mut pending_tool_call_names = VecDeque::new();
    let mut synthetic_call_index = 0usize;

    if let Some(system_instruction) =
        get_value_field(source, &["systemInstruction", "system_instruction"])
    {
        let system_text = extract_text_from_content_like(system_instruction);
        if !system_text.trim().is_empty() {
            messages.push(json!({
                "role": "system",
                "content": system_text,
            }));
        }
    }

    let contents = source
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| "gemini contents field is required".to_string())?;
    for content in contents {
        let Some(content_obj) = content.as_object() else {
            return Err("invalid gemini content item".to_string());
        };
        let role = content_obj
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user");
        let parts = content_obj
            .get("parts")
            .and_then(Value::as_array)
            .ok_or_else(|| "gemini content parts field is required".to_string())?;

        let mut pending_user_parts = Vec::new();
        let mut pending_model_parts = Vec::new();

        let mut part_index = 0usize;
        while part_index < parts.len() {
            let Some(part_obj) = parts[part_index].as_object() else {
                part_index += 1;
                continue;
            };
            if let Some(text) = part_obj
                .get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                match role {
                    "model" => pending_model_parts.push(json!({
                        "type": "output_text",
                        "text": text,
                    })),
                    _ => pending_user_parts.push(json!({
                        "type": "input_text",
                        "text": text,
                    })),
                }
                part_index += 1;
                continue;
            }

            if let Some(image_item) = map_gemini_image_part_to_responses_item(part_obj) {
                pending_user_parts.push(image_item);
                part_index += 1;
                continue;
            }

            if let Some(function_call) =
                get_object_field(part_obj, &["functionCall", "function_call"])
            {
                if !pending_model_parts.is_empty() {
                    messages.push(json!({
                        "role": "assistant",
                        "content": pending_model_parts.clone(),
                    }));
                    pending_model_parts.clear();
                }
                let Some(name) = function_call
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    continue;
                };
                let mapped_name =
                    super::openai::shorten_openai_tool_name_with_map(name, tool_name_map);
                let call_id = function_call
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        synthetic_call_index += 1;
                        format!("call_gemini_{synthetic_call_index}")
                    });
                pending_tool_call_names.push_back(mapped_name.clone());
                pending_tool_calls
                    .entry(mapped_name.clone())
                    .or_default()
                    .push_back(call_id.clone());
                let arguments = function_call
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                messages.push(json!({
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": call_id,
                        "name": mapped_name,
                        "input": arguments,
                    }],
                }));
                part_index += 1;
                continue;
            }

            if let Some(function_response) =
                get_object_field(part_obj, &["functionResponse", "function_response"])
            {
                if !pending_user_parts.is_empty() {
                    messages.push(json!({
                        "role": "user",
                        "content": pending_user_parts.clone(),
                    }));
                    pending_user_parts.clear();
                }
                let mapped_name = function_response
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|name| {
                        super::openai::shorten_openai_tool_name_with_map(name, tool_name_map)
                    })
                    .map(|mapped_name| {
                        consume_pending_tool_call_name(
                            &mut pending_tool_call_names,
                            Some(mapped_name.as_str()),
                        )
                        .unwrap_or(mapped_name)
                    })
                    .or_else(|| consume_pending_tool_call_name(&mut pending_tool_call_names, None));
                let call_id = function_response
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        mapped_name.as_deref().and_then(|mapped_name| {
                            pending_tool_calls
                                .get_mut(mapped_name)
                                .and_then(VecDeque::pop_front)
                        })
                    })
                    .unwrap_or_else(|| {
                        synthetic_call_index += 1;
                        format!("call_gemini_{synthetic_call_index}")
                    });
                let output = function_response
                    .get("response")
                    .cloned()
                    .unwrap_or_else(|| Value::String(String::new()));
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": collect_gemini_function_response_content(parts, part_index, output),
                }));
                part_index = advance_gemini_function_response_index(parts, part_index);
                continue;
            }

            part_index += 1;
        }

        if !pending_user_parts.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": pending_user_parts,
            }));
        }
        if !pending_model_parts.is_empty() {
            messages.push(json!({
                "role": "assistant",
                "content": pending_model_parts,
            }));
        }
    }

    super::convert_chat_messages_to_responses_input(&messages, tool_name_map)
}

fn advance_gemini_function_response_index(parts: &[Value], start_index: usize) -> usize {
    let mut index = start_index + 1;
    while index < parts.len() {
        let Some(part_obj) = parts[index].as_object() else {
            break;
        };
        if part_obj.get("functionResponse").is_some() || part_obj.get("functionCall").is_some() {
            break;
        }
        if part_obj
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
            || map_gemini_image_part_to_responses_item(part_obj).is_some()
        {
            index += 1;
            continue;
        }
        break;
    }
    index
}

fn collect_gemini_function_response_content(
    parts: &[Value],
    start_index: usize,
    output: Value,
) -> Value {
    let mut content_items = map_gemini_function_response_output_items(&output);
    let mut index = start_index + 1;
    while index < parts.len() {
        let Some(part_obj) = parts[index].as_object() else {
            break;
        };
        if part_obj.get("functionResponse").is_some() || part_obj.get("functionCall").is_some() {
            break;
        }
        if let Some(text) = part_obj
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            content_items.push(json!({
                "type": "input_text",
                "text": text,
            }));
            index += 1;
            continue;
        }
        if let Some(image_item) = map_gemini_image_part_to_responses_item(part_obj) {
            content_items.push(image_item);
            index += 1;
            continue;
        }
        break;
    }
    if content_items.is_empty() {
        output
    } else {
        Value::Array(content_items)
    }
}

fn map_gemini_function_response_output_items(output: &Value) -> Vec<Value> {
    match output {
        Value::Null => Vec::new(),
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "type": "input_text",
                    "text": trimmed,
                })]
            }
        }
        Value::Array(items) => items
            .iter()
            .flat_map(map_gemini_function_response_output_items)
            .collect(),
        Value::Object(obj) => {
            if let Some(output_value) = obj.get("output") {
                let nested_items = map_gemini_function_response_output_items(output_value);
                if !nested_items.is_empty() {
                    return nested_items;
                }
            }
            let serialized = serde_json::to_string(output).unwrap_or_default();
            let trimmed = serialized.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "type": "input_text",
                    "text": trimmed,
                })]
            }
        }
        other => {
            let serialized = serde_json::to_string(other).unwrap_or_default();
            let trimmed = serialized.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "type": "input_text",
                    "text": trimmed,
                })]
            }
        }
    }
}

fn resolve_parallel_tool_calls(source: &serde_json::Map<String, Value>) -> bool {
    get_tool_config_field_value(
        source,
        &["disableParallelToolUse", "disable_parallel_tool_use"],
    )
    .and_then(Value::as_bool)
    .map(|disabled| !disabled)
    .unwrap_or(true)
}

fn resolve_tool_choice(
    source: &serde_json::Map<String, Value>,
    tool_name_map: &BTreeMap<String, String>,
    allowed_function_names: Option<&BTreeSet<String>>,
) -> Option<Value> {
    let config = source
        .get("toolConfig")
        .or_else(|| source.get("tool_config"))
        .and_then(Value::as_object)
        .and_then(|value| {
            get_object_field(value, &["functionCallingConfig", "function_calling_config"])
        })?;
    let mode = config
        .get("mode")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_uppercase())
        .unwrap_or_else(|| "AUTO".to_string());
    let allowed = get_value_field(config, &["allowedFunctionNames", "allowed_function_names"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter(|value| {
                    allowed_function_names
                        .map(|allowed_names| allowed_names.contains(*value))
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match mode.as_str() {
        "NONE" => Some(Value::String("none".to_string())),
        "ANY" => {
            if allowed.is_empty() && allowed_function_names.is_some() {
                return Some(Value::String("none".to_string()));
            }
            if allowed.len() == 1 {
                let mapped_name =
                    super::openai::shorten_openai_tool_name_with_map(allowed[0], tool_name_map);
                Some(json!({
                    "type": "function",
                    "name": mapped_name,
                }))
            } else {
                Some(Value::String("required".to_string()))
            }
        }
        _ => Some(Value::String("auto".to_string())),
    }
}

fn map_gemini_tools_to_responses(
    source: &serde_json::Map<String, Value>,
    tool_name_map: &BTreeMap<String, String>,
    allowed_function_names: Option<&BTreeSet<String>>,
) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    let Some(tools) = get_array_field(source, &["tools"]) else {
        return None;
    };
    for tool in tools {
        let Some(tool_obj) = tool.as_object() else {
            continue;
        };
        let Some(function_declarations) =
            get_array_field(tool_obj, &["functionDeclarations", "function_declarations"])
        else {
            continue;
        };
        for declaration in function_declarations {
            let Some(name) = declaration
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if allowed_function_names.is_some_and(|items| !items.contains(name)) {
                continue;
            }
            let mapped_name = super::openai::shorten_openai_tool_name_with_map(name, tool_name_map);
            let mut mapped = serde_json::Map::new();
            mapped.insert("type".to_string(), Value::String("function".to_string()));
            mapped.insert("name".to_string(), Value::String(mapped_name));
            if let Some(description) = declaration.get("description") {
                mapped.insert("description".to_string(), description.clone());
            }
            let parameters = get_value_field(
                declaration.as_object().expect("declaration object"),
                &[
                    "parametersJsonSchema",
                    "parameters_json_schema",
                    "parameters",
                ],
            )
            .cloned()
            .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
            mapped.insert(
                "parameters".to_string(),
                super::fix_array_items_in_schema(parameters),
            );
            out.push(Value::Object(mapped));
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn get_part_object_field<'a>(
    part: &'a Value,
    keys: &[&str],
) -> Option<&'a serde_json::Map<String, Value>> {
    part.as_object()
        .and_then(|part_obj| get_object_field(part_obj, keys))
}

fn consume_pending_tool_call_name(
    pending_tool_call_names: &mut VecDeque<String>,
    expected_name: Option<&str>,
) -> Option<String> {
    match expected_name {
        Some(expected_name) => {
            let position = pending_tool_call_names
                .iter()
                .position(|item| item == expected_name)?;
            pending_tool_call_names.remove(position)
        }
        None => pending_tool_call_names.pop_front(),
    }
}

fn map_gemini_image_part_to_responses_item(
    part_obj: &serde_json::Map<String, Value>,
) -> Option<Value> {
    if let Some(inline_data) = get_object_field(part_obj, &["inlineData", "inline_data"]) {
        let mime_type = inline_data
            .get("mimeType")
            .or_else(|| inline_data.get("mime_type"))
            .and_then(Value::as_str)
            .unwrap_or("image/png");
        let data = inline_data
            .get("data")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        return Some(json!({
            "type": "input_image",
            "image_url": format!("data:{mime_type};base64,{data}"),
        }));
    }

    let file_data = get_object_field(part_obj, &["fileData", "file_data"])?;
    let file_uri = file_data
        .get("fileUri")
        .or_else(|| file_data.get("file_uri"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(json!({
        "type": "input_image",
        "image_url": file_uri,
    }))
}

fn extract_text_from_content_like(value: &Value) -> String {
    let mut parts = Vec::new();
    match value {
        Value::String(text) => parts.push(text.to_string()),
        Value::Object(obj) => {
            if let Some(part_items) = obj.get("parts").and_then(Value::as_array) {
                for item in part_items {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            parts.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                let text = extract_text_from_content_like(item);
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        _ => {}
    }
    parts.join("\n\n")
}

fn copy_optional_field_alias(
    source: &serde_json::Map<String, Value>,
    target: &mut serde_json::Map<String, Value>,
    from_keys: &[&str],
    to_key: &str,
) {
    if let Some(value) = get_value_field(source, from_keys) {
        target.insert(to_key.to_string(), value.clone());
    }
}

fn get_value_field<'a>(
    source: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a Value> {
    keys.iter().find_map(|key| source.get(*key))
}

fn get_array_field<'a>(
    source: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a Vec<Value>> {
    get_value_field(source, keys).and_then(Value::as_array)
}

fn get_object_field<'a>(
    source: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a serde_json::Map<String, Value>> {
    get_value_field(source, keys).and_then(Value::as_object)
}

fn get_tool_config_field_value<'a>(
    source: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a Value> {
    source
        .get("toolConfig")
        .or_else(|| source.get("tool_config"))
        .and_then(Value::as_object)
        .and_then(|value| {
            get_object_field(value, &["functionCallingConfig", "function_calling_config"])
        })
        .and_then(|value| get_value_field(value, keys))
}

fn get_tool_config_field_array<'a>(
    source: &'a serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<&'a Vec<Value>> {
    get_tool_config_field_value(source, keys).and_then(Value::as_array)
}

fn resolve_gemini_reasoning_decision(
    source: &serde_json::Map<String, Value>,
) -> Option<GeminiReasoningDecision> {
    let generation_config = get_object_field(source, &["generationConfig", "generation_config"])?;
    let thinking_config =
        get_object_field(generation_config, &["thinkingConfig", "thinking_config"])?;
    let include_thoughts =
        get_value_field(thinking_config, &["includeThoughts", "include_thoughts"])
            .and_then(Value::as_bool)
            .unwrap_or(true);

    if let Some(level) = get_value_field(thinking_config, &["thinkingLevel", "thinking_level"])
        .and_then(Value::as_str)
        .and_then(normalize_gemini_thinking_level)
    {
        return Some(build_gemini_reasoning_decision(level, include_thoughts));
    }

    let budget = get_value_field(thinking_config, &["thinkingBudget", "thinking_budget"])
        .and_then(Value::as_i64)?;
    Some(build_gemini_reasoning_decision(
        map_gemini_thinking_budget_to_level(budget)?,
        include_thoughts,
    ))
}

fn build_gemini_reasoning_decision(
    level: GeminiThinkingLevel,
    include_thoughts: bool,
) -> GeminiReasoningDecision {
    match level {
        GeminiThinkingLevel::Disabled | GeminiThinkingLevel::Auto => GeminiReasoningDecision {
            effort: None,
            include_reasoning: false,
        },
        GeminiThinkingLevel::Effort(effort) => GeminiReasoningDecision {
            effort: Some(effort),
            include_reasoning: include_thoughts,
        },
    }
}

fn normalize_gemini_thinking_level(value: &str) -> Option<GeminiThinkingLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Some(GeminiThinkingLevel::Disabled),
        "auto" => Some(GeminiThinkingLevel::Auto),
        "minimal" => Some(GeminiThinkingLevel::Effort("low")),
        "low" => Some(GeminiThinkingLevel::Effort("low")),
        "medium" => Some(GeminiThinkingLevel::Effort("medium")),
        "high" => Some(GeminiThinkingLevel::Effort("high")),
        "xhigh" | "extra_high" => Some(GeminiThinkingLevel::Effort("xhigh")),
        _ => None,
    }
}

fn map_gemini_thinking_budget_to_level(budget: i64) -> Option<GeminiThinkingLevel> {
    match budget {
        i64::MIN..=-2 => None,
        -1 => Some(GeminiThinkingLevel::Auto),
        0 => Some(GeminiThinkingLevel::Disabled),
        1..=1024 => Some(GeminiThinkingLevel::Effort("low")),
        1025..=8192 => Some(GeminiThinkingLevel::Effort("medium")),
        8193..=24576 => Some(GeminiThinkingLevel::Effort("high")),
        24577..=i64::MAX => Some(GeminiThinkingLevel::Effort("xhigh")),
    }
}
