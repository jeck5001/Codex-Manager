use codexmanager_core::rpc::types::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;

pub(super) fn try_handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let result = match req.method.as_str() {
        "gateway/routeStrategy/get" => {
            let strategy = crate::gateway::current_route_strategy();
            super::as_json(serde_json::json!({
                "strategy": strategy,
                "options": ["ordered", "balanced"],
                "manualPreferredAccountId": crate::gateway::manual_preferred_account(),
            }))
        }
        "gateway/routeStrategy/set" => {
            let strategy = super::str_param(req, "strategy").unwrap_or("");
            super::value_or_error(crate::set_gateway_route_strategy(strategy).map(|applied| {
                serde_json::json!({
                    "strategy": applied
                })
            }))
        }
        "gateway/manualAccount/get" => super::as_json(serde_json::json!({
            "accountId": crate::gateway::manual_preferred_account()
        })),
        "gateway/manualAccount/set" => {
            let account_id = super::str_param(req, "accountId").unwrap_or("");
            super::ok_or_error(crate::gateway::set_manual_preferred_account(account_id))
        }
        "gateway/manualAccount/clear" => {
            crate::gateway::clear_manual_preferred_account();
            super::ok_result()
        }
        "gateway/headerPolicy/get" => super::as_json(serde_json::json!({
            "cpaNoCookieHeaderModeEnabled": crate::gateway::cpa_no_cookie_header_mode_enabled(),
            "envKey": "CODEXMANAGER_CPA_NO_COOKIE_HEADER_MODE",
        })),
        "gateway/headerPolicy/set" => {
            let enabled = super::bool_param(req, "cpaNoCookieHeaderModeEnabled")
                .or_else(|| super::bool_param(req, "enabled"))
                .unwrap_or(false);
            super::value_or_error(crate::set_gateway_cpa_no_cookie_header_mode(enabled).map(
                |applied| {
                    serde_json::json!({
                        "cpaNoCookieHeaderModeEnabled": applied,
                    })
                },
            ))
        }
        "gateway/backgroundTasks/get" => {
            super::as_json(crate::usage_refresh::background_tasks_settings())
        }
        "gateway/upstreamProxy/get" => super::as_json(serde_json::json!({
            "proxyUrl": crate::gateway::current_upstream_proxy_url(),
            "envKey": "CODEXMANAGER_UPSTREAM_PROXY_URL",
            "requiresRestart": false,
        })),
        "gateway/upstreamProxy/set" => {
            let requested = req
                .params
                .as_ref()
                .and_then(|params| params.get("proxyUrl"))
                .and_then(|value| match value {
                    Value::Null => Some(None),
                    Value::String(text) => Some(Some(text.as_str())),
                    _ => None,
                })
                .or_else(|| super::str_param(req, "url").map(|value| Some(value)));
            let proxy_url = requested.unwrap_or(None);
            super::value_or_error(
                crate::set_gateway_upstream_proxy_url(proxy_url).map(|applied| {
                    serde_json::json!({
                        "proxyUrl": applied,
                        "envKey": "CODEXMANAGER_UPSTREAM_PROXY_URL",
                        "requiresRestart": false,
                    })
                }),
            )
        }
        "gateway/freeProxy/sync" => {
            let result = (|| {
                let input = req
                    .params
                    .as_ref()
                    .map(|params| {
                        serde_json::from_value::<crate::gateway::FreeProxySyncInput>(params.clone())
                    })
                    .transpose()
                    .map_err(|err| format!("invalid freeproxy sync payload: {err}"))?
                    .unwrap_or_default();
                crate::gateway::sync_proxy_pool_from_freeproxy(input)
            })();
            super::value_or_error(result)
        }
        "gateway/transport/get" => super::as_json(serde_json::json!({
            "sseKeepaliveIntervalMs": crate::current_gateway_sse_keepalive_interval_ms(),
            "upstreamStreamTimeoutMs": crate::current_gateway_upstream_stream_timeout_ms(),
            "envKeys": [
                "CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS",
                "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS"
            ],
            "requiresRestart": false,
        })),
        "gateway/transport/set" => {
            let requested_sse_keepalive_interval_ms = u64_param(req, "sseKeepaliveIntervalMs");
            let requested_upstream_stream_timeout_ms = u64_param(req, "upstreamStreamTimeoutMs");
            super::value_or_error((|| {
                let sse_keepalive_interval_ms =
                    if let Some(value) = requested_sse_keepalive_interval_ms {
                        crate::set_gateway_sse_keepalive_interval_ms(value)?
                    } else {
                        crate::current_gateway_sse_keepalive_interval_ms()
                    };
                let upstream_stream_timeout_ms =
                    if let Some(value) = requested_upstream_stream_timeout_ms {
                        crate::set_gateway_upstream_stream_timeout_ms(value)?
                    } else {
                        crate::current_gateway_upstream_stream_timeout_ms()
                    };
                Ok(serde_json::json!({
                    "sseKeepaliveIntervalMs": sse_keepalive_interval_ms,
                    "upstreamStreamTimeoutMs": upstream_stream_timeout_ms,
                    "requiresRestart": false,
                }))
            })())
        }
        "gateway/backgroundTasks/set" => {
            let patch = crate::usage_refresh::BackgroundTasksSettingsPatch {
                usage_polling_enabled: super::bool_param(req, "usagePollingEnabled")
                    .or_else(|| super::bool_param(req, "usagePolling")),
                usage_poll_interval_secs: u64_param(req, "usagePollIntervalSecs"),
                gateway_keepalive_enabled: super::bool_param(req, "gatewayKeepaliveEnabled")
                    .or_else(|| super::bool_param(req, "gatewayKeepalive")),
                gateway_keepalive_interval_secs: u64_param(req, "gatewayKeepaliveIntervalSecs"),
                token_refresh_polling_enabled: super::bool_param(req, "tokenRefreshPollingEnabled")
                    .or_else(|| super::bool_param(req, "tokenRefreshPolling")),
                token_refresh_poll_interval_secs: u64_param(req, "tokenRefreshPollIntervalSecs"),
                session_probe_polling_enabled: super::bool_param(req, "sessionProbePollingEnabled")
                    .or_else(|| super::bool_param(req, "sessionProbePolling")),
                session_probe_interval_secs: u64_param(req, "sessionProbeIntervalSecs"),
                session_probe_sample_size: usize_param(req, "sessionProbeSampleSize"),
                usage_refresh_workers: usize_param(req, "usageRefreshWorkers"),
                http_worker_factor: usize_param(req, "httpWorkerFactor"),
                http_worker_min: usize_param(req, "httpWorkerMin"),
                http_stream_worker_factor: usize_param(req, "httpStreamWorkerFactor"),
                http_stream_worker_min: usize_param(req, "httpStreamWorkerMin"),
                auto_register_pool_enabled: super::bool_param(req, "autoRegisterPoolEnabled"),
                auto_register_ready_account_count: usize_param(
                    req,
                    "autoRegisterReadyAccountCount",
                ),
                auto_register_ready_remain_percent: u64_param(
                    req,
                    "autoRegisterReadyRemainPercent",
                ),
                auto_disable_risky_accounts_enabled: super::bool_param(
                    req,
                    "autoDisableRiskyAccountsEnabled",
                ),
                auto_disable_risky_accounts_failure_threshold: usize_param(
                    req,
                    "autoDisableRiskyAccountsFailureThreshold",
                ),
                auto_disable_risky_accounts_health_score_threshold: usize_param(
                    req,
                    "autoDisableRiskyAccountsHealthScoreThreshold",
                ),
                auto_disable_risky_accounts_lookback_mins: u64_param(
                    req,
                    "autoDisableRiskyAccountsLookbackMins",
                ),
                account_cooldown_auth_secs: u64_param(req, "accountCooldownAuthSecs"),
                account_cooldown_rate_limited_secs: u64_param(
                    req,
                    "accountCooldownRateLimitedSecs",
                ),
                account_cooldown_server_error_secs: u64_param(
                    req,
                    "accountCooldownServerErrorSecs",
                ),
                account_cooldown_network_secs: u64_param(req, "accountCooldownNetworkSecs"),
                account_cooldown_low_quota_secs: u64_param(req, "accountCooldownLowQuotaSecs"),
                account_cooldown_deactivated_secs: u64_param(
                    req,
                    "accountCooldownDeactivatedSecs",
                ),
            };
            let input = crate::BackgroundTasksInput {
                usage_polling_enabled: patch.usage_polling_enabled,
                usage_poll_interval_secs: patch.usage_poll_interval_secs,
                gateway_keepalive_enabled: patch.gateway_keepalive_enabled,
                gateway_keepalive_interval_secs: patch.gateway_keepalive_interval_secs,
                token_refresh_polling_enabled: patch.token_refresh_polling_enabled,
                token_refresh_poll_interval_secs: patch.token_refresh_poll_interval_secs,
                session_probe_polling_enabled: patch.session_probe_polling_enabled,
                session_probe_interval_secs: patch.session_probe_interval_secs,
                session_probe_sample_size: patch.session_probe_sample_size,
                usage_refresh_workers: patch.usage_refresh_workers,
                http_worker_factor: patch.http_worker_factor,
                http_worker_min: patch.http_worker_min,
                http_stream_worker_factor: patch.http_stream_worker_factor,
                http_stream_worker_min: patch.http_stream_worker_min,
                auto_register_pool_enabled: patch.auto_register_pool_enabled,
                auto_register_ready_account_count: patch.auto_register_ready_account_count,
                auto_register_ready_remain_percent: patch.auto_register_ready_remain_percent,
                auto_disable_risky_accounts_enabled: patch.auto_disable_risky_accounts_enabled,
                auto_disable_risky_accounts_failure_threshold: patch
                    .auto_disable_risky_accounts_failure_threshold,
                auto_disable_risky_accounts_health_score_threshold: patch
                    .auto_disable_risky_accounts_health_score_threshold,
                auto_disable_risky_accounts_lookback_mins: patch
                    .auto_disable_risky_accounts_lookback_mins,
                account_cooldown_auth_secs: patch.account_cooldown_auth_secs,
                account_cooldown_rate_limited_secs: patch.account_cooldown_rate_limited_secs,
                account_cooldown_server_error_secs: patch.account_cooldown_server_error_secs,
                account_cooldown_network_secs: patch.account_cooldown_network_secs,
                account_cooldown_low_quota_secs: patch.account_cooldown_low_quota_secs,
                account_cooldown_deactivated_secs: patch.account_cooldown_deactivated_secs,
            };
            super::value_or_error(crate::set_gateway_background_tasks(input))
        }
        _ => return None,
    };

    Some(super::response(req, result))
}

fn u64_param(req: &JsonRpcRequest, key: &str) -> Option<u64> {
    let value = req.params.as_ref()?.get(key)?;
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
}

fn usize_param(req: &JsonRpcRequest, key: &str) -> Option<usize> {
    u64_param(req, key).and_then(|value| usize::try_from(value).ok())
}
