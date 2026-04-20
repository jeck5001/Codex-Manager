use super::{
    AccountListParams, AccountListResult, AccountSummary, AuditLogItem, CostExportResult,
    CostSummaryParams, DashboardAccountStatusBucket, DashboardGatewayMetricsResult,
    DashboardHealthResult, DashboardTrendPoint, DashboardTrendResult, GovernanceSummaryItem,
    HeatmapTrendResult, ModelOption, ModelPricingItem, ModelTrendResult, OperationAuditItem,
    RequestLogFilterSummaryResult, RequestLogListParams, RequestLogListResult,
    RequestLogSummary, RequestLogTodaySummaryResult, StartupSnapshotResult, RequestTrendResult,
    TrendQueryParams, UsageAggregateSummaryResult, UsagePredictionSummaryResult,
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
        new_account_protection_until: Some(15),
        new_account_protection_reason: Some("新号保护期内，已自动降优先级".to_string()),
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
        "newAccountProtectionUntil",
        "newAccountProtectionReason",
        "subscriptionPlanType",
        "subscriptionUpdatedAt",
        "teamManagerUploadedAt",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }

    for key in ["workspaceId", "workspaceName", "note", "updatedAt"] {
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
            new_account_protection_until: Some(15),
            new_account_protection_reason: Some("新号保护期内，已自动降优先级".to_string()),
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
fn audit_log_item_serialization_uses_camel_case() {
    let result = AuditLogItem {
        id: 1,
        action: "update".to_string(),
        object_type: "account".to_string(),
        object_id: Some("acc-1".to_string()),
        operator: "desktop-app".to_string(),
        changes: serde_json::json!({ "before": { "status": "disabled" }, "after": { "status": "active" } }),
        created_at: 2,
    };

    let value = serde_json::to_value(result).expect("serialize audit log item");
    let obj = value.as_object().expect("audit log object");
    for key in [
        "id",
        "action",
        "objectType",
        "objectId",
        "operator",
        "changes",
        "createdAt",
    ] {
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
        candidate_count: Some(12),
        attempted_count: Some(2),
        skipped_count: Some(10),
        skipped_cooldown_count: Some(9),
        skipped_inflight_count: Some(1),
        route_strategy: Some("weighted".to_string()),
        requested_model: Some("o3".to_string()),
        model_fallback_path: vec!["o3".to_string(), "o4-mini".to_string()],
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
        "candidateCount",
        "attemptedCount",
        "skippedCount",
        "skippedCooldownCount",
        "skippedInflightCount",
        "originalPath",
        "adaptedPath",
        "routeStrategy",
        "responseAdapter",
        "requestPath",
        "upstreamUrl",
        "durationMs",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn model_pricing_item_serialization_uses_camel_case() {
    let item = ModelPricingItem {
        model_slug: "o3".to_string(),
        input_price_per_1k: 0.02,
        output_price_per_1k: 0.08,
        updated_at: Some(1),
    };

    let value = serde_json::to_value(item).expect("serialize model pricing item");
    let obj = value.as_object().expect("model pricing object");
    for key in [
        "modelSlug",
        "inputPricePer1k",
        "outputPricePer1k",
        "updatedAt",
    ] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn cost_summary_params_serialization_uses_camel_case() {
    let params = CostSummaryParams {
        preset: Some("custom".to_string()),
        start_ts: Some(1),
        end_ts: Some(2),
    };

    let value = serde_json::to_value(params).expect("serialize cost summary params");
    let obj = value.as_object().expect("cost summary params object");
    for key in ["preset", "startTs", "endTs"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn cost_export_result_serialization_uses_camel_case() {
    let result = CostExportResult {
        file_name: "costs.csv".to_string(),
        content: "a,b\n1,2\n".to_string(),
    };

    let value = serde_json::to_value(result).expect("serialize cost export result");
    let obj = value.as_object().expect("cost export result object");
    for key in ["fileName", "content"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn trend_query_params_serialization_uses_camel_case() {
    let params = TrendQueryParams {
        preset: Some("30d".to_string()),
        start_ts: Some(1),
        end_ts: Some(2),
        granularity: Some("week".to_string()),
    };

    let value = serde_json::to_value(params).expect("serialize trend query params");
    let obj = value.as_object().expect("trend query params object");
    for key in ["preset", "startTs", "endTs", "granularity"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn trend_results_serialization_uses_camel_case() {
    for value in [
        serde_json::to_value(RequestTrendResult::default()).expect("serialize request trend"),
        serde_json::to_value(ModelTrendResult::default()).expect("serialize model trend"),
        serde_json::to_value(HeatmapTrendResult::default()).expect("serialize heatmap trend"),
    ] {
        let obj = value.as_object().expect("trend result object");
        assert!(obj.contains_key("preset"));
        assert!(obj.contains_key("rangeStart"));
        assert!(obj.contains_key("rangeEnd"));
        assert!(obj.contains_key("items"));
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
            candidate_count: Some(12),
            attempted_count: Some(2),
            skipped_count: Some(10),
            skipped_cooldown_count: Some(9),
            skipped_inflight_count: Some(1),
            route_strategy: Some("least-latency".to_string()),
            requested_model: Some("o3".to_string()),
            model_fallback_path: vec!["o3".to_string(), "o4-mini".to_string()],
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
fn dashboard_health_result_serialization_uses_camel_case() {
    let result = DashboardHealthResult {
        generated_at: 123,
        account_status_buckets: vec![DashboardAccountStatusBucket {
            key: "online".to_string(),
            label: "在线".to_string(),
            count: 6,
            percent: 60,
        }],
        gateway_metrics: DashboardGatewayMetricsResult {
            window_minutes: 5,
            total_requests: 20,
            success_requests: 18,
            error_requests: 2,
            qps: 0.07,
            success_rate: 90.0,
            p50_latency_ms: Some(120),
            p95_latency_ms: Some(280),
            p99_latency_ms: Some(500),
        },
        recent_healthcheck: None,
    };

    let value = serde_json::to_value(result).expect("serialize dashboard health");
    let obj = value.as_object().expect("dashboard health object");
    for key in ["generatedAt", "accountStatusBuckets", "gatewayMetrics"] {
        assert!(obj.contains_key(key), "missing key: {key}");
    }
}

#[test]
fn dashboard_trend_result_serialization_uses_camel_case() {
    let result = DashboardTrendResult {
        generated_at: 456,
        bucket_minutes: 1,
        points: vec![DashboardTrendPoint {
            bucket_ts: 450,
            request_count: 12,
            error_count: 3,
            error_rate: 25.0,
        }],
    };

    let value = serde_json::to_value(result).expect("serialize dashboard trend");
    let obj = value.as_object().expect("dashboard trend object");
    for key in ["generatedAt", "bucketMinutes", "points"] {
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

#[test]
fn startup_snapshot_result_serialization_uses_lightweight_request_log_fields() {
    let result = StartupSnapshotResult {
        accounts: vec![],
        usage_aggregate_summary: UsageAggregateSummaryResult::default(),
        usage_prediction_summary: UsagePredictionSummaryResult::default(),
        failure_reason_summary: vec![],
        governance_summary: vec![],
        operation_audits: vec![],
        api_keys: vec![],
        api_model_options: vec![ModelOption {
            slug: "gpt-5".to_string(),
            display_name: "GPT-5".to_string(),
        }],
        manual_preferred_account_id: Some("acc_manual".to_string()),
        request_log_today_summary: RequestLogTodaySummaryResult {
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            reasoning_output_tokens: 0,
            today_tokens: 0,
            estimated_cost: 0.0,
        },
        recent_request_log_count: 12,
        latest_request_account_id: Some("acc_latest".to_string()),
    };

    let value = serde_json::to_value(result).expect("serialize startup snapshot");
    let obj = value.as_object().expect("startup snapshot object");

    assert_eq!(
        obj.get("recentRequestLogCount").and_then(|value| value.as_i64()),
        Some(12)
    );
    assert_eq!(
        obj.get("latestRequestAccountId")
            .and_then(|value| value.as_str()),
        Some("acc_latest")
    );
    assert!(!obj.contains_key("requestLogs"));
    assert!(!obj.contains_key("usageSnapshots"));
}
