use super::{
    AccountListParams, AccountListResult, AccountSummary, GovernanceSummaryItem,
    OperationAuditItem,
    RequestLogFilterSummaryResult, RequestLogListParams, RequestLogListResult, RequestLogSummary,
};

#[test]
fn account_summary_serialization_matches_compact_contract() {
    let summary = AccountSummary {
        id: "acc-1".to_string(),
        label: "主账号".to_string(),
        group_name: Some("TEAM".to_string()),
        tags: vec!["free".to_string(), "imported".to_string()],
        sort: 10,
        status: "active".to_string(),
        health_score: 108,
        last_status_reason: Some("手动禁用".to_string()),
        last_status_changed_at: Some(11),
        last_governance_reason: Some("Refresh 连续失效".to_string()),
        last_governance_at: Some(12),
        last_isolation_reason_code: Some("refresh_token".to_string()),
        last_isolation_reason: Some("Refresh 连续失效".to_string()),
        last_isolation_at: Some(13),
        cooldown_until: Some(14),
        cooldown_reason_code: Some("rate_limited".to_string()),
        cooldown_reason: Some("速率限制".to_string()),
        subscription_plan_type: Some("plus".to_string()),
        subscription_updated_at: Some(1),
        team_manager_uploaded_at: Some(2),
        official_promo_link: Some("https://example.com".to_string()),
        official_promo_link_updated_at: Some(3),
    };

    let value = serde_json::to_value(summary).expect("serialize account summary");
    let obj = value.as_object().expect("account summary object");

    for key in [
        "id",
        "label",
        "groupName",
        "tags",
        "sort",
        "status",
        "healthScore",
        "lastStatusReason",
        "lastStatusChangedAt",
        "lastGovernanceReason",
        "lastGovernanceAt",
        "lastIsolationReasonCode",
        "lastIsolationReason",
        "lastIsolationAt",
        "cooldownUntil",
        "cooldownReasonCode",
        "cooldownReason",
        "subscriptionPlanType",
        "subscriptionUpdatedAt",
        "teamManagerUploadedAt",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }

    for key in ["workspaceId", "workspaceName", "note", "tags", "updatedAt"] {
        assert!(!obj.contains_key(key), "unexpected key: {key}");
    }
}

#[test]
fn account_list_params_default_to_first_page_with_five_items() {
    let params: AccountListParams =
        serde_json::from_value(serde_json::json!({})).expect("deserialize params");
    let normalized = params.normalized();

    assert_eq!(normalized.page, 1);
    assert_eq!(normalized.page_size, 5);
}

#[test]
fn account_list_result_serialization_includes_pagination_fields() {
    let result = AccountListResult {
        items: vec![AccountSummary {
            id: "acc-1".to_string(),
            label: "主账号".to_string(),
            group_name: Some("TEAM".to_string()),
            tags: vec!["team".to_string()],
            sort: 10,
            status: "active".to_string(),
            health_score: 108,
            last_status_reason: Some("手动禁用".to_string()),
            last_status_changed_at: Some(11),
            last_governance_reason: Some("Refresh 连续失效".to_string()),
            last_governance_at: Some(12),
            last_isolation_reason_code: Some("refresh_token".to_string()),
            last_isolation_reason: Some("Refresh 连续失效".to_string()),
            last_isolation_at: Some(13),
            cooldown_until: Some(14),
            cooldown_reason_code: Some("rate_limited".to_string()),
            cooldown_reason: Some("速率限制".to_string()),
            subscription_plan_type: Some("team".to_string()),
            subscription_updated_at: Some(1),
            team_manager_uploaded_at: Some(2),
            official_promo_link: Some("https://example.com".to_string()),
            official_promo_link_updated_at: Some(3),
        }],
        total: 9,
        page: 2,
        page_size: 3,
    };

    let value = serde_json::to_value(result).expect("serialize account list result");
    let obj = value.as_object().expect("account list result object");
    for key in ["items", "total", "page", "pageSize"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn governance_summary_item_serialization_uses_camel_case() {
    let result = GovernanceSummaryItem {
        code: "refresh_token_disabled".to_string(),
        label: "Refresh 连续失效".to_string(),
        target_status: "disabled".to_string(),
        count: 2,
        affected_accounts: 2,
        last_seen_at: Some(1),
    };

    let value = serde_json::to_value(result).expect("serialize governance summary");
    let obj = value.as_object().expect("governance summary object");
    for key in [
        "code",
        "label",
        "targetStatus",
        "count",
        "affectedAccounts",
        "lastSeenAt",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn operation_audit_item_serialization_uses_camel_case() {
    let result = OperationAuditItem {
        action: "freeproxy_sync".to_string(),
        label: "同步 freeproxy 代理池".to_string(),
        detail: "已同步 20 个代理".to_string(),
        account_id: None,
        created_at: Some(1),
    };

    let value = serde_json::to_value(result).expect("serialize operation audit item");
    let obj = value.as_object().expect("operation audit object");
    for key in ["action", "label", "detail", "accountId", "createdAt"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn request_log_summary_serialization_includes_trace_route_fields() {
    let summary = RequestLogSummary {
        trace_id: Some("trc_1".to_string()),
        key_id: Some("gk_1".to_string()),
        account_id: Some("acc_1".to_string()),
        initial_account_id: Some("acc_free".to_string()),
        attempted_account_ids: vec!["acc_free".to_string(), "acc_1".to_string()],
        request_path: "/v1/responses".to_string(),
        original_path: Some("/v1/chat/completions".to_string()),
        adapted_path: Some("/v1/responses".to_string()),
        method: "POST".to_string(),
        model: Some("gpt-5.3-codex".to_string()),
        reasoning_effort: Some("high".to_string()),
        response_adapter: Some("OpenAIChatCompletionsJson".to_string()),
        upstream_url: Some("https://api.openai.com/v1".to_string()),
        status_code: Some(502),
        duration_ms: Some(1450),
        input_tokens: Some(10),
        cached_input_tokens: Some(0),
        output_tokens: Some(3),
        total_tokens: Some(13),
        reasoning_output_tokens: Some(1),
        estimated_cost_usd: Some(0.12),
        error: Some("internal_error".to_string()),
        created_at: 1,
    };

    let value = serde_json::to_value(summary).expect("serialize request log summary");
    let obj = value.as_object().expect("request log summary object");
    for key in [
        "traceId",
        "initialAccountId",
        "attemptedAccountIds",
        "originalPath",
        "adaptedPath",
        "responseAdapter",
        "requestPath",
        "upstreamUrl",
        "durationMs",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn request_log_list_params_default_to_first_page_with_twenty_items() {
    let params: RequestLogListParams =
        serde_json::from_value(serde_json::json!({})).expect("deserialize params");
    let normalized = params.normalized();

    assert_eq!(normalized.page, 1);
    assert_eq!(normalized.page_size, 20);
}

#[test]
fn request_log_list_result_serialization_includes_pagination_fields() {
    let result = RequestLogListResult {
        items: vec![RequestLogSummary {
            trace_id: Some("trc_1".to_string()),
            key_id: Some("gk_1".to_string()),
            account_id: Some("acc_1".to_string()),
            initial_account_id: Some("acc_free".to_string()),
            attempted_account_ids: vec!["acc_free".to_string(), "acc_1".to_string()],
            request_path: "/v1/responses".to_string(),
            original_path: Some("/v1/chat/completions".to_string()),
            adapted_path: Some("/v1/responses".to_string()),
            method: "POST".to_string(),
            model: Some("gpt-5.3-codex".to_string()),
            reasoning_effort: Some("high".to_string()),
            response_adapter: Some("OpenAIChatCompletionsJson".to_string()),
            upstream_url: Some("https://api.openai.com/v1".to_string()),
            status_code: Some(200),
            duration_ms: Some(1200),
            input_tokens: Some(10),
            cached_input_tokens: Some(1),
            output_tokens: Some(2),
            total_tokens: Some(12),
            reasoning_output_tokens: Some(1),
            estimated_cost_usd: Some(0.12),
            error: None,
            created_at: 1,
        }],
        total: 88,
        page: 3,
        page_size: 25,
    };

    let value = serde_json::to_value(result).expect("serialize request log list result");
    let obj = value.as_object().expect("request log list result object");
    for key in ["items", "total", "page", "pageSize"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn request_log_filter_summary_serialization_uses_camel_case() {
    let result = RequestLogFilterSummaryResult {
        total_count: 120,
        filtered_count: 33,
        success_count: 30,
        error_count: 3,
        total_tokens: 123456,
    };

    let value = serde_json::to_value(result).expect("serialize request log filter summary");
    let obj = value
        .as_object()
        .expect("request log filter summary object");
    for key in [
        "totalCount",
        "filteredCount",
        "successCount",
        "errorCount",
        "totalTokens",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}
