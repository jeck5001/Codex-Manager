use std::collections::{BTreeMap, HashSet};

use codexmanager_core::rpc::types::{
    ApiKeyModelListResult, ManagedModelCatalogEntry, ManagedModelCatalogResult,
    ManagedModelCatalogUpsertParams, ModelInfo, ModelOption, ModelReasoningLevel,
    ModelTruncationPolicy,
};
use codexmanager_core::storage::{
    now_ts, ModelCatalogModelRecord, ModelCatalogReasoningLevelRecord, ModelCatalogScopeRecord,
    ModelCatalogStringItemRecord, Storage,
};
use serde_json::Value;

use crate::gateway;
use crate::storage_helpers;

const MODEL_CACHE_SCOPE_DEFAULT: &str = "default";
const MODEL_SOURCE_KIND_CUSTOM: &str = "custom";
const MODEL_SOURCE_KIND_REMOTE: &str = "remote";

pub(crate) fn read_model_options(refresh_remote: bool) -> Result<ApiKeyModelListResult, String> {
    let catalog = read_managed_model_catalog(refresh_remote)?;
    Ok(ApiKeyModelListResult {
        items: catalog
            .items
            .iter()
            .map(model_option_from_entry)
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn read_managed_model_catalog(
    refresh_remote: bool,
) -> Result<ManagedModelCatalogResult, String> {
    let storage =
        storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let cached_catalog = read_managed_model_catalog_from_storage(&storage)?;
    if !refresh_remote && !cached_catalog.items.is_empty() {
        return Ok(cached_catalog);
    }

    match gateway::fetch_models_for_picker() {
        Ok(items) => {
            let merged_catalog = merge_managed_model_catalog(cached_catalog.clone(), items);
            let _ = save_managed_model_catalog_with_storage(&storage, &merged_catalog);
            Ok(merged_catalog)
        }
        Err(err) => {
            if !cached_catalog.items.is_empty() {
                return Ok(cached_catalog);
            }
            if refresh_remote {
                Err(err)
            } else {
                Ok(ManagedModelCatalogResult::default())
            }
        }
    }
}

pub(crate) fn read_managed_model_catalog_from_storage(
    storage: &Storage,
) -> Result<ManagedModelCatalogResult, String> {
    let rows = storage
        .list_model_catalog_models(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return read_legacy_model_options_catalog(storage);
    }

    let scope_record = storage
        .get_model_catalog_scope(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    let reasoning_levels = storage
        .list_model_catalog_reasoning_levels(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    let additional_speed_tiers = storage
        .list_model_catalog_additional_speed_tiers(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    let experimental_supported_tools = storage
        .list_model_catalog_experimental_supported_tools(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    let input_modalities = storage
        .list_model_catalog_input_modalities(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;
    let available_in_plans = storage
        .list_model_catalog_available_in_plans(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;

    let mut reasoning_by_slug = group_reasoning_levels_by_slug(reasoning_levels);
    let mut speed_tiers_by_slug = group_string_items_by_slug(additional_speed_tiers);
    let mut tools_by_slug = group_string_items_by_slug(experimental_supported_tools);
    let mut modalities_by_slug = group_string_items_by_slug(input_modalities);
    let mut plans_by_slug = group_string_items_by_slug(available_in_plans);

    let response_extra = scope_record
        .as_ref()
        .and_then(|record| parse_extra_json_map(Some(record.extra_json.as_str())))
        .unwrap_or_default();

    let mut rebuilt_items = Vec::new();
    for row in rows {
        let slug = row.slug.clone();
        if let Some(item) = managed_catalog_entry_from_row(
            row,
            reasoning_by_slug.remove(&slug),
            speed_tiers_by_slug.remove(&slug),
            tools_by_slug.remove(&slug),
            modalities_by_slug.remove(&slug),
            plans_by_slug.remove(&slug),
        ) {
            rebuilt_items.push(item);
        }
    }

    Ok(normalize_managed_model_catalog(ManagedModelCatalogResult {
        items: rebuilt_items,
        extra: response_extra,
    }))
}

pub(crate) fn save_managed_model_catalog_model(
    params: ManagedModelCatalogUpsertParams,
) -> Result<ManagedModelCatalogEntry, String> {
    let storage =
        storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut catalog = read_managed_model_catalog_from_storage(&storage)?;
    let normalized_model =
        normalize_model_info(params.model).ok_or_else(|| "模型 slug 不能为空".to_string())?;
    let previous_slug = params
        .previous_slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if current_slug_in_use(&catalog, previous_slug.as_deref(), normalized_model.slug.as_str()) {
        return Err(format!("模型 `{}` 已存在", normalized_model.slug));
    }

    let existing_entry = previous_slug
        .as_ref()
        .and_then(|slug| catalog.items.iter().find(|item| item.model.slug == *slug))
        .cloned()
        .or_else(|| {
            catalog
                .items
                .iter()
                .find(|item| item.model.slug == normalized_model.slug)
                .cloned()
        });

    let next_sort_index = params.sort_index.unwrap_or_else(|| {
        existing_entry
            .as_ref()
            .map(|item| item.sort_index)
            .unwrap_or_else(|| {
                catalog
                    .items
                    .iter()
                    .map(|item| item.sort_index)
                    .max()
                    .unwrap_or(-1)
                    + 1
            })
    });
    let next_entry = ManagedModelCatalogEntry {
        model: normalized_model,
        source_kind: params
            .source_kind
            .as_deref()
            .map(normalize_source_kind)
            .or_else(|| {
                existing_entry
                    .as_ref()
                    .map(|item| normalize_source_kind(item.source_kind.as_str()))
            })
            .unwrap_or_else(|| MODEL_SOURCE_KIND_CUSTOM.to_string()),
        user_edited: params.user_edited.unwrap_or(true),
        sort_index: next_sort_index,
        updated_at: now_ts(),
    };

    if let Some(previous_slug) = previous_slug.as_deref() {
        catalog.items.retain(|item| item.model.slug != previous_slug);
    }
    catalog
        .items
        .retain(|item| item.model.slug != next_entry.model.slug);
    catalog.items.push(next_entry.clone());
    save_managed_model_catalog_with_storage(&storage, &catalog)?;
    Ok(next_entry)
}

pub(crate) fn delete_managed_model_catalog_model(slug: &str) -> Result<(), String> {
    let normalized_slug = slug.trim();
    if normalized_slug.is_empty() {
        return Err("模型 slug 不能为空".to_string());
    }

    let storage =
        storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut catalog = read_managed_model_catalog_from_storage(&storage)?;
    catalog.items.retain(|item| item.model.slug != normalized_slug);
    save_managed_model_catalog_with_storage(&storage, &catalog)
}

fn save_managed_model_catalog_with_storage(
    storage: &Storage,
    catalog: &ManagedModelCatalogResult,
) -> Result<(), String> {
    let normalized = normalize_managed_model_catalog(catalog.clone());
    let updated_at = now_ts();
    storage
        .reset_model_catalog_scope(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?;

    storage
        .upsert_model_catalog_scope(&ModelCatalogScopeRecord {
            scope: MODEL_CACHE_SCOPE_DEFAULT.to_string(),
            extra_json: serialize_extra_map(&normalized.extra)?,
            updated_at,
        })
        .map_err(|e| e.to_string())?;

    let mut model_rows = Vec::new();
    let mut reasoning_rows = Vec::new();
    let mut additional_speed_tiers = Vec::new();
    let mut experimental_supported_tools = Vec::new();
    let mut input_modalities = Vec::new();
    let mut available_in_plans = Vec::new();

    for (index, item) in normalized.items.iter().enumerate() {
        let item_updated_at = if item.updated_at > 0 {
            item.updated_at
        } else {
            updated_at
        };
        let sort_index = if item.sort_index >= 0 {
            item.sort_index
        } else {
            index as i64
        };
        model_rows.push(model_record_from_entry(item, sort_index, item_updated_at)?);
        reasoning_rows.extend(reasoning_records_from_model(&item.model, item_updated_at)?);
        additional_speed_tiers.extend(string_records_from_values(
            &item.model.slug,
            &item.model.additional_speed_tiers,
            item_updated_at,
        ));
        experimental_supported_tools.extend(string_records_from_values(
            &item.model.slug,
            &item.model.experimental_supported_tools,
            item_updated_at,
        ));
        input_modalities.extend(string_records_from_values(
            &item.model.slug,
            &item.model.input_modalities,
            item_updated_at,
        ));
        available_in_plans.extend(string_records_from_values(
            &item.model.slug,
            &item.model.available_in_plans,
            item_updated_at,
        ));
    }

    storage
        .upsert_model_catalog_models(&model_rows)
        .map_err(|e| e.to_string())?;
    storage
        .upsert_model_catalog_reasoning_levels(&reasoning_rows)
        .map_err(|e| e.to_string())?;
    storage
        .upsert_model_catalog_additional_speed_tiers(&additional_speed_tiers)
        .map_err(|e| e.to_string())?;
    storage
        .upsert_model_catalog_experimental_supported_tools(&experimental_supported_tools)
        .map_err(|e| e.to_string())?;
    storage
        .upsert_model_catalog_input_modalities(&input_modalities)
        .map_err(|e| e.to_string())?;
    storage
        .upsert_model_catalog_available_in_plans(&available_in_plans)
        .map_err(|e| e.to_string())?;

    let legacy_items = normalized
        .items
        .iter()
        .map(model_option_from_entry)
        .collect::<Vec<_>>();
    let legacy_json = serde_json::to_string(&legacy_items).map_err(|e| e.to_string())?;
    storage
        .upsert_model_options_cache(MODEL_CACHE_SCOPE_DEFAULT, &legacy_json, updated_at)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_legacy_model_options_catalog(storage: &Storage) -> Result<ManagedModelCatalogResult, String> {
    let Some(cache) = storage
        .get_model_options_cache(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?
    else {
        return Ok(ManagedModelCatalogResult::default());
    };
    let items = serde_json::from_str::<Vec<ModelOption>>(&cache.items_json).unwrap_or_default();
    Ok(legacy_model_options_to_catalog(items))
}

fn legacy_model_options_to_catalog(items: Vec<ModelOption>) -> ManagedModelCatalogResult {
    ManagedModelCatalogResult {
        items: normalize_model_options(items)
            .into_iter()
            .enumerate()
            .map(|(index, item)| ManagedModelCatalogEntry {
                model: model_info_from_option(item),
                source_kind: MODEL_SOURCE_KIND_REMOTE.to_string(),
                user_edited: false,
                sort_index: index as i64,
                updated_at: 0,
            })
            .collect(),
        extra: BTreeMap::new(),
    }
}

fn merge_managed_model_catalog(
    cached: ManagedModelCatalogResult,
    incoming: Vec<ModelOption>,
) -> ManagedModelCatalogResult {
    let cached = normalize_managed_model_catalog(cached);
    let incoming = normalize_model_options(incoming);
    if cached.items.is_empty() {
        return legacy_model_options_to_catalog(incoming);
    }
    if incoming.is_empty() {
        return cached;
    }

    let mut cached_by_slug = BTreeMap::new();
    for item in &cached.items {
        cached_by_slug.insert(item.model.slug.clone(), item.clone());
    }

    let mut merged_items = Vec::new();
    let mut seen = HashSet::new();
    for (index, incoming_model) in incoming.into_iter().enumerate() {
        let slug = incoming_model.slug.clone();
        let merged_item = match cached_by_slug.get(&slug) {
            Some(cached_item) if cached_item.user_edited => {
                let mut preserved = cached_item.clone();
                if preserved.sort_index < 0 {
                    preserved.sort_index = index as i64;
                }
                preserved
            }
            Some(cached_item) => ManagedModelCatalogEntry {
                model: merge_cached_model_with_remote(cached_item.model.clone(), incoming_model),
                source_kind: normalize_source_kind(cached_item.source_kind.as_str()),
                user_edited: false,
                sort_index: cached_item.sort_index,
                updated_at: cached_item.updated_at,
            },
            None => ManagedModelCatalogEntry {
                model: model_info_from_option(incoming_model),
                source_kind: MODEL_SOURCE_KIND_REMOTE.to_string(),
                user_edited: false,
                sort_index: index as i64,
                updated_at: 0,
            },
        };
        seen.insert(slug);
        merged_items.push(merged_item);
    }

    for cached_item in cached.items {
        if seen.insert(cached_item.model.slug.clone()) {
            merged_items.push(cached_item);
        }
    }

    normalize_managed_model_catalog(ManagedModelCatalogResult {
        items: merged_items,
        extra: cached.extra,
    })
}

fn normalize_managed_model_catalog(
    catalog: ManagedModelCatalogResult,
) -> ManagedModelCatalogResult {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for item in catalog.items {
        let Some(model) = normalize_model_info(item.model) else {
            continue;
        };
        if !seen.insert(model.slug.clone()) {
            continue;
        }
        items.push(ManagedModelCatalogEntry {
            model,
            source_kind: normalize_source_kind(item.source_kind.as_str()),
            user_edited: item.user_edited,
            sort_index: item.sort_index,
            updated_at: item.updated_at,
        });
    }
    ManagedModelCatalogResult {
        items,
        extra: catalog.extra,
    }
}

fn normalize_model_options(items: Vec<ModelOption>) -> Vec<ModelOption> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for item in items {
        let slug = item.slug.trim().to_string();
        if slug.is_empty() || !seen.insert(slug.clone()) {
            continue;
        }
        let display_name = if item.display_name.trim().is_empty() {
            slug.clone()
        } else {
            item.display_name.trim().to_string()
        };
        normalized.push(ModelOption { slug, display_name });
    }
    normalized
}

fn normalize_model_info(mut model: ModelInfo) -> Option<ModelInfo> {
    let slug = model.slug.trim().to_string();
    if slug.is_empty() {
        return None;
    }

    model.slug = slug;
    if model.display_name.trim().is_empty() {
        model.display_name = model.slug.clone();
    } else {
        model.display_name = model.display_name.trim().to_string();
    }
    model.visibility = normalize_visibility(model.visibility);
    if model.input_modalities.is_empty() {
        model.input_modalities = vec!["text".to_string()];
    } else {
        model.input_modalities = normalize_string_vec(model.input_modalities);
    }
    model.additional_speed_tiers = normalize_string_vec(model.additional_speed_tiers);
    model.experimental_supported_tools = normalize_string_vec(model.experimental_supported_tools);
    model.available_in_plans = normalize_string_vec(model.available_in_plans);
    Some(model)
}

fn normalize_source_kind(source_kind: &str) -> String {
    match source_kind.trim() {
        MODEL_SOURCE_KIND_CUSTOM => MODEL_SOURCE_KIND_CUSTOM.to_string(),
        _ => MODEL_SOURCE_KIND_REMOTE.to_string(),
    }
}

fn normalize_visibility(value: Option<String>) -> Option<String> {
    let normalized = value
        .as_deref()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_ascii_lowercase())?;
    match normalized.as_str() {
        "hidden" => Some("hide".to_string()),
        _ => Some(normalized),
    }
}

fn normalize_string_vec(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        normalized.push(value);
    }
    normalized
}

fn model_info_from_option(item: ModelOption) -> ModelInfo {
    ModelInfo {
        slug: item.slug,
        display_name: item.display_name,
        input_modalities: vec!["text".to_string()],
        supported_in_api: true,
        ..Default::default()
    }
}

fn merge_cached_model_with_remote(mut cached: ModelInfo, incoming: ModelOption) -> ModelInfo {
    cached.slug = incoming.slug;
    if !incoming.display_name.trim().is_empty() {
        cached.display_name = incoming.display_name;
    }
    normalize_model_info(cached).unwrap_or_default()
}

fn model_option_from_entry(item: &ManagedModelCatalogEntry) -> ModelOption {
    ModelOption {
        slug: item.model.slug.clone(),
        display_name: item.model.display_name.clone(),
    }
}

fn current_slug_in_use(
    catalog: &ManagedModelCatalogResult,
    previous_slug: Option<&str>,
    next_slug: &str,
) -> bool {
    catalog.items.iter().any(|item| {
        item.model.slug == next_slug && previous_slug.map(|slug| slug != next_slug).unwrap_or(true)
    })
}

fn model_record_from_entry(
    item: &ManagedModelCatalogEntry,
    sort_index: i64,
    updated_at: i64,
) -> Result<ModelCatalogModelRecord, String> {
    let model = &item.model;
    let truncation_extra_json = model
        .truncation_policy
        .as_ref()
        .map(|policy| serialize_extra_map(&policy.extra))
        .transpose()?;
    Ok(ModelCatalogModelRecord {
        scope: MODEL_CACHE_SCOPE_DEFAULT.to_string(),
        slug: model.slug.clone(),
        display_name: model.display_name.clone(),
        source_kind: normalize_source_kind(item.source_kind.as_str()),
        user_edited: item.user_edited,
        description: model.description.clone(),
        default_reasoning_level: model.default_reasoning_level.clone(),
        shell_type: model.shell_type.clone(),
        visibility: model.visibility.clone(),
        supported_in_api: Some(model.supported_in_api),
        priority: Some(model.priority),
        availability_nux_json: serialize_json_option(&model.availability_nux)?,
        upgrade_json: serialize_json_option(&model.upgrade)?,
        base_instructions: model.base_instructions.clone(),
        model_messages_json: serialize_json_option(&model.model_messages)?,
        supports_reasoning_summaries: model.supports_reasoning_summaries,
        default_reasoning_summary: model.default_reasoning_summary.clone(),
        support_verbosity: model.support_verbosity,
        default_verbosity_json: serialize_json_option(&model.default_verbosity)?,
        apply_patch_tool_type: model.apply_patch_tool_type.clone(),
        web_search_tool_type: model.web_search_tool_type.clone(),
        truncation_mode: model
            .truncation_policy
            .as_ref()
            .map(|policy| policy.mode.clone()),
        truncation_limit: model.truncation_policy.as_ref().map(|policy| policy.limit),
        truncation_extra_json,
        supports_parallel_tool_calls: model.supports_parallel_tool_calls,
        supports_image_detail_original: model.supports_image_detail_original,
        context_window: model.context_window,
        auto_compact_token_limit: model.auto_compact_token_limit,
        effective_context_window_percent: model.effective_context_window_percent,
        minimal_client_version_json: serialize_json_option(&model.minimal_client_version)?,
        supports_search_tool: model.supports_search_tool,
        extra_json: serialize_extra_map(&model.extra)?,
        sort_index,
        updated_at,
    })
}

fn reasoning_records_from_model(
    model: &ModelInfo,
    updated_at: i64,
) -> Result<Vec<ModelCatalogReasoningLevelRecord>, String> {
    let mut records = Vec::new();
    for (index, level) in model.supported_reasoning_levels.iter().enumerate() {
        records.push(ModelCatalogReasoningLevelRecord {
            scope: MODEL_CACHE_SCOPE_DEFAULT.to_string(),
            slug: model.slug.clone(),
            effort: level.effort.clone(),
            description: level.description.clone(),
            extra_json: serialize_extra_map(&level.extra)?,
            sort_index: index as i64,
            updated_at,
        });
    }
    Ok(records)
}

fn string_records_from_values(
    slug: &str,
    values: &[String],
    updated_at: i64,
) -> Vec<ModelCatalogStringItemRecord> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| ModelCatalogStringItemRecord {
            scope: MODEL_CACHE_SCOPE_DEFAULT.to_string(),
            slug: slug.to_string(),
            value: value.clone(),
            sort_index: index as i64,
            updated_at,
        })
        .collect()
}

fn managed_catalog_entry_from_row(
    row: ModelCatalogModelRecord,
    reasoning_levels: Option<Vec<ModelReasoningLevel>>,
    additional_speed_tiers: Option<Vec<String>>,
    experimental_supported_tools: Option<Vec<String>>,
    input_modalities: Option<Vec<String>>,
    available_in_plans: Option<Vec<String>>,
) -> Option<ManagedModelCatalogEntry> {
    let source_kind = normalize_source_kind(row.source_kind.as_str());
    let user_edited = row.user_edited;
    let sort_index = row.sort_index;
    let updated_at = row.updated_at;
    model_info_from_row(
        row,
        reasoning_levels,
        additional_speed_tiers,
        experimental_supported_tools,
        input_modalities,
        available_in_plans,
    )
    .map(|model| ManagedModelCatalogEntry {
        model,
        source_kind,
        user_edited,
        sort_index,
        updated_at,
    })
}

fn model_info_from_row(
    row: ModelCatalogModelRecord,
    reasoning_levels: Option<Vec<ModelReasoningLevel>>,
    additional_speed_tiers: Option<Vec<String>>,
    experimental_supported_tools: Option<Vec<String>>,
    input_modalities: Option<Vec<String>>,
    available_in_plans: Option<Vec<String>>,
) -> Option<ModelInfo> {
    let mut model = ModelInfo {
        slug: row.slug.clone(),
        display_name: row.display_name.clone(),
        supported_in_api: row.supported_in_api.unwrap_or(true),
        priority: row.priority.unwrap_or_default(),
        extra: parse_extra_json_map(Some(row.extra_json.as_str())).unwrap_or_default(),
        ..Default::default()
    };

    model.description = row.description;
    model.default_reasoning_level = row.default_reasoning_level;
    model.shell_type = row.shell_type;
    model.visibility = row.visibility;
    model.availability_nux = parse_json_value(row.availability_nux_json.as_deref());
    model.upgrade = parse_json_value(row.upgrade_json.as_deref());
    model.base_instructions = row.base_instructions;
    model.model_messages = parse_json_value(row.model_messages_json.as_deref());
    model.supports_reasoning_summaries = row.supports_reasoning_summaries;
    model.default_reasoning_summary = row.default_reasoning_summary;
    model.support_verbosity = row.support_verbosity;
    model.default_verbosity = parse_json_value(row.default_verbosity_json.as_deref());
    model.apply_patch_tool_type = row.apply_patch_tool_type;
    model.web_search_tool_type = row.web_search_tool_type;
    model.truncation_policy = build_truncation_policy(
        row.truncation_mode.as_deref(),
        row.truncation_limit,
        row.truncation_extra_json.as_deref(),
    );
    model.supports_parallel_tool_calls = row.supports_parallel_tool_calls;
    model.supports_image_detail_original = row.supports_image_detail_original;
    model.context_window = row.context_window;
    model.auto_compact_token_limit = row.auto_compact_token_limit;
    model.effective_context_window_percent = row.effective_context_window_percent;
    model.minimal_client_version = parse_json_value(row.minimal_client_version_json.as_deref());
    model.supports_search_tool = row.supports_search_tool;
    model.supported_reasoning_levels = reasoning_levels.unwrap_or_default();
    model.additional_speed_tiers = additional_speed_tiers.unwrap_or_default();
    model.experimental_supported_tools = experimental_supported_tools.unwrap_or_default();
    model.input_modalities = input_modalities.unwrap_or_else(|| vec!["text".to_string()]);
    model.available_in_plans = available_in_plans.unwrap_or_default();
    normalize_model_info(model)
}

fn serialize_json_option(value: &Option<Value>) -> Result<Option<String>, String> {
    value
        .as_ref()
        .map(|item| serde_json::to_string(item).map_err(|e| e.to_string()))
        .transpose()
}

fn serialize_extra_map(extra: &BTreeMap<String, Value>) -> Result<String, String> {
    serde_json::to_string(extra).map_err(|e| e.to_string())
}

fn parse_json_value(raw: Option<&str>) -> Option<Value> {
    raw.and_then(|item| serde_json::from_str::<Value>(item).ok())
}

fn parse_extra_json_map(raw: Option<&str>) -> Option<BTreeMap<String, Value>> {
    raw.and_then(|item| serde_json::from_str::<BTreeMap<String, Value>>(item).ok())
}

fn build_truncation_policy(
    mode: Option<&str>,
    limit: Option<i64>,
    extra_json: Option<&str>,
) -> Option<ModelTruncationPolicy> {
    if mode.is_none() && limit.is_none() && extra_json.is_none() {
        return None;
    }
    Some(ModelTruncationPolicy {
        mode: mode.unwrap_or("auto").to_string(),
        limit: limit.unwrap_or_default(),
        extra: parse_extra_json_map(extra_json).unwrap_or_default(),
    })
}

fn group_reasoning_levels_by_slug(
    records: Vec<ModelCatalogReasoningLevelRecord>,
) -> BTreeMap<String, Vec<ModelReasoningLevel>> {
    let mut grouped = BTreeMap::new();
    for record in records {
        grouped
            .entry(record.slug)
            .or_insert_with(Vec::new)
            .push(ModelReasoningLevel {
                effort: record.effort,
                description: record.description,
                extra: parse_extra_json_map(Some(record.extra_json.as_str())).unwrap_or_default(),
            });
    }
    grouped
}

fn group_string_items_by_slug(
    records: Vec<ModelCatalogStringItemRecord>,
) -> BTreeMap<String, Vec<String>> {
    let mut grouped = BTreeMap::new();
    for record in records {
        grouped
            .entry(record.slug)
            .or_insert_with(Vec::new)
            .push(record.value);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use codexmanager_core::rpc::types::{ManagedModelCatalogResult, ModelOption};

    use super::{merge_managed_model_catalog, model_option_from_entry};

    #[test]
    fn merge_managed_model_catalog_preserves_custom_models() {
        let cached = ManagedModelCatalogResult {
            items: vec![serde_json::from_value(serde_json::json!({
                "slug": "gpt-5-custom",
                "displayName": "GPT-5 Custom",
                "sourceKind": "custom",
                "userEdited": true,
                "sortIndex": 9
            }))
            .expect("custom catalog entry")],
            extra: Default::default(),
        };

        let merged = merge_managed_model_catalog(
            cached,
            vec![ModelOption {
                slug: "gpt-5".to_string(),
                display_name: "GPT-5".to_string(),
            }],
        );

        let slugs = merged
            .items
            .iter()
            .map(model_option_from_entry)
            .map(|item| item.slug)
            .collect::<Vec<_>>();
        assert_eq!(slugs, vec!["gpt-5".to_string(), "gpt-5-custom".to_string()]);
    }
}
