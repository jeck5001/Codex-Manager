import type {
  ActionResult,
  AccountRecord,
  ApiKeyRecord,
  AppSettingsSnapshot,
  RequestLogRecord,
  StartupSnapshot,
  UpdateActionResponse,
  UpdateCheckResponse,
  UpdatePrepareResponse,
  UpdateStatusResponse,
  UsageSnapshot,
} from "@/lib/codex/types";
import { resolveBusinessErrorMessage, unwrapRpcError, unwrapRpcResult } from "@/lib/codex/rpc-payload";
import { normalizeSettingsSnapshot } from "@/lib/codex/settings";

type RpcOptions = {
  signal?: AbortSignal;
  timeoutMs?: number;
};

type AccountListOptions = {
  page?: number;
  pageSize?: number;
  query?: string;
  filter?: string;
  groupFilter?: string;
};

type LoginStartPayload = {
  loginType?: string;
  openBrowser?: boolean;
  note?: string | null;
  tags?: string | null;
  groupName?: string | null;
  workspaceId?: string | null;
};

type ApiKeyProfile = {
  protocolType?: string | null;
  upstreamBaseUrl?: string | null;
  staticHeadersJson?: string | null;
};

type TauriBridge = {
  core?: {
    invoke: <T>(method: string, params?: Record<string, unknown>) => Promise<T>;
  };
};

type TimeoutHandle = {
  signal?: AbortSignal;
  dispose?: () => void;
};

function unwrapUsageSnapshot(payload: unknown): UsageSnapshot | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const record = payload as Record<string, unknown>;
  const snapshot = record.snapshot;
  if (snapshot && typeof snapshot === "object" && !Array.isArray(snapshot)) {
    return snapshot as UsageSnapshot;
  }

  return payload as UsageSnapshot;
}

function isAbortError(error: unknown) {
  return error instanceof Error && error.name === "AbortError";
}

function withTimeout(signal: AbortSignal | undefined, timeoutMs: number | undefined): TimeoutHandle {
  if (!timeoutMs || timeoutMs <= 0) {
    return { signal };
  }
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  if (signal) {
    signal.addEventListener("abort", () => controller.abort(), { once: true });
  }

  return {
    signal: controller.signal,
    dispose: () => clearTimeout(timer),
  };
}

export function isTauriRuntime() {
  const tauri =
    typeof window === "undefined"
      ? undefined
      : (window as Window & { __TAURI__?: TauriBridge }).__TAURI__;
  return Boolean(tauri && tauri.core && tauri.core.invoke);
}

async function invoke<T>(method: string, params?: Record<string, unknown>, options: RpcOptions = {}) {
  const result = await invokeRaw<unknown>(method, params, options);
  const businessError = resolveBusinessErrorMessage(result);
  if (businessError) {
    throw new Error(businessError);
  }
  return result as T;
}

async function invokeRaw<T>(method: string, params?: Record<string, unknown>, options: RpcOptions = {}) {
  const tauri =
    typeof window === "undefined"
      ? undefined
      : (window as Window & { __TAURI__?: TauriBridge }).__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new Error("桌面接口不可用（请在桌面端运行）");
  }

  const timeout = withTimeout(options.signal, options.timeoutMs);
  try {
    return (await tauri.core.invoke(method, params || {})) as T;
  } finally {
    timeout?.dispose?.();
  }
}

let rpcRequestId = 1;

async function rpcInvoke<T>(method: string, params?: Record<string, unknown>, options: RpcOptions = {}) {
  const timeout = withTimeout(options.signal, options.timeoutMs ?? 8000);
  try {
    const response = await fetch("/api/rpc", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: rpcRequestId++,
        method,
        params: params == null ? undefined : params,
      }),
      signal: timeout?.signal,
    });

    if (!response.ok) {
      const detail = (await response.text()).trim();
      if (detail && detail !== "{}") {
        throw new Error(`RPC 请求失败（HTTP ${response.status}）：${detail}`);
      }
      throw new Error(`RPC 请求失败（HTTP ${response.status}）`);
    }

    const payload = (await response.json()) as Record<string, unknown>;
    const rpcError = unwrapRpcError(payload);
    if (rpcError) {
      throw new Error(rpcError);
    }

    const result = Object.prototype.hasOwnProperty.call(payload, "result")
      ? payload.result
      : payload;
    const businessError = resolveBusinessErrorMessage(result);
    if (businessError) {
      throw new Error(businessError);
    }
    return result as T;
  } finally {
    timeout?.dispose?.();
  }
}

function withServiceAddress(address: string, extra?: Record<string, unknown>) {
  return {
    addr: address || null,
    ...(extra || {}),
  };
}

async function serviceCall<T>(
  address: string,
  tauriMethod: string,
  rpcMethod: string,
  payload?: Record<string, unknown>,
  options?: RpcOptions,
) {
  if (isTauriRuntime()) {
    const response = await invokeRaw<Record<string, unknown>>(tauriMethod, withServiceAddress(address, payload), options);
    return unwrapRpcResult<T>(response);
  }
  return rpcInvoke<T>(rpcMethod, payload, options);
}

export async function serviceInitialize(address: string, options?: RpcOptions) {
  return serviceCall<{ server_name?: string; version?: string }>(
    address,
    "service_initialize",
    "initialize",
    undefined,
    options,
  );
}

export async function serviceStartupSnapshot(address: string, requestLogLimit = 300) {
  return serviceCall<StartupSnapshot>(
    address,
    "service_startup_snapshot",
    "startup/snapshot",
    { requestLogLimit },
  );
}

export async function serviceStart(address: string) {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持启动/停止服务，请手动启动 codexmanager-service");
  }
  return invoke<ActionResult>("service_start", { addr: address });
}

export async function serviceStop() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持启动/停止服务，请手动停止 codexmanager-service");
  }
  return invoke<ActionResult>("service_stop", {});
}

export async function appSettingsGet() {
  const result = isTauriRuntime()
    ? await invoke<Record<string, unknown>>("app_settings_get", {})
    : await rpcInvoke<Record<string, unknown>>("appSettings/get");
  return normalizeSettingsSnapshot(result);
}

export async function appSettingsSet(patch: Partial<AppSettingsSnapshot> & Record<string, unknown>) {
  const payload = patch as Record<string, unknown>;
  const result = isTauriRuntime()
    ? await invoke<Record<string, unknown>>("app_settings_set", { patch: payload })
    : await rpcInvoke<Record<string, unknown>>("appSettings/set", payload);
  return normalizeSettingsSnapshot(result);
}

export async function serviceAccountList(address: string, options: AccountListOptions = {}) {
  return serviceCall<{ items: AccountRecord[]; total?: number; page?: number; pageSize?: number }>(
    address,
    "service_account_list",
    "account/list",
    Object.fromEntries(
      Object.entries({
        page: options.page,
        pageSize: options.pageSize,
        query: options.query?.trim() || undefined,
        filter: options.filter?.trim() || undefined,
        groupFilter: options.groupFilter && options.groupFilter !== "all" ? options.groupFilter : undefined,
      }).filter(([, value]) => value != null),
    ),
  );
}

export async function serviceUsageList(address: string) {
  return serviceCall<{ items: UsageSnapshot[] }>(
    address,
    "service_usage_list",
    "account/usage/list",
  );
}

export async function serviceUsageAggregate(address: string) {
  return serviceCall<Record<string, unknown>>(
    address,
    "service_usage_aggregate",
    "account/usage/aggregate",
  );
}

export async function serviceUsageRead(address: string, accountId: string) {
  const result = await serviceCall<UsageSnapshot | { snapshot?: UsageSnapshot | null }>(
    address,
    "service_usage_read",
    "account/usage/read",
    { accountId },
  );
  return unwrapUsageSnapshot(result);
}

export async function serviceUsageRefresh(address: string, accountId?: string) {
  return serviceCall<ActionResult>(
    address,
    "service_usage_refresh",
    "account/usage/refresh",
    accountId ? { accountId } : undefined,
  );
}

export async function serviceRequestLogList(address: string, query?: string, limit = 300, options?: RpcOptions) {
  return serviceCall<{ items: RequestLogRecord[] }>(
    address,
    "service_requestlog_list",
    "requestlog/list",
    {
      query: query?.trim() || undefined,
      limit,
    },
    options,
  );
}

export async function serviceRequestLogClear(address: string) {
  return serviceCall<ActionResult>(
    address,
    "service_requestlog_clear",
    "requestlog/clear",
  );
}

export async function serviceRequestLogTodaySummary(address: string) {
  return serviceCall<Record<string, unknown>>(
    address,
    "service_requestlog_today_summary",
    "requestlog/today_summary",
  );
}

export async function serviceApiKeyList(address: string) {
  return serviceCall<{ items: ApiKeyRecord[] }>(
    address,
    "service_apikey_list",
    "apikey/list",
  );
}

export async function serviceApiKeyModels(address: string, refreshRemote = false) {
  return serviceCall<{ items: Array<{ slug: string; displayName?: string | null }> }>(
    address,
    "service_apikey_models",
    "apikey/models",
    refreshRemote ? { refreshRemote } : undefined,
  );
}

export async function serviceApiKeyCreate(
  address: string,
  name: string | null,
  modelSlug: string | null,
  reasoningEffort: string | null,
  profile: ApiKeyProfile = {},
) {
  return serviceCall<{ id?: string; key?: string }>(
    address,
    "service_apikey_create",
    "apikey/create",
    {
      name,
      modelSlug,
      reasoningEffort,
      protocolType: profile.protocolType || null,
      upstreamBaseUrl: profile.upstreamBaseUrl || null,
      staticHeadersJson: profile.staticHeadersJson || null,
    },
  );
}

export async function serviceApiKeyDelete(address: string, keyId: string) {
  return serviceCall<ActionResult>(
    address,
    "service_apikey_delete",
    "apikey/delete",
    { id: keyId, keyId },
  );
}

export async function serviceApiKeyDisable(address: string, keyId: string) {
  return serviceCall<ActionResult>(
    address,
    "service_apikey_disable",
    "apikey/disable",
    { id: keyId, keyId },
  );
}

export async function serviceApiKeyEnable(address: string, keyId: string) {
  return serviceCall<ActionResult>(
    address,
    "service_apikey_enable",
    "apikey/enable",
    { id: keyId, keyId },
  );
}

export async function serviceApiKeyReadSecret(address: string, keyId: string) {
  return serviceCall<{ id?: string; key?: string }>(
    address,
    "service_apikey_read_secret",
    "apikey/readSecret",
    { id: keyId, keyId },
  );
}

export async function serviceApiKeyUpdateModel(
  address: string,
  keyId: string,
  modelSlug: string | null,
  reasoningEffort: string | null,
  profile: ApiKeyProfile = {},
) {
  return serviceCall<ActionResult>(
    address,
    "service_apikey_update_model",
    "apikey/updateModel",
    {
      id: keyId,
      keyId,
      modelSlug,
      reasoningEffort,
      protocolType: profile.protocolType || null,
      upstreamBaseUrl: profile.upstreamBaseUrl || null,
      staticHeadersJson: profile.staticHeadersJson || null,
    },
  );
}

export async function serviceLoginStart(address: string, payload: LoginStartPayload) {
  return serviceCall<{
    authUrl?: string;
    loginId?: string;
    warning?: string;
  }>(
    address,
    "service_login_start",
    "account/login/start",
    {
      type: payload.loginType || "chatgpt",
      loginType: payload.loginType || "chatgpt",
      openBrowser: payload.openBrowser !== false,
      note: payload.note || null,
      tags: payload.tags || null,
      groupName: payload.groupName || null,
      workspaceId: payload.workspaceId || null,
    },
  );
}

export async function serviceLoginStatus(address: string, loginId: string, options?: RpcOptions) {
  return serviceCall<{ status?: string; error?: string }>(
    address,
    "service_login_status",
    "account/login/status",
    { loginId },
    options,
  );
}

export async function serviceLoginComplete(address: string, loginState: string, code: string, redirectUri?: string) {
  return serviceCall<ActionResult>(
    address,
    "service_login_complete",
    "account/login/complete",
    { state: loginState, code, redirectUri },
  );
}

export async function serviceAccountDelete(address: string, accountId: string) {
  return serviceCall<ActionResult>(
    address,
    "service_account_delete",
    "account/delete",
    { accountId },
  );
}

export async function serviceAccountDeleteUnavailableFree(address: string) {
  return serviceCall<ActionResult>(
    address,
    "service_account_delete_unavailable_free",
    "account/deleteUnavailableFree",
  );
}

export async function serviceAccountDeleteMany(address: string, accountIds: string[]) {
  return serviceCall<ActionResult>(
    address,
    "service_account_delete_many",
    "account/deleteMany",
    { accountIds },
  );
}

export async function serviceAccountUpdate(address: string, accountId: string, sort: number) {
  return serviceCall<ActionResult>(
    address,
    "service_account_update",
    "account/update",
    { accountId, sort },
  );
}

export async function serviceAccountImport(address: string, contents: string[]) {
  return serviceCall<ActionResult>(
    address,
    "service_account_import",
    "account/import",
    { contents },
  );
}

export async function serviceAccountImportByDirectory(address: string) {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式暂不支持导入文件夹，请使用桌面端");
  }
  return invoke<{
    canceled?: boolean;
    contents?: string[];
    fileCount?: number;
    directoryPath?: string;
  }>("service_account_import_by_directory", withServiceAddress(address));
}

export async function serviceAccountExportByAccountFiles(address: string) {
  return serviceCall<{
    canceled?: boolean;
    exported?: number;
    skippedMissingToken?: number;
    outputDir?: string;
    files?: Array<{ fileName?: string; content?: string }>;
  }>(
    address,
    "service_account_export_by_account_files",
    "account/exportData",
  );
}

export async function serviceGatewayManualAccountGet(address: string) {
  return serviceCall<{ accountId?: string }>(
    address,
    "service_gateway_manual_account_get",
    "gateway/manualAccount/get",
  );
}

export async function serviceGatewayManualAccountSet(address: string, accountId: string) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_manual_account_set",
    "gateway/manualAccount/set",
    { accountId },
  );
}

export async function serviceGatewayManualAccountClear(address: string) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_manual_account_clear",
    "gateway/manualAccount/clear",
  );
}

export async function serviceGatewayRouteStrategySet(address: string, strategy: string) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_route_strategy_set",
    "gateway/routeStrategy/set",
    { strategy },
  );
}

export async function serviceGatewayHeaderPolicySet(
  address: string,
  cpaNoCookieHeaderModeEnabled: boolean,
) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_header_policy_set",
    "gateway/headerPolicy/set",
    { cpaNoCookieHeaderModeEnabled },
  );
}

export async function serviceGatewayUpstreamProxySet(address: string, proxyUrl: string | null) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_upstream_proxy_set",
    "gateway/upstreamProxy/set",
    { proxyUrl },
  );
}

export async function serviceGatewayTransportSet(
  address: string,
  payload: { sseKeepaliveIntervalMs: number; upstreamStreamTimeoutMs: number },
) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_transport_set",
    "gateway/transport/set",
    payload,
  );
}

export async function serviceGatewayBackgroundTasksSet(
  address: string,
  payload: Record<string, unknown>,
) {
  return serviceCall<ActionResult>(
    address,
    "service_gateway_background_tasks_set",
    "gateway/backgroundTasks/set",
    payload,
  );
}

export async function openInBrowser(url: string) {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return { ok: true };
  }
  return invoke<ActionResult>("open_in_browser", { url });
}

export async function appWindowUnsavedDraftSectionsSet(sections: string[]) {
  if (!isTauriRuntime()) {
    return;
  }
  await invoke<void>("app_window_unsaved_draft_sections_set", { sections });
}

export async function appUpdateCheck() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持桌面端更新");
  }
  return invoke<UpdateCheckResponse>("app_update_check", {});
}

export async function appUpdatePrepare() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持桌面端更新");
  }
  return invoke<UpdatePrepareResponse>("app_update_prepare", {});
}

export async function appUpdateApplyPortable() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持桌面端更新");
  }
  return invoke<UpdateActionResponse>("app_update_apply_portable", {});
}

export async function appUpdateLaunchInstaller() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持桌面端更新");
  }
  return invoke<UpdateActionResponse>("app_update_launch_installer", {});
}

export async function appUpdateStatus() {
  if (!isTauriRuntime()) {
    throw new Error("浏览器模式不支持桌面端更新");
  }
  return invoke<UpdateStatusResponse>("app_update_status", {});
}

export function getErrorMessage(error: unknown) {
  if (isAbortError(error)) {
    return "请求已取消";
  }
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  return String(error || "未知错误");
}
