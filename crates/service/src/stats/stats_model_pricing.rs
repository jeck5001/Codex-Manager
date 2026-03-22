use codexmanager_core::rpc::types::{ModelPricingItem, ModelPricingListResult};
use codexmanager_core::storage::ModelPricing;

use crate::storage_helpers::open_storage;

pub(crate) fn read_model_pricing() -> Result<ModelPricingListResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let items = storage
        .list_model_pricing()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|item| ModelPricingItem {
            model_slug: item.model_slug,
            input_price_per_1k: item.input_price_per_1k,
            output_price_per_1k: item.output_price_per_1k,
            updated_at: Some(item.updated_at),
        })
        .collect::<Vec<_>>();
    Ok(ModelPricingListResult { items })
}

pub(crate) fn update_model_pricing(items: Vec<ModelPricingItem>) -> Result<(), String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let model_slug = item.model_slug.trim().to_string();
        if model_slug.is_empty() {
            return Err("modelSlug required".to_string());
        }
        if !item.input_price_per_1k.is_finite() || item.input_price_per_1k < 0.0 {
            return Err(format!("inputPricePer1k invalid for model {}", model_slug));
        }
        if !item.output_price_per_1k.is_finite() || item.output_price_per_1k < 0.0 {
            return Err(format!("outputPricePer1k invalid for model {}", model_slug));
        }
        normalized.push(ModelPricing {
            model_slug,
            input_price_per_1k: item.input_price_per_1k,
            output_price_per_1k: item.output_price_per_1k,
            updated_at: item
                .updated_at
                .unwrap_or_else(codexmanager_core::storage::now_ts),
        });
    }

    normalized.sort_by(|left, right| left.model_slug.cmp(&right.model_slug));
    normalized.dedup_by(|left, right| left.model_slug == right.model_slug);
    storage
        .replace_model_pricing(normalized.as_slice())
        .map_err(|err| err.to_string())
}
