use rusqlite::params;

use super::{
    ModelCatalogModelRecord, ModelCatalogReasoningLevelRecord, ModelCatalogScopeRecord,
    ModelCatalogStringItemRecord, ModelOptionsCacheRecord, Storage,
};

const STRING_ITEM_KIND_ADDITIONAL_SPEED_TIERS: &str = "additional_speed_tiers";
const STRING_ITEM_KIND_EXPERIMENTAL_SUPPORTED_TOOLS: &str = "experimental_supported_tools";
const STRING_ITEM_KIND_INPUT_MODALITIES: &str = "input_modalities";
const STRING_ITEM_KIND_AVAILABLE_IN_PLANS: &str = "available_in_plans";

impl Storage {
    pub fn upsert_model_options_cache(
        &self,
        scope: &str,
        items_json: &str,
        updated_at: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO model_options_cache (scope, items_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scope) DO UPDATE SET
               items_json = excluded.items_json,
               updated_at = excluded.updated_at",
            params![scope, items_json, updated_at],
        )?;
        Ok(())
    }

    pub fn get_model_options_cache(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Option<ModelOptionsCacheRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, items_json, updated_at
             FROM model_options_cache
             WHERE scope = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([scope])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(ModelOptionsCacheRecord {
                scope: row.get(0)?,
                items_json: row.get(1)?,
                updated_at: row.get(2)?,
            }));
        }
        Ok(None)
    }

    pub fn upsert_model_catalog_scope(
        &self,
        record: &ModelCatalogScopeRecord,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO model_catalog_scopes (scope, extra_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scope) DO UPDATE SET
               extra_json = excluded.extra_json,
               updated_at = excluded.updated_at",
            params![record.scope, record.extra_json, record.updated_at],
        )?;
        Ok(())
    }

    pub fn get_model_catalog_scope(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Option<ModelCatalogScopeRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, extra_json, updated_at
             FROM model_catalog_scopes
             WHERE scope = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([scope])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(ModelCatalogScopeRecord {
                scope: row.get(0)?,
                extra_json: row.get(1)?,
                updated_at: row.get(2)?,
            }));
        }
        Ok(None)
    }

    pub fn reset_model_catalog_scope(&self, scope: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM model_catalog_string_items WHERE scope = ?1",
            params![scope],
        )?;
        self.conn.execute(
            "DELETE FROM model_catalog_reasoning_levels WHERE scope = ?1",
            params![scope],
        )?;
        self.conn.execute(
            "DELETE FROM model_catalog_models WHERE scope = ?1",
            params![scope],
        )?;
        self.conn.execute(
            "DELETE FROM model_catalog_scopes WHERE scope = ?1",
            params![scope],
        )?;
        Ok(())
    }

    pub fn upsert_model_catalog_models(
        &self,
        models: &[ModelCatalogModelRecord],
    ) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for model in models {
            tx.execute(
                "INSERT INTO model_catalog_models (
                    scope, slug, display_name, source_kind, user_edited,
                    description, default_reasoning_level, shell_type, visibility, supported_in_api, priority,
                    availability_nux_json, upgrade_json, base_instructions,
                    model_messages_json, supports_reasoning_summaries,
                    default_reasoning_summary, support_verbosity,
                    default_verbosity_json, apply_patch_tool_type,
                    web_search_tool_type, truncation_mode, truncation_limit,
                    truncation_extra_json, supports_parallel_tool_calls,
                    supports_image_detail_original, context_window,
                    auto_compact_token_limit, effective_context_window_percent,
                    minimal_client_version_json, supports_search_tool,
                    extra_json, sort_index, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24,
                    ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34
                 )
                 ON CONFLICT(scope, slug) DO UPDATE SET
                    display_name = excluded.display_name,
                    source_kind = excluded.source_kind,
                    user_edited = excluded.user_edited,
                    description = excluded.description,
                    default_reasoning_level = excluded.default_reasoning_level,
                    shell_type = excluded.shell_type,
                    visibility = excluded.visibility,
                    supported_in_api = excluded.supported_in_api,
                    priority = excluded.priority,
                    availability_nux_json = excluded.availability_nux_json,
                    upgrade_json = excluded.upgrade_json,
                    base_instructions = excluded.base_instructions,
                    model_messages_json = excluded.model_messages_json,
                    supports_reasoning_summaries = excluded.supports_reasoning_summaries,
                    default_reasoning_summary = excluded.default_reasoning_summary,
                    support_verbosity = excluded.support_verbosity,
                    default_verbosity_json = excluded.default_verbosity_json,
                    apply_patch_tool_type = excluded.apply_patch_tool_type,
                    web_search_tool_type = excluded.web_search_tool_type,
                    truncation_mode = excluded.truncation_mode,
                    truncation_limit = excluded.truncation_limit,
                    truncation_extra_json = excluded.truncation_extra_json,
                    supports_parallel_tool_calls = excluded.supports_parallel_tool_calls,
                    supports_image_detail_original = excluded.supports_image_detail_original,
                    context_window = excluded.context_window,
                    auto_compact_token_limit = excluded.auto_compact_token_limit,
                    effective_context_window_percent = excluded.effective_context_window_percent,
                    minimal_client_version_json = excluded.minimal_client_version_json,
                    supports_search_tool = excluded.supports_search_tool,
                    extra_json = excluded.extra_json,
                    sort_index = excluded.sort_index,
                    updated_at = excluded.updated_at",
                params![
                    model.scope,
                    model.slug,
                    model.display_name,
                    model.source_kind,
                    model.user_edited,
                    model.description,
                    model.default_reasoning_level,
                    model.shell_type,
                    model.visibility,
                    model.supported_in_api,
                    model.priority,
                    model.availability_nux_json,
                    model.upgrade_json,
                    model.base_instructions,
                    model.model_messages_json,
                    model.supports_reasoning_summaries,
                    model.default_reasoning_summary,
                    model.support_verbosity,
                    model.default_verbosity_json,
                    model.apply_patch_tool_type,
                    model.web_search_tool_type,
                    model.truncation_mode,
                    model.truncation_limit,
                    model.truncation_extra_json,
                    model.supports_parallel_tool_calls,
                    model.supports_image_detail_original,
                    model.context_window,
                    model.auto_compact_token_limit,
                    model.effective_context_window_percent,
                    model.minimal_client_version_json,
                    model.supports_search_tool,
                    model.extra_json,
                    model.sort_index,
                    model.updated_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_model_catalog_models(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogModelRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                scope, slug, display_name, source_kind, user_edited,
                description, default_reasoning_level, shell_type, visibility, supported_in_api, priority,
                availability_nux_json, upgrade_json, base_instructions,
                model_messages_json, supports_reasoning_summaries,
                default_reasoning_summary, support_verbosity,
                default_verbosity_json, apply_patch_tool_type,
                web_search_tool_type, truncation_mode, truncation_limit,
                truncation_extra_json, supports_parallel_tool_calls,
                supports_image_detail_original, context_window,
                auto_compact_token_limit, effective_context_window_percent,
                minimal_client_version_json, supports_search_tool,
                extra_json, sort_index, updated_at
             FROM model_catalog_models
             WHERE scope = ?1
             ORDER BY sort_index ASC, updated_at DESC, slug ASC",
        )?;
        let rows = stmt.query_map([scope], |row| {
            Ok(ModelCatalogModelRecord {
                scope: row.get(0)?,
                slug: row.get(1)?,
                display_name: row.get(2)?,
                source_kind: row.get(3)?,
                user_edited: row.get(4)?,
                description: row.get(5)?,
                default_reasoning_level: row.get(6)?,
                shell_type: row.get(7)?,
                visibility: row.get(8)?,
                supported_in_api: row.get(9)?,
                priority: row.get(10)?,
                availability_nux_json: row.get(11)?,
                upgrade_json: row.get(12)?,
                base_instructions: row.get(13)?,
                model_messages_json: row.get(14)?,
                supports_reasoning_summaries: row.get(15)?,
                default_reasoning_summary: row.get(16)?,
                support_verbosity: row.get(17)?,
                default_verbosity_json: row.get(18)?,
                apply_patch_tool_type: row.get(19)?,
                web_search_tool_type: row.get(20)?,
                truncation_mode: row.get(21)?,
                truncation_limit: row.get(22)?,
                truncation_extra_json: row.get(23)?,
                supports_parallel_tool_calls: row.get(24)?,
                supports_image_detail_original: row.get(25)?,
                context_window: row.get(26)?,
                auto_compact_token_limit: row.get(27)?,
                effective_context_window_percent: row.get(28)?,
                minimal_client_version_json: row.get(29)?,
                supports_search_tool: row.get(30)?,
                extra_json: row.get(31)?,
                sort_index: row.get(32)?,
                updated_at: row.get(33)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn delete_model_catalog_model(&self, scope: &str, slug: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM model_catalog_models WHERE scope = ?1 AND slug = ?2",
            params![scope, slug],
        )?;
        Ok(())
    }

    pub fn upsert_model_catalog_reasoning_levels(
        &self,
        levels: &[ModelCatalogReasoningLevelRecord],
    ) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for level in levels {
            tx.execute(
                "INSERT INTO model_catalog_reasoning_levels (
                    scope, slug, effort, description, extra_json, sort_index, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(scope, slug, effort) DO UPDATE SET
                    description = excluded.description,
                    extra_json = excluded.extra_json,
                    sort_index = excluded.sort_index,
                    updated_at = excluded.updated_at",
                params![
                    level.scope,
                    level.slug,
                    level.effort,
                    level.description,
                    level.extra_json,
                    level.sort_index,
                    level.updated_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_model_catalog_reasoning_levels(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogReasoningLevelRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, slug, effort, description, extra_json, sort_index, updated_at
             FROM model_catalog_reasoning_levels
             WHERE scope = ?1
             ORDER BY slug ASC, sort_index ASC, effort ASC",
        )?;
        let rows = stmt.query_map([scope], |row| {
            Ok(ModelCatalogReasoningLevelRecord {
                scope: row.get(0)?,
                slug: row.get(1)?,
                effort: row.get(2)?,
                description: row.get(3)?,
                extra_json: row.get(4)?,
                sort_index: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn delete_model_catalog_reasoning_levels(
        &self,
        scope: &str,
        slug: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM model_catalog_reasoning_levels WHERE scope = ?1 AND slug = ?2",
            params![scope, slug],
        )?;
        Ok(())
    }

    pub fn upsert_model_catalog_additional_speed_tiers(
        &self,
        items: &[ModelCatalogStringItemRecord],
    ) -> rusqlite::Result<()> {
        self.upsert_model_catalog_string_items(STRING_ITEM_KIND_ADDITIONAL_SPEED_TIERS, items)
    }

    pub fn list_model_catalog_additional_speed_tiers(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogStringItemRecord>> {
        self.list_model_catalog_string_items(STRING_ITEM_KIND_ADDITIONAL_SPEED_TIERS, scope)
    }

    pub fn upsert_model_catalog_experimental_supported_tools(
        &self,
        items: &[ModelCatalogStringItemRecord],
    ) -> rusqlite::Result<()> {
        self.upsert_model_catalog_string_items(STRING_ITEM_KIND_EXPERIMENTAL_SUPPORTED_TOOLS, items)
    }

    pub fn list_model_catalog_experimental_supported_tools(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogStringItemRecord>> {
        self.list_model_catalog_string_items(STRING_ITEM_KIND_EXPERIMENTAL_SUPPORTED_TOOLS, scope)
    }

    pub fn upsert_model_catalog_input_modalities(
        &self,
        items: &[ModelCatalogStringItemRecord],
    ) -> rusqlite::Result<()> {
        self.upsert_model_catalog_string_items(STRING_ITEM_KIND_INPUT_MODALITIES, items)
    }

    pub fn list_model_catalog_input_modalities(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogStringItemRecord>> {
        self.list_model_catalog_string_items(STRING_ITEM_KIND_INPUT_MODALITIES, scope)
    }

    pub fn upsert_model_catalog_available_in_plans(
        &self,
        items: &[ModelCatalogStringItemRecord],
    ) -> rusqlite::Result<()> {
        self.upsert_model_catalog_string_items(STRING_ITEM_KIND_AVAILABLE_IN_PLANS, items)
    }

    pub fn list_model_catalog_available_in_plans(
        &self,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogStringItemRecord>> {
        self.list_model_catalog_string_items(STRING_ITEM_KIND_AVAILABLE_IN_PLANS, scope)
    }

    pub fn delete_model_catalog_string_items(
        &self,
        scope: &str,
        slug: &str,
        item_kind: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM model_catalog_string_items
             WHERE scope = ?1 AND slug = ?2 AND item_kind = ?3",
            params![scope, slug, item_kind],
        )?;
        Ok(())
    }

    fn upsert_model_catalog_string_items(
        &self,
        item_kind: &str,
        items: &[ModelCatalogStringItemRecord],
    ) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for item in items {
            tx.execute(
                "INSERT INTO model_catalog_string_items
                    (scope, slug, item_kind, value, sort_index, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(scope, slug, item_kind, value) DO UPDATE SET
                    sort_index = excluded.sort_index,
                    updated_at = excluded.updated_at",
                params![
                    item.scope,
                    item.slug,
                    item_kind,
                    item.value,
                    item.sort_index,
                    item.updated_at
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn list_model_catalog_string_items(
        &self,
        item_kind: &str,
        scope: &str,
    ) -> rusqlite::Result<Vec<ModelCatalogStringItemRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, slug, value, sort_index, updated_at
             FROM model_catalog_string_items
             WHERE item_kind = ?1 AND scope = ?2
             ORDER BY slug ASC, sort_index ASC, value ASC",
        )?;
        let rows = stmt.query_map(params![item_kind, scope], |row| {
            Ok(ModelCatalogStringItemRecord {
                scope: row.get(0)?,
                slug: row.get(1)?,
                value: row.get(2)?,
                sort_index: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }
}
