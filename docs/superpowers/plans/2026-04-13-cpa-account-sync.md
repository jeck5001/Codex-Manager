# CPA Account Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a CLIProxyAPI / CPA sync source that lets users save a CPA API URL plus Management Key, test connectivity, and manually sync Codex auth files into the local account pool.

**Architecture:** Reuse the existing `account/import` pipeline instead of inventing a second account-ingest path. Add a focused CPA sync module in `crates/service`, expose `account/cpa/test` and `account/cpa/sync` RPCs, persist CPA settings alongside the existing Team Manager settings, and render a new settings card in the Next.js UI.

**Tech Stack:** Rust (`reqwest`, existing RPC/app-settings stack), Next.js 14 + TypeScript, TanStack Query mutations, existing SQLite-backed app settings storage.

---

### Task 1: Persist CPA Sync Settings in App Settings

**Files:**
- Modify: `crates/service/src/app_settings/shared.rs`
- Modify: `crates/service/src/app_settings/api/current.rs`
- Modify: `crates/service/src/app_settings/api/patch.rs`
- Modify: `apps/src/types/index.ts`
- Modify: `apps/src/lib/api/normalize.ts`
- Modify: `apps/src/lib/store/useAppStore.ts`

- [ ] **Step 1: Write the failing Rust settings test**

Create or extend `crates/service/tests/app_settings.rs` with a test that proves CPA settings round-trip correctly without leaking the Management Key:

```rust
#[test]
fn app_settings_set_persists_cpa_sync_snapshot_without_exposing_key() {
    codexmanager_service::initialize_storage_if_needed().expect("init storage");

    let snapshot = codexmanager_service::app_settings_set(Some(&serde_json::json!({
        "cpaSyncEnabled": true,
        "cpaSyncApiUrl": "https://cpa.example.com",
        "cpaSyncManagementKey": "mgmt-key-123",
    })))
    .expect("set settings");

    assert_eq!(snapshot.get("cpaSyncEnabled").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        snapshot.get("cpaSyncApiUrl").and_then(|v| v.as_str()),
        Some("https://cpa.example.com")
    );
    assert_eq!(
        snapshot.get("cpaSyncHasManagementKey").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(snapshot.get("cpaSyncManagementKey").is_none());
}
```

- [ ] **Step 2: Run the Rust test to verify it fails**

Run:

```bash
cargo test -p codexmanager-service app_settings_set_persists_cpa_sync_snapshot_without_exposing_key -- --nocapture
```

Expected: FAIL because the CPA settings fields and persistence hooks do not exist yet.

- [ ] **Step 3: Add the settings keys and current snapshot fields**

Update `crates/service/src/app_settings/shared.rs` and `crates/service/src/app_settings/api/current.rs` to add:

```rust
pub const APP_SETTING_CPA_SYNC_ENABLED_KEY: &str = "cpa_sync.enabled";
pub const APP_SETTING_CPA_SYNC_API_URL_KEY: &str = "cpa_sync.api_url";
pub const APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY: &str = "cpa_sync.management_key";
```

And expose the snapshot fields:

```rust
let cpa_sync_enabled = get_persisted_app_setting(APP_SETTING_CPA_SYNC_ENABLED_KEY)
    .map(|raw| parse_bool_with_default(&raw, false))
    .unwrap_or(false);
let cpa_sync_api_url =
    get_persisted_app_setting(APP_SETTING_CPA_SYNC_API_URL_KEY).unwrap_or_default();
let cpa_sync_has_management_key =
    get_persisted_app_setting(APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY).is_some();

// in serde_json::json! snapshot:
"cpaSyncEnabled": cpa_sync_enabled,
"cpaSyncApiUrl": cpa_sync_api_url,
"cpaSyncHasManagementKey": cpa_sync_has_management_key,
```

- [ ] **Step 4: Add patch handling without exposing or clearing the key by accident**

Update `crates/service/src/app_settings/api/patch.rs` to accept:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AppSettingsPatch {
    // existing fields...
    cpa_sync_enabled: Option<bool>,
    cpa_sync_api_url: Option<String>,
    cpa_sync_management_key: Option<String>,
}
```

And persist them with the same “only overwrite when provided” behavior used by Team Manager:

```rust
if let Some(enabled) = patch.cpa_sync_enabled {
    save_persisted_bool_setting(crate::APP_SETTING_CPA_SYNC_ENABLED_KEY, enabled)?;
}
if let Some(api_url) = patch.cpa_sync_api_url {
    save_persisted_app_setting(crate::APP_SETTING_CPA_SYNC_API_URL_KEY, Some(&api_url))?;
}
if let Some(management_key) = patch.cpa_sync_management_key {
    save_persisted_app_setting(
        crate::APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY,
        Some(&management_key),
    )?;
}
```

- [ ] **Step 5: Add matching TypeScript fields and normalization**

Update `apps/src/types/index.ts`, `apps/src/lib/api/normalize.ts`, and `apps/src/lib/store/useAppStore.ts`:

```ts
export interface AppSettings {
  // existing fields...
  cpaSyncEnabled: boolean;
  cpaSyncApiUrl: string;
  cpaSyncHasManagementKey: boolean;
  cpaSyncManagementKey?: string;
}
```

```ts
cpaSyncEnabled: asBoolean(source.cpaSyncEnabled, false),
cpaSyncApiUrl: asString(source.cpaSyncApiUrl),
cpaSyncHasManagementKey: asBoolean(source.cpaSyncHasManagementKey, false),
```

```ts
cpaSyncEnabled: false,
cpaSyncApiUrl: "",
cpaSyncHasManagementKey: false,
```

- [ ] **Step 6: Re-run the targeted settings test**

Run:

```bash
cargo test -p codexmanager-service app_settings_set_persists_cpa_sync_snapshot_without_exposing_key -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit the settings slice**

```bash
git add crates/service/src/app_settings/shared.rs crates/service/src/app_settings/api/current.rs crates/service/src/app_settings/api/patch.rs apps/src/types/index.ts apps/src/lib/api/normalize.ts apps/src/lib/store/useAppStore.ts crates/service/tests/app_settings.rs
git commit -m "feat: persist CPA sync settings"
```

### Task 2: Add CPA Test and Sync RPCs in `crates/service`

**Files:**
- Create: `crates/service/src/account/account_cpa_sync.rs`
- Modify: `crates/service/src/account/mod.rs`
- Modify: `crates/service/src/rpc_dispatch/account.rs`
- Test: `crates/service/src/account/tests/account_cpa_sync_tests.rs`

- [ ] **Step 1: Write the failing CPA sync tests**

Create `crates/service/src/account/tests/account_cpa_sync_tests.rs` with focused unit tests around settings validation and conversion:

```rust
#[test]
fn cpa_test_connection_rejects_missing_url() {
    let err = super::test_cpa_connection(Some(&serde_json::json!({
        "managementKey": "key-1"
    })))
    .expect_err("missing url should fail");

    assert!(err.contains("CPA API URL 未配置"));
}

#[test]
fn cpa_sync_reuses_account_import_pipeline() {
    let payloads = vec![r#"{"access_token":"a","id_token":"i","refresh_token":"r"}"#.to_string()];
    let result = super::import_cpa_payloads_for_test(payloads).expect("import");
    assert_eq!(result.created, 1);
    assert_eq!(result.failed, 0);
}
```

- [ ] **Step 2: Run the targeted CPA sync tests to verify they fail**

Run:

```bash
cargo test -p codexmanager-service account_cpa_sync -- --nocapture
```

Expected: FAIL because the module and helper functions do not exist yet.

- [ ] **Step 3: Create the CPA sync module with focused types**

Create `crates/service/src/account/account_cpa_sync.rs` with narrow structs and helpers:

```rust
#[derive(Debug, Clone, Default)]
struct CpaSyncSettings {
    enabled: bool,
    api_url: String,
    management_key: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CpaConnectionPayload {
    api_url: Option<String>,
    management_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CpaSyncResult {
    total_files: usize,
    eligible_files: usize,
    downloaded_files: usize,
    created: usize,
    updated: usize,
    failed: usize,
    imported_account_ids: Vec<String>,
    errors: Vec<String>,
}
```

- [ ] **Step 4: Implement shared settings resolution and HTTP client helpers**

Add helpers that mirror Team Manager’s pattern but remain isolated in the new module:

```rust
fn resolve_cpa_settings(payload: Option<&Value>) -> Result<CpaSyncSettings, String> {
    let parsed: CpaConnectionPayload = payload
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|err| format!("invalid CPA payload: {err}"))?
        .unwrap_or_default();

    let api_url = parsed
        .api_url
        .or_else(|| get_persisted_app_setting(APP_SETTING_CPA_SYNC_API_URL_KEY))
        .unwrap_or_default()
        .trim()
        .to_string();
    if api_url.is_empty() {
        return Err("CPA API URL 未配置".to_string());
    }

    let management_key = parsed
        .management_key
        .filter(|v| !v.trim().is_empty())
        .or_else(|| get_persisted_app_setting(APP_SETTING_CPA_SYNC_MANAGEMENT_KEY_KEY))
        .unwrap_or_default()
        .trim()
        .to_string();
    if management_key.is_empty() {
        return Err("CPA Management Key 未配置".to_string());
    }

    Ok(CpaSyncSettings {
        enabled: get_persisted_app_setting(APP_SETTING_CPA_SYNC_ENABLED_KEY)
            .map(|raw| parse_bool_with_default(&raw, false))
            .unwrap_or(false),
        api_url,
        management_key,
    })
}
```

- [ ] **Step 5: Implement `account/cpa/test`**

Use `GET /v0/management/auth-files` and accept either saved or just-entered credentials:

```rust
pub(crate) fn test_cpa_connection(params: Option<&Value>) -> Result<Value, String> {
    let settings = resolve_cpa_settings(params)?;
    let response = cpa_http_client()?
        .get(format!("{}/v0/management/auth-files", settings.api_url.trim_end_matches('/')))
        .bearer_auth(&settings.management_key)
        .send()
        .map_err(|err| format!("CPA 连接失败: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("CPA 连接失败: HTTP {}", response.status().as_u16()));
    }

    let payload: Value = response
        .json()
        .map_err(|err| format!("invalid CPA auth-files response: {err}"))?;
    let count = payload
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| items.len())
        .unwrap_or(0);

    Ok(serde_json::json!({
        "success": true,
        "message": format!("CPA 连接测试成功，可见 {} 个 auth 文件", count),
        "authFileCount": count,
    }))
}
```

- [ ] **Step 6: Implement `account/cpa/sync` by transforming remote auth files into import payloads**

Keep the sync loop explicit and small:

```rust
pub(crate) fn sync_cpa_accounts(params: Option<&Value>) -> Result<Value, String> {
    let settings = resolve_cpa_settings(params)?;
    let list = fetch_cpa_auth_files(&settings)?;
    let mut import_payloads = Vec::new();
    let mut errors = Vec::new();

    for item in list {
        match download_and_convert_auth_file(&settings, &item) {
            Ok(Some(payload)) => import_payloads.push(payload),
            Ok(None) => {}
            Err(err) => errors.push(err),
        }
    }

    let import_result = crate::account_import::import_account_auth_json(import_payloads)?;
    Ok(serde_json::json!({
        "totalFiles": list.len(),
        "eligibleFiles": import_result.total + errors.len(),
        "downloadedFiles": import_result.total,
        "created": import_result.created,
        "updated": import_result.updated,
        "failed": import_result.failed + errors.len(),
        "errors": errors,
    }))
}
```

- [ ] **Step 7: Wire the module into account mod + RPC dispatch**

Update:

```rust
// crates/service/src/account/mod.rs
#[path = "account_cpa_sync.rs"]
pub(crate) mod cpa_sync;
```

```rust
// crates/service/src/rpc_dispatch/account.rs
"account/cpa/test" => super::value_or_error(account_cpa_sync::test_cpa_connection(req.params.as_ref())),
"account/cpa/sync" => super::value_or_error(account_cpa_sync::sync_cpa_accounts(req.params.as_ref())),
```

- [ ] **Step 8: Run the targeted CPA sync tests**

Run:

```bash
cargo test -p codexmanager-service account_cpa_sync -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Commit the service slice**

```bash
git add crates/service/src/account/account_cpa_sync.rs crates/service/src/account/mod.rs crates/service/src/rpc_dispatch/account.rs crates/service/src/account/tests/account_cpa_sync_tests.rs
git commit -m "feat: add CPA account sync RPCs"
```

### Task 3: Add CPA Settings UI and Mutations

**Files:**
- Modify: `apps/src/lib/api/transport.ts`
- Modify: `apps/src/lib/api/account-client.ts`
- Modify: `apps/src/app/settings/page.tsx`

- [ ] **Step 1: Write the failing UI/API type check**

Add the new client methods first so TypeScript has a failing contract. Introduce calls in `settings/page.tsx` before implementing them:

```ts
await accountClient.testCpa(cpaApiUrl, cpaManagementKey)
await accountClient.syncCpaAccounts()
```

- [ ] **Step 2: Run the front-end type check to verify it fails**

Run:

```bash
cd apps && pnpm exec tsc --noEmit
```

Expected: FAIL because `testCpa` and `syncCpaAccounts` are missing.

- [ ] **Step 3: Add transport and account client methods**

Update `apps/src/lib/api/transport.ts` and `apps/src/lib/api/account-client.ts`:

```ts
service_account_cpa_test: { rpcMethod: "account/cpa/test" },
service_account_cpa_sync: { rpcMethod: "account/cpa/sync" },
```

```ts
testCpa: (apiUrl?: string | null, managementKey?: string | null) =>
  invoke<{ success: boolean; message: string; authFileCount?: number }>(
    "service_account_cpa_test",
    withAddr({ apiUrl: apiUrl ?? null, managementKey: managementKey ?? null })
  ),
syncCpaAccounts: (apiUrl?: string | null, managementKey?: string | null) =>
  invoke<{
    totalFiles: number;
    eligibleFiles: number;
    downloadedFiles: number;
    created: number;
    updated: number;
    failed: number;
    errors: string[];
  }>(
    "service_account_cpa_sync",
    withAddr({ apiUrl: apiUrl ?? null, managementKey: managementKey ?? null })
  ),
```

- [ ] **Step 4: Add the CPA card to the settings page**

Mirror the Team Manager card pattern in `apps/src/app/settings/page.tsx`:

```tsx
<Card className="glass-card border-none shadow-md">
  <CardHeader>
    <div className="flex items-center gap-2">
      <Globe className="h-4 w-4 text-primary" />
      <CardTitle className="text-base">CLIProxyAPI / CPA</CardTitle>
    </div>
    <CardDescription>从 CLIProxyAPI 的 auth-files 管理接口同步已登录 Codex 账号</CardDescription>
  </CardHeader>
  <CardContent className="space-y-5">
    <div className="flex items-center justify-between">
      <div className="space-y-0.5">
        <Label>启用同步</Label>
        <p className="text-xs text-muted-foreground">这里只保存同步源，不会删除 CPA 侧账号</p>
      </div>
      <Switch
        checked={snapshot.cpaSyncEnabled}
        onCheckedChange={(value) => updateSettings.mutate({ cpaSyncEnabled: value })}
      />
    </div>
    {/* URL + Management Key inputs */}
    {/* Save / Test / Sync buttons */}
  </CardContent>
</Card>
```

- [ ] **Step 5: Add local state and actions for save/test/sync**

Use the same draft-state shape as Team Manager:

```ts
const [cpaApiUrlDraft, setCpaApiUrlDraft] = useState<string | null>(null);
const [cpaManagementKeyDraft, setCpaManagementKeyDraft] = useState("");

const handleSaveCpa = async () => {
  await updateSettings.mutateAsync({
    cpaSyncEnabled: snapshot.cpaSyncEnabled,
    cpaSyncApiUrl: cpaApiUrlInput.trim(),
    ...(cpaManagementKeyDraft.trim()
      ? { cpaSyncManagementKey: cpaManagementKeyDraft.trim() }
      : {}),
  });
  setCpaManagementKeyDraft("");
};
```

And add mutations:

```ts
const testCpa = useMutation({
  mutationFn: (payload: { apiUrl?: string | null; managementKey?: string | null }) =>
    accountClient.testCpa(payload.apiUrl, payload.managementKey),
  onSuccess: (result) => toast.success(result.message || "CPA 连接测试成功"),
  onError: (error) => toast.error(`测试 CPA 失败: ${getAppErrorMessage(error)}`),
});

const syncCpa = useMutation({
  mutationFn: () =>
    accountClient.syncCpaAccounts(
      cpaApiUrlInput.trim() || null,
      cpaManagementKeyDraft.trim() || null,
    ),
  onSuccess: async (result) => {
    await queryClient.invalidateQueries({ queryKey: ["accounts"] });
    toast.success(`CPA 同步完成：新增 ${result.created}，更新 ${result.updated}，失败 ${result.failed}`);
  },
});
```

- [ ] **Step 6: Re-run the front-end type check**

Run:

```bash
cd apps && pnpm exec tsc --noEmit
```

Expected: PASS.

- [ ] **Step 7: Commit the UI slice**

```bash
git add apps/src/lib/api/transport.ts apps/src/lib/api/account-client.ts apps/src/app/settings/page.tsx
git commit -m "feat: add CPA sync settings UI"
```

### Task 4: Full Verification and Build

**Files:**
- Verify only

- [ ] **Step 1: Run the focused Rust tests**

Run:

```bash
cargo test -p codexmanager-service app_settings_set_persists_cpa_sync_snapshot_without_exposing_key account_cpa_sync -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run the front-end type check**

Run:

```bash
cd apps && pnpm exec tsc --noEmit
```

Expected: PASS.

- [ ] **Step 3: Run the required desktop build validation**

Run:

```bash
cd apps && pnpm run build:desktop
```

Expected: PASS.

- [ ] **Step 4: Sanity-check the new user flow manually**

Manual QA checklist:

```text
1. 打开“设置”页，看到新的 CLIProxyAPI / CPA 卡片
2. 保存 CPA URL，不填 key 时不会清空已保存 key
3. 点“测试连接”能拿到 auth file 数量
4. 点“立即同步”后，账号页出现新增/更新的 Codex 账号
5. 同步失败时显示摘要，不泄漏 Management Key
```

- [ ] **Step 5: Commit the verification-complete state**

```bash
git status --short
git add .
git commit -m "feat: add CLIProxyAPI account sync"
```
