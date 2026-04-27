# Gateway Route Account Whitelist Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a multi-account routing whitelist so only selected accounts participate in gateway routing, with a clear action that restores default all-account routing.

**Architecture:** Replace the in-memory single-account `manual preferred account` routing hint with a route-account whitelist that filters the candidate pool before existing routing strategies run. Update the service RPC surface, startup snapshot, and frontend account-management UI to read and write the whitelist consistently. Keep the actual route ordering logic (`ordered`, `balanced`, `weighted`, `least-latency`, `cost-first`) unchanged inside the filtered pool.

**Tech Stack:** Rust service routing/RPC types, Tauri command bridge, Next.js TypeScript hooks/pages, `node:test`, Rust unit tests, `pnpm run build:desktop`.

---

### Task 1: Replace backend single-account routing with a route whitelist

**Files:**
- Modify: `crates/service/src/gateway/routing/route_hint.rs`
- Modify: `crates/service/src/gateway/routing/tests/route_hint_tests.rs`
- Modify: `crates/service/src/gateway/upstream/support/candidates.rs`
- Modify: `crates/service/src/gateway/upstream/proxy_pipeline/request_setup.rs`
- Modify: `crates/service/src/gateway/upstream/proxy.rs`
- Modify: `crates/service/src/gateway/mod.rs`
- Modify: `crates/service/src/auth/auth_account.rs`

- [ ] **Step 1: Write the failing route-whitelist tests**

Add these tests to `crates/service/src/gateway/routing/tests/route_hint_tests.rs`:

```rust
#[test]
fn route_whitelist_filters_candidates_before_ordered_strategy() {
    let _guard = route_strategy_test_guard();
    clear_route_state_for_tests();
    set_manual_route_account_ids(&["acc-b".to_string(), "acc-c".to_string()])
        .expect("set route whitelist");

    let mut candidates = candidate_list();
    retain_manual_route_account_ids(&mut candidates);
    apply_route_strategy(&mut candidates, "gk-route-whitelist", Some("gpt-5.3-codex"));

    assert_eq!(
        account_ids(&candidates),
        vec!["acc-b".to_string(), "acc-c".to_string()]
    );
    assert_eq!(
        get_manual_route_account_ids(),
        vec!["acc-b".to_string(), "acc-c".to_string()]
    );
}

#[test]
fn route_whitelist_clear_restores_full_candidate_pool() {
    let _guard = route_strategy_test_guard();
    clear_route_state_for_tests();
    set_manual_route_account_ids(&["acc-b".to_string()]).expect("set route whitelist");
    clear_manual_route_account_ids();

    let mut candidates = candidate_list();
    retain_manual_route_account_ids(&mut candidates);
    apply_route_strategy(&mut candidates, "gk-route-whitelist-clear", Some("gpt-5.3-codex"));

    assert_eq!(
        account_ids(&candidates),
        vec![
            "acc-a".to_string(),
            "acc-b".to_string(),
            "acc-c".to_string()
        ]
    );
    assert!(get_manual_route_account_ids().is_empty());
}

#[test]
fn route_whitelist_dedupes_ids_and_ignores_missing_accounts() {
    let _guard = route_strategy_test_guard();
    clear_route_state_for_tests();
    set_manual_route_account_ids(&[
        "acc-c".to_string(),
        "acc-c".to_string(),
        "acc-missing".to_string(),
    ])
    .expect("set route whitelist");

    let mut candidates = candidate_list();
    retain_manual_route_account_ids(&mut candidates);
    apply_route_strategy(&mut candidates, "gk-route-whitelist-dedupe", Some("gpt-5.3-codex"));

    assert_eq!(account_ids(&candidates), vec!["acc-c".to_string()]);
    assert_eq!(get_manual_route_account_ids(), vec!["acc-c".to_string()]);
}
```

- [ ] **Step 2: Run the route test to verify it fails**

Run:

```bash
cargo test -p codexmanager-service route_whitelist_filters_candidates_before_ordered_strategy -- --nocapture
```

Expected:

- compile or test failure because `set_manual_route_account_ids`, `get_manual_route_account_ids`, and `clear_manual_route_account_ids` do not exist yet

- [ ] **Step 3: Implement the new route-whitelist state and candidate filtering**

Update `crates/service/src/gateway/routing/route_hint.rs`:

```rust
#[derive(Default)]
struct RouteRoundRobinState {
    next_start_by_key_model: HashMap<String, RouteStateEntry<usize>>,
    p2c_nonce_by_key_model: HashMap<String, RouteStateEntry<u64>>,
    manual_route_account_ids: Option<Vec<String>>,
    maintenance_tick: u64,
}

pub(crate) fn get_manual_route_account_ids() -> Vec<String> {
    ensure_route_config_loaded();
    let lock = ROUTE_STATE.get_or_init(|| Mutex::new(RouteRoundRobinState::default()));
    let state = crate::lock_utils::lock_recover(lock, "route_state");
    state.manual_route_account_ids.clone().unwrap_or_default()
}

pub(crate) fn set_manual_route_account_ids(account_ids: &[String]) -> Result<Vec<String>, String> {
    ensure_route_config_loaded();
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for account_id in account_ids {
        let id = account_id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        normalized.push(id.to_string());
    }
    let lock = ROUTE_STATE.get_or_init(|| Mutex::new(RouteRoundRobinState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_state");
    state.manual_route_account_ids = if normalized.is_empty() {
        None
    } else {
        Some(normalized.clone())
    };
    Ok(normalized)
}

pub(crate) fn clear_manual_route_account_ids() {
    ensure_route_config_loaded();
    let lock = ROUTE_STATE.get_or_init(|| Mutex::new(RouteRoundRobinState::default()));
    let mut state = crate::lock_utils::lock_recover(lock, "route_state");
    state.manual_route_account_ids = None;
}

pub(crate) fn retain_manual_route_account_ids(candidates: &mut Vec<(Account, Token)>) {
    let route_account_ids = get_manual_route_account_ids();
    if route_account_ids.is_empty() {
        return;
    }
    let allowed = route_account_ids
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    candidates.retain(|(account, _)| allowed.contains(account.id.as_str()));
}
```

Then remove the single-account head rotation:

```rust
-    if rotate_to_manual_preferred_account(candidates) {
-        return;
-    }
```

and delete `rotate_to_manual_preferred_account`.

Update the test-only reset helper:

```rust
#[cfg(test)]
fn clear_route_state_for_tests() {
    super::route_quality::clear_route_quality_for_tests();
    super::route_latency::clear_route_latency_for_tests();
    if let Some(lock) = ROUTE_STATE.get() {
        let mut state = crate::lock_utils::lock_recover(lock, "route_state");
        state.next_start_by_key_model.clear();
        state.p2c_nonce_by_key_model.clear();
        state.manual_route_account_ids = None;
        state.maintenance_tick = 0;
    }
}
```

Update `crates/service/src/gateway/upstream/support/candidates.rs` to remove the manual-head bypass:

```rust
pub(in super::super) fn candidate_skip_reason_for_proxy(
    account_id: &str,
    idx: usize,
    candidate_count: usize,
    account_max_inflight: usize,
) -> Option<CandidateSkipReason> {
    let has_more_candidates = idx + 1 < candidate_count;
    if super::super::super::is_account_in_cooldown(account_id) && has_more_candidates {
        super::super::super::record_gateway_failover_attempt();
        return Some(CandidateSkipReason::Cooldown);
    }
    if account_max_inflight > 0
        && super::super::super::account_inflight_count(account_id) >= account_max_inflight
        && has_more_candidates
    {
        super::super::super::record_gateway_failover_attempt();
        return Some(CandidateSkipReason::Inflight);
    }
    None
}
```

Update `crates/service/src/gateway/upstream/proxy_pipeline/request_setup.rs` to filter the candidate `Vec` before computing `candidate_count`:

```rust
pub(in super::super) fn prepare_request_setup(
    input: PrepareRequestSetupInput<'_>,
    candidates: &mut Vec<(Account, Token)>,
) -> UpstreamRequestSetup {
    let upstream_base = super::super::super::resolve_upstream_base_url();
    let upstream_fallback_base =
        super::super::super::resolve_upstream_fallback_base_url(upstream_base.as_str());
    let (url, url_alt) = super::super::super::request_rewrite::compute_upstream_url(
        upstream_base.as_str(),
        input.path,
    );
    let upstream_cookie = super::super::super::upstream_cookie();

    super::super::super::retain_manual_route_account_ids(candidates);
    let candidate_count = candidates.len();
    let account_max_inflight = super::super::super::account_max_inflight_limit();
    let anthropic_has_prompt_cache_key =
        input.protocol_type == PROTOCOL_ANTHROPIC_NATIVE && input.has_prompt_cache_key;
    super::super::super::apply_route_strategy(
        candidates.as_mut_slice(),
        input.key_id,
        input.model_for_log,
    );
    let candidate_order = candidates
        .iter()
        .map(|(account, _)| format!("{}#sort={}", account.id, account.sort))
        .collect::<Vec<_>>();
    super::super::super::trace_log::log_candidate_pool(
        input.trace_id,
        input.key_id,
        super::super::super::current_route_strategy(),
        candidate_order.as_slice(),
    );
```

Update the call site in `crates/service/src/gateway/upstream/proxy.rs`:

```rust
        let setup = prepare_request_setup(
            PrepareRequestSetupInput {
                path: path.as_str(),
                protocol_type: protocol_type.as_str(),
                has_prompt_cache_key,
                incoming_headers: &incoming_headers,
                body: body.as_ref(),
                key_id: key_id.as_str(),
                model_for_log: current_model_for_log,
                trace_id: trace_id.as_str(),
            },
-            candidates.as_mut_slice(),
+            &mut candidates,
        );
```

Update `crates/service/src/gateway/mod.rs` to validate whitelist members against the routable candidate pool:

```rust
pub(crate) fn manual_route_account_ids() -> Vec<String> {
    route_hint::get_manual_route_account_ids()
}

pub(crate) fn set_manual_route_account_ids(account_ids: &[String]) -> Result<Vec<String>, String> {
    let storage = open_storage().ok_or_else(|| "storage not initialized".to_string())?;
    let candidates = collect_gateway_candidates(&storage)?;
    let available = candidates
        .iter()
        .map(|(account, _)| account.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let requested = account_ids
        .iter()
        .map(|account_id| account_id.trim())
        .filter(|account_id| !account_id.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if requested.iter().any(|account_id| !available.contains(account_id.as_str())) {
        return Err("one or more accounts are not available for routing".to_string());
    }
    route_hint::set_manual_route_account_ids(&requested)
}

pub(crate) fn clear_manual_route_account_ids() {
    route_hint::clear_manual_route_account_ids();
}
```

Update `crates/service/src/auth/auth_account.rs` to stop mutating route state during auth login/logout:

```rust
-    let _ = crate::gateway::set_manual_preferred_account(&account_id);
```

```rust
-        let _ = clear_manual_preferred_account_if(account_id);
```

```rust
-            let _ = clear_manual_preferred_account_if(&account_id);
```

- [ ] **Step 4: Run the route tests to verify the backend behavior passes**

Run:

```bash
cargo test -p codexmanager-service route_whitelist_filters_candidates_before_ordered_strategy -- --nocapture
cargo test -p codexmanager-service route_whitelist_clear_restores_full_candidate_pool -- --nocapture
cargo test -p codexmanager-service route_whitelist_dedupes_ids_and_ignores_missing_accounts -- --nocapture
```

Expected:

- all three tests PASS

- [ ] **Step 5: Commit the backend route-whitelist core**

```bash
git add \
  crates/service/src/gateway/routing/route_hint.rs \
  crates/service/src/gateway/routing/tests/route_hint_tests.rs \
  crates/service/src/gateway/upstream/support/candidates.rs \
  crates/service/src/gateway/upstream/proxy_pipeline/request_setup.rs \
  crates/service/src/gateway/upstream/proxy.rs \
  crates/service/src/gateway/mod.rs \
  crates/service/src/auth/auth_account.rs
git commit -m "feat: add gateway route account whitelist core"
```

### Task 2: Replace the RPC and startup-snapshot contract with route-account IDs

**Files:**
- Modify: `crates/core/src/rpc/types.rs`
- Modify: `crates/core/src/rpc/tests/types_tests.rs`
- Modify: `crates/service/src/startup_snapshot.rs`
- Modify: `crates/service/src/rpc_dispatch/gateway.rs`
- Modify: `apps/src-tauri/src/commands/settings/gateway.rs`
- Modify: `apps/src-tauri/src/commands/registry.rs`
- Modify: `apps/src/lib/api/transport.ts`
- Modify: `apps/src/lib/api/service-client.ts`
- Modify: `apps/src/lib/api/normalize.ts`
- Modify: `apps/src/lib/api/startup-snapshot-state.ts`
- Modify: `apps/src/lib/api/startup-snapshot-state.test.ts`
- Modify: `apps/src/components/layout/app-bootstrap.tsx`
- Modify: `apps/src/hooks/useDashboardStats.ts`
- Modify: `apps/src/types/index.ts`

- [ ] **Step 1: Write the failing contract tests**

Update `crates/core/src/rpc/tests/types_tests.rs` with a startup-snapshot serialization test:

```rust
#[test]
fn startup_snapshot_result_serialization_uses_route_account_ids() {
    let result = StartupSnapshotResult {
        accounts: vec![],
        usage_aggregate_summary: UsageAggregateSummaryResult::default(),
        usage_prediction_summary: UsagePredictionSummaryResult::default(),
        failure_reason_summary: vec![],
        governance_summary: vec![],
        operation_audits: vec![],
        api_keys: vec![],
        api_model_options: vec![],
        manual_route_account_ids: vec!["acc-a".to_string(), "acc-b".to_string()],
        request_log_today_summary: RequestLogTodaySummaryResult {
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            reasoning_output_tokens: 0,
            today_tokens: 0,
            estimated_cost: 0.0,
        },
        recent_request_log_count: 0,
        latest_request_account_id: None,
    };

    let value = serde_json::to_value(result).expect("serialize startup snapshot");
    let obj = value.as_object().expect("startup snapshot object");
    assert_eq!(
        obj.get("manualRouteAccountIds")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert!(!obj.contains_key("manualPreferredAccountId"));
}
```

Update `apps/src/lib/api/startup-snapshot-state.test.ts`:

```ts
test("pickCurrentAccountId keeps fallback selection inside route whitelist", () => {
  const accountId = pickCurrentAccountId(
    [
      { id: "acc-a", availabilityLevel: "ok" },
      { id: "acc-b", availabilityLevel: "ok" },
      { id: "acc-c", availabilityLevel: "warn" },
    ],
    "acc-a",
    ["acc-b"],
  );

  assert.equal(accountId, "acc-b");
});

test("pickCurrentAccountId still uses latest request account when whitelist is empty", () => {
  const accountId = pickCurrentAccountId(
    [
      { id: "acc-a", availabilityLevel: "ok" },
      { id: "acc-b", availabilityLevel: "ok" },
    ],
    "acc-b",
    [],
  );

  assert.equal(accountId, "acc-b");
});
```

- [ ] **Step 2: Run the contract tests to verify they fail**

Run:

```bash
cargo test -p codexmanager-core startup_snapshot_result_serialization_uses_route_account_ids -- --nocapture
cd apps && node --test src/lib/api/startup-snapshot-state.test.ts
```

Expected:

- Rust test fails because `StartupSnapshotResult` still exposes `manual_preferred_account_id`
- frontend test fails because `pickCurrentAccountId` still accepts a preferred single-account input

- [ ] **Step 3: Implement the new RPC, Tauri, and startup-snapshot contract**

Update `crates/core/src/rpc/types.rs`:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSnapshotResult {
    pub accounts: Vec<StartupAccountSummary>,
    #[serde(default)]
    pub usage_aggregate_summary: UsageAggregateSummaryResult,
    #[serde(default)]
    pub usage_prediction_summary: UsagePredictionSummaryResult,
    #[serde(default)]
    pub failure_reason_summary: Vec<FailureReasonSummaryItem>,
    #[serde(default)]
    pub governance_summary: Vec<GovernanceSummaryItem>,
    #[serde(default)]
    pub operation_audits: Vec<OperationAuditItem>,
    pub api_keys: Vec<ApiKeySummary>,
    pub api_model_options: Vec<ModelOption>,
    #[serde(default)]
    pub manual_route_account_ids: Vec<String>,
    pub request_log_today_summary: RequestLogTodaySummaryResult,
    #[serde(default)]
    pub recent_request_log_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_request_account_id: Option<String>,
}
```

Update `crates/service/src/startup_snapshot.rs`:

```rust
let manual_route_account_ids = gateway::manual_route_account_ids();

Ok(StartupSnapshotResult {
    accounts,
    usage_aggregate_summary,
    usage_prediction_summary,
    failure_reason_summary,
    governance_summary,
    operation_audits,
    api_keys,
    api_model_options,
    manual_route_account_ids,
    request_log_today_summary,
    recent_request_log_count,
    latest_request_account_id,
})
```

Update `crates/service/src/rpc_dispatch/gateway.rs`:

```rust
"gateway/routeStrategy/get" => {
    let strategy = crate::gateway::current_route_strategy();
    super::as_json(serde_json::json!({
        "strategy": strategy,
        "options": ["ordered", "balanced", "weighted", "least-latency", "cost-first"],
        "routeAccountIds": crate::gateway::manual_route_account_ids(),
    }))
}
"gateway/routeAccounts/get" => super::as_json(serde_json::json!({
    "accountIds": crate::gateway::manual_route_account_ids()
})),
"gateway/routeAccounts/set" => {
    let account_ids = string_array_param(req, "accountIds").unwrap_or_default();
    super::value_or_error(crate::gateway::set_manual_route_account_ids(&account_ids).map(
        |applied| serde_json::json!({ "accountIds": applied }),
    ))
}
"gateway/routeAccounts/clear" => {
    crate::gateway::clear_manual_route_account_ids();
    super::ok_result()
}
```

Add the helper at the bottom of the same file:

```rust
fn string_array_param(req: &JsonRpcRequest, key: &str) -> Option<Vec<String>> {
    let items = req.params.as_ref()?.get(key)?.as_array()?;
    items
        .iter()
        .map(|item| item.as_str().map(str::trim).map(ToString::to_string))
        .collect::<Option<Vec<_>>>()
}
```

Update `apps/src-tauri/src/commands/settings/gateway.rs`:

```rust
#[tauri::command]
pub async fn service_gateway_route_accounts_get(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/routeAccounts/get", addr, None).await
}

#[tauri::command]
pub async fn service_gateway_route_accounts_set(
    addr: Option<String>,
    account_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "accountIds": account_ids });
    rpc_call_in_background("gateway/routeAccounts/set", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_gateway_route_accounts_clear(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("gateway/routeAccounts/clear", addr, None).await
}
```

Register the new commands in `apps/src-tauri/src/commands/registry.rs`:

```rust
crate::commands::settings::gateway::service_gateway_route_accounts_get,
crate::commands::settings::gateway::service_gateway_route_accounts_set,
crate::commands::settings::gateway::service_gateway_route_accounts_clear,
```

Update `apps/src/lib/api/transport.ts`:

```ts
service_gateway_route_accounts_get: { rpcMethod: "gateway/routeAccounts/get" },
service_gateway_route_accounts_set: { rpcMethod: "gateway/routeAccounts/set" },
service_gateway_route_accounts_clear: { rpcMethod: "gateway/routeAccounts/clear" },
```

Update `apps/src/lib/api/service-client.ts`:

```ts
function readStringArrayField(payload: unknown, key: string): string[] {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return [];
  }
  const value = (payload as Record<string, unknown>)[key];
  return Array.isArray(value)
    ? value
        .map((item) => (typeof item === "string" ? item.trim() : ""))
        .filter(Boolean)
    : [];
}

async getRouteAccountIds(): Promise<string[]> {
  const result = await invoke<unknown>("service_gateway_route_accounts_get", withAddr());
  return readStringArrayField(result, "accountIds");
},
setRouteAccounts: (accountIds: string[]) =>
  invoke("service_gateway_route_accounts_set", withAddr({ accountIds })),
clearRouteAccounts: () =>
  invoke("service_gateway_route_accounts_clear", withAddr()),
async getRouteStrategy(): Promise<GatewayRouteStrategyInfo> {
  const result = await invoke<unknown>("service_gateway_route_strategy_get", withAddr());
  return normalizeGatewayRouteStrategy(result);
},
```

Update `apps/src/types/index.ts`:

```ts
export interface GatewayRouteStrategyInfo {
  strategy: string;
  options: string[];
  routeAccountIds: string[];
}

export interface StartupSnapshot {
  accounts: Account[];
  usageAggregateSummary: UsageAggregateSummary;
  usagePredictionSummary: UsagePredictionSummary;
  failureReasonSummary: FailureReasonSummaryItem[];
  governanceSummary: GovernanceSummaryItem[];
  operationAudits: OperationAuditItem[];
  apiKeys: ApiKey[];
  apiModelOptions: ModelOption[];
  manualRouteAccountIds: string[];
  requestLogTodaySummary: RequestLogTodaySummary;
  recentRequestLogCount: number;
  latestRequestAccountId: string | null;
}
```

Update `apps/src/lib/api/normalize.ts`:

```ts
function normalizeStringList(payload: unknown): string[] {
  return asArray(payload)
    .map((item) => asString(item))
    .filter(Boolean);
}

export function normalizeGatewayRouteStrategy(payload: unknown): GatewayRouteStrategyInfo {
  const source = asObject(payload);
  return {
    strategy: asString(source.strategy) || "ordered",
    options: normalizeStringList(source.options),
    routeAccountIds: normalizeStringList(source.routeAccountIds ?? source.route_account_ids),
  };
}

export function normalizeStartupSnapshot(payload: unknown): StartupSnapshot {
  const source = asObject(payload);
  const accounts = asArray(source.accounts)
    .map((item) => normalizeAccount(item))
    .filter((item): item is Account => Boolean(item));

  return {
    accounts,
    usageAggregateSummary: normalizeUsageAggregateSummary(source.usageAggregateSummary),
    usagePredictionSummary: normalizeUsagePredictionSummary(
      source.usagePredictionSummary ?? source.usage_prediction_summary
    ),
    failureReasonSummary: normalizeFailureReasonSummary(source.failureReasonSummary),
    governanceSummary: normalizeGovernanceSummary(source.governanceSummary),
    operationAudits: normalizeOperationAudits(source.operationAudits),
    apiKeys: normalizeApiKeys(source.apiKeys),
    apiModelOptions: normalizeModelOptions(source.apiModelOptions),
    manualRouteAccountIds: normalizeStringList(
      source.manualRouteAccountIds ?? source.manual_route_account_ids
    ),
    requestLogTodaySummary: normalizeTodaySummary(source.requestLogTodaySummary),
    recentRequestLogCount: asInteger(source.recentRequestLogCount, 0, 0),
    latestRequestAccountId: asString(source.latestRequestAccountId) || null,
  };
}
```

Update `apps/src/lib/api/startup-snapshot-state.ts`:

```ts
type CurrentAccountCandidate = {
  id: string;
  availabilityLevel?: string | null;
};

export function pickCurrentAccountId(
  accounts: CurrentAccountCandidate[],
  latestRequestAccountId?: string | null,
  routeAccountIds?: string[] | null,
): string | null {
  if (!accounts.length) return null;

  const allowedIds = Array.isArray(routeAccountIds)
    ? routeAccountIds.map((item) => String(item || "").trim()).filter(Boolean)
    : [];
  const scopedAccounts = allowedIds.length
    ? accounts.filter((item) => allowedIds.includes(item.id))
    : accounts;
  if (!scopedAccounts.length) {
    return null;
  }

  const latestId = String(latestRequestAccountId || "").trim();
  if (latestId) {
    const fromLatest = scopedAccounts.find((item) => item.id === latestId);
    if (fromLatest && canParticipateInRouting(fromLatest.availabilityLevel)) {
      return fromLatest.id;
    }
  }

  return (
    scopedAccounts.find((item) => canParticipateInRouting(item.availabilityLevel))?.id ||
    scopedAccounts[0]?.id ||
    null
  );
}
```

Update `apps/src/hooks/useDashboardStats.ts`:

```ts
const currentAccountId = pickCurrentAccountId(
  accounts,
  data?.latestRequestAccountId ?? null,
  data?.manualRouteAccountIds ?? [],
);
```

Update `apps/src/components/layout/app-bootstrap.tsx`:

```ts
queryClient.prefetchQuery({
  queryKey: ["gateway", "route-accounts", addr || null],
  queryFn: () => serviceClient.getRouteAccountIds(),
  staleTime: PRIMARY_PAGE_WARMUP_STALE_TIME,
}),
```

- [ ] **Step 4: Run the contract and state tests to verify they pass**

Run:

```bash
cargo test -p codexmanager-core startup_snapshot_result_serialization_uses_route_account_ids -- --nocapture
cd apps && node --test src/lib/api/startup-snapshot-state.test.ts
```

Expected:

- Rust startup-snapshot contract test PASS
- frontend startup-snapshot state tests PASS

- [ ] **Step 5: Commit the new route-account RPC/state contract**

```bash
git add \
  crates/core/src/rpc/types.rs \
  crates/core/src/rpc/tests/types_tests.rs \
  crates/service/src/startup_snapshot.rs \
  crates/service/src/rpc_dispatch/gateway.rs \
  apps/src-tauri/src/commands/settings/gateway.rs \
  apps/src-tauri/src/commands/registry.rs \
  apps/src/lib/api/transport.ts \
  apps/src/lib/api/service-client.ts \
  apps/src/lib/api/normalize.ts \
  apps/src/lib/api/startup-snapshot-state.ts \
  apps/src/lib/api/startup-snapshot-state.test.ts \
  apps/src/components/layout/app-bootstrap.tsx \
  apps/src/hooks/useDashboardStats.ts \
  apps/src/types/index.ts
git commit -m "feat: expose route account whitelist state"
```

### Task 3: Replace the accounts-page preferred-account UX with batch route-whitelist controls

**Files:**
- Create: `apps/src/app/accounts/route-account-state.ts`
- Create: `apps/src/app/accounts/route-account-state.test.ts`
- Modify: `apps/src/hooks/useAccounts.ts`
- Modify: `apps/src/app/accounts/page.tsx`

- [ ] **Step 1: Write the failing accounts-page state tests**

Create `apps/src/app/accounts/route-account-state.test.ts`:

```ts
import test from "node:test";
import assert from "node:assert/strict";

import {
  describeRouteAccountScope,
  isRouteAccountSelected,
  normalizeRouteAccountIds,
} from "./route-account-state.ts";

test("normalizeRouteAccountIds trims, dedupes, and drops empty ids", () => {
  assert.deepEqual(
    normalizeRouteAccountIds([" acc-a ", "", "acc-a", "acc-b"]),
    ["acc-a", "acc-b"],
  );
});

test("describeRouteAccountScope reports unrestricted routing for empty whitelist", () => {
  assert.equal(describeRouteAccountScope([], ["acc-a", "acc-b"]), "全部可用账号参与路由");
});

test("describeRouteAccountScope counts only known route accounts", () => {
  assert.equal(
    describeRouteAccountScope(["acc-a", "acc-missing", "acc-b"], ["acc-a", "acc-b"]),
    "已限制为 2 个账号参与路由",
  );
});

test("isRouteAccountSelected matches exact account ids", () => {
  assert.equal(isRouteAccountSelected(["acc-a", "acc-b"], "acc-b"), true);
  assert.equal(isRouteAccountSelected(["acc-a", "acc-b"], "acc-c"), false);
});
```

- [ ] **Step 2: Run the accounts-page state tests to verify they fail**

Run:

```bash
cd apps && node --test src/app/accounts/route-account-state.test.ts
```

Expected:

- FAIL because `route-account-state.ts` does not exist yet

- [ ] **Step 3: Implement the frontend route-whitelist state helper and hook mutations**

Create `apps/src/app/accounts/route-account-state.ts`:

```ts
export function normalizeRouteAccountIds(accountIds: string[]): string[] {
  const output: string[] = [];
  const seen = new Set<string>();
  for (const accountId of accountIds) {
    const normalized = String(accountId || "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    output.push(normalized);
  }
  return output;
}

export function isRouteAccountSelected(
  routeAccountIds: string[],
  accountId: string,
): boolean {
  return normalizeRouteAccountIds(routeAccountIds).includes(String(accountId || "").trim());
}

export function describeRouteAccountScope(
  routeAccountIds: string[],
  knownAccountIds: string[],
): string {
  const known = new Set(knownAccountIds.map((item) => String(item || "").trim()).filter(Boolean));
  const effective = normalizeRouteAccountIds(routeAccountIds).filter((accountId) =>
    known.has(accountId),
  );
  if (!effective.length) {
    return "全部可用账号参与路由";
  }
  return `已限制为 ${effective.length} 个账号参与路由`;
}
```

Update `apps/src/hooks/useAccounts.ts`:

```ts
const routeAccountIdsQuery = useQuery({
  queryKey: ["gateway", "route-accounts", serviceStatus.addr || null],
  queryFn: () => serviceClient.getRouteAccountIds(),
  enabled: serviceStatus.connected,
  retry: 1,
});

const invalidateRouteAccounts = async () => {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ["gateway", "route-accounts"] }),
    queryClient.invalidateQueries({ queryKey: ["startup-snapshot"] }),
  ]);
};

const setRouteAccountsMutation = useMutation({
  mutationFn: (accountIds: string[]) => serviceClient.setRouteAccounts(accountIds),
  onSuccess: async (_result, accountIds) => {
    await invalidateRouteAccounts();
    toast.success(`已限制 ${accountIds.length} 个账号参与路由`);
  },
  onError: (error: unknown) => {
    toast.error(`设置路由账号失败: ${getAppErrorMessage(error)}`);
  },
});

const clearRouteAccountsMutation = useMutation({
  mutationFn: () => serviceClient.clearRouteAccounts(),
  onSuccess: async () => {
    await invalidateRouteAccounts();
    toast.success("已恢复全部可用账号参与路由");
  },
  onError: (error: unknown) => {
    toast.error(`清空路由限制失败: ${getAppErrorMessage(error)}`);
  },
});
```

Expose them from the hook:

```ts
routeAccountIds: routeAccountIdsQuery.data || [],
setRouteAccounts: (accountIds: string[]) => setRouteAccountsMutation.mutate(accountIds),
clearRouteAccounts: () => clearRouteAccountsMutation.mutate(),
isUpdatingRouteAccounts:
  setRouteAccountsMutation.isPending || clearRouteAccountsMutation.isPending,
```

- [ ] **Step 4: Replace the accounts-page UI actions and badges**

Update `apps/src/app/accounts/page.tsx` imports:

```ts
import {
  describeRouteAccountScope,
  isRouteAccountSelected,
} from "./route-account-state";
```

Switch the hook fields:

```ts
const {
  accounts,
  groups,
  isLoading,
  refreshAccount,
  refreshAllAccounts,
  deleteAccount,
  deleteManyAccounts,
  deleteUnavailableFree,
  deleteBannedAccounts,
  importByFile,
  importByDirectory,
  exportAccounts,
  isRefreshingAccountId,
  isRefreshingAllAccounts,
  isExporting,
  isDeletingMany,
  isDeletingBanned,
  isDeletingUnavailableFree,
  routeAccountIds,
  setRouteAccounts,
  clearRouteAccounts,
  isUpdatingRouteAccounts,
  updateAccountSort,
  isUpdatingSortAccountId,
  toggleAccountStatus,
  isUpdatingStatusAccountId,
  bulkToggleAccountStatus,
  isBulkUpdatingStatus,
  updateManyTags,
  isBulkUpdatingTags,
  checkSubscription,
  checkSubscriptions,
  markSubscription,
  markManySubscriptions,
  uploadToTeamManager,
  uploadManyToTeamManager,
  isCheckingSubscriptionAccountId,
  isCheckingSubscriptions,
  isMarkingSubscriptionAccountId,
  isMarkingManySubscriptions,
  isUploadingTeamManagerAccountId,
  isUploadingManyToTeamManager,
} = useAccounts();
```

Add handlers near the other batch actions:

```ts
const handleRestrictRouteToSelected = () => {
  if (!effectiveSelectedIds.length) {
    toast.error("请先选择要参与路由的账号");
    return;
  }
  setRouteAccounts(effectiveSelectedIds);
};

const routeScopeLabel = describeRouteAccountScope(
  routeAccountIds,
  accounts.map((account) => account.id),
);
```

Add batch dropdown items in the `账号操作` menu:

```tsx
<DropdownMenuItem
  className="h-9 rounded-lg px-2"
  disabled={!effectiveSelectedIds.length || isUpdatingRouteAccounts}
  onClick={handleRestrictRouteToSelected}
>
  <ShieldCheck className="mr-2 h-4 w-4" /> 仅所选参与路由
  <DropdownMenuShortcut>{effectiveSelectedIds.length || "-"}</DropdownMenuShortcut>
</DropdownMenuItem>
<DropdownMenuItem
  className="h-9 rounded-lg px-2"
  disabled={isUpdatingRouteAccounts}
  onClick={() => clearRouteAccounts()}
>
  <ShieldOff className="mr-2 h-4 w-4" /> 清空路由限制
</DropdownMenuItem>
```

Show the summary near the page controls:

```tsx
<div className="rounded-full bg-primary/10 px-3 py-1 text-[11px] font-medium text-primary">
  {routeScopeLabel}
</div>
```

Replace the row badge:

```tsx
{isRouteAccountSelected(routeAccountIds, account.id) ? (
  <Badge
    variant="secondary"
    className="h-4 bg-emerald-500/15 px-1.5 text-[9px] text-emerald-700 dark:text-emerald-300"
  >
    路由中
  </Badge>
) : null}
```

Delete the row-level preferred action:

```tsx
- <DropdownMenuItem
-   className="gap-2"
-   disabled={isUpdatingPreferred}
-   onClick={() =>
-     manualPreferredAccountId === account.id
-       ? clearPreferredAccount()
-       : setPreferredAccount(account.id)
-   }
- >
-   <Pin className="h-4 w-4" />
-   {manualPreferredAccountId === account.id ? "取消优先" : "设为优先"}
- </DropdownMenuItem>
```

- [ ] **Step 5: Run the frontend state tests to verify they pass**

Run:

```bash
cd apps && node --test src/app/accounts/route-account-state.test.ts src/lib/api/startup-snapshot-state.test.ts
```

Expected:

- both test files PASS

- [ ] **Step 6: Commit the accounts-page route-whitelist UX**

```bash
git add \
  apps/src/app/accounts/route-account-state.ts \
  apps/src/app/accounts/route-account-state.test.ts \
  apps/src/hooks/useAccounts.ts \
  apps/src/app/accounts/page.tsx
git commit -m "feat: manage route account whitelist from accounts page"
```

### Task 4: Update docs and run full verification

**Files:**
- Modify: `crates/service/src/gateway/README.md`

- [ ] **Step 1: Update the gateway README to describe whitelist routing instead of preferred-account pinning**

Replace the old “manual preferred account” section in `crates/service/src/gateway/README.md` with:

```md
额外覆盖规则：

- 如果设置了路由账号白名单（route account whitelist），会先按白名单过滤候选池
- 不在白名单内的账号不会参与本次路由
- 白名单内账号仍然会继续经过现有可用性、冷却、并发、模型、健康度与新号保护规则
- 清空白名单后，恢复默认的全量可用账号参与路由
```

- [ ] **Step 2: Run focused backend verification**

Run:

```bash
cargo test -p codexmanager-service route_whitelist -- --nocapture
cargo test -p codexmanager-core startup_snapshot_result_serialization_uses_route_account_ids -- --nocapture
```

Expected:

- all targeted Rust tests PASS

- [ ] **Step 3: Run frontend verification**

Run:

```bash
cd apps && node --test src/app/accounts/route-account-state.test.ts src/lib/api/startup-snapshot-state.test.ts
cd apps && pnpm run build:desktop
```

Expected:

- node tests PASS
- `next build` exits 0

- [ ] **Step 4: Run service compile verification**

Run:

```bash
cargo test -p codexmanager-service --no-run
```

Expected:

- compile completes successfully with no test failures

- [ ] **Step 5: Commit docs and final integrated changes**

```bash
git add crates/service/src/gateway/README.md
git commit -m "docs: document route account whitelist"
```
