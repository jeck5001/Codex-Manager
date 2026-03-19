import { invoke, withAddr } from "./transport";
import {
  normalizeAccountList,
  normalizeApiKeyCreateResult,
  normalizeApiKeyList,
  normalizeApiKeyUsageStats,
  normalizeLoginStartResult,
  normalizeModelOptions,
  normalizeUsageAggregateSummary,
  normalizeUsageList,
  normalizeUsageSnapshot,
} from "./normalize";
import {
  AccountListResult,
  AccountUsage,
  ApiKey,
  ApiKeyCreateResult,
  ApiKeyUsageStat,
  ChatgptAuthTokensRefreshResult,
  CurrentAccessTokenAccountReadResult,
  LoginStatusResult,
  LoginStartResult,
  ModelOption,
  RegisterAvailableServicesResult,
  RegisterEmailService,
  RegisterEmailServiceField,
  RegisterEmailServiceListResult,
  RegisterEmailServiceTestResult,
  RegisterEmailServiceType,
  RegisterEmailServiceTypeCatalog,
  RegisterImportResult,
  RegisterOutlookBatchImportResult,
  RegisterServiceGroup,
  RegisterTaskSnapshot,
  UsageAggregateSummary,
} from "../../types";

interface AccountImportResult {
  canceled?: boolean;
  total?: number;
  created?: number;
  updated?: number;
  failed?: number;
  fileCount?: number;
  directoryPath?: string;
  contents?: string[];
}

interface AccountExportResult {
  canceled?: boolean;
  exported?: number;
  outputDir?: string;
}

interface DeleteUnavailableFreeResult {
  deleted?: number;
}

interface LoginStartPayload {
  loginType?: string;
  openBrowser?: boolean;
  note?: string | null;
  tags?: string[] | string | null;
  group?: string | null;
  groupName?: string | null;
  workspaceId?: string | null;
}

interface ChatgptAuthTokensLoginPayload {
  accessToken: string;
  refreshToken?: string | null;
  idToken?: string | null;
  chatgptAccountId?: string | null;
  workspaceId?: string | null;
  chatgptPlanType?: string | null;
}

interface ApiKeyPayload {
  name?: string | null;
  modelSlug?: string | null;
  reasoningEffort?: string | null;
  protocolType?: string | null;
  upstreamBaseUrl?: string | null;
  staticHeadersJson?: string | null;
}

interface RegisterStartPayload {
  emailServiceType: string;
  emailServiceId?: number | null;
  proxy?: string | null;
}

interface RegisterEmailServiceListPayload {
  serviceType?: string | null;
  enabledOnly?: boolean;
}

interface RegisterEmailServiceCreatePayload {
  serviceType: string;
  name: string;
  enabled?: boolean;
  priority?: number;
  config?: Record<string, unknown>;
}

interface RegisterEmailServiceUpdatePayload {
  serviceId: number;
  name?: string | null;
  enabled?: boolean;
  priority?: number | null;
  config?: Record<string, unknown>;
}

interface RegisterOutlookBatchImportPayload {
  data: string;
  enabled?: boolean;
  priority?: number;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function normalizeRegisterServiceGroup(value: unknown): RegisterServiceGroup {
  const source = asRecord(value) ?? {};
  const services = Array.isArray(source.services) ? source.services : [];
  return {
    available: source.available === true,
    count: typeof source.count === "number" && Number.isFinite(source.count)
      ? source.count
      : services.length,
    services: services.map((item) => {
      const service = asRecord(item) ?? {};
      return {
        id: typeof service.id === "number" && Number.isFinite(service.id) ? service.id : null,
        name: typeof service.name === "string" ? service.name : "",
        type: typeof service.type === "string" ? service.type : "",
        description: typeof service.description === "string" ? service.description : "",
      };
    }),
  };
}

function normalizeRegisterAvailableServices(value: unknown): RegisterAvailableServicesResult {
  const source = asRecord(value) ?? {};
  return {
    serviceUrl: typeof source.serviceUrl === "string" ? source.serviceUrl : "",
    tempmail: normalizeRegisterServiceGroup(source.tempmail),
    outlook: normalizeRegisterServiceGroup(source.outlook),
    customDomain: normalizeRegisterServiceGroup(source.custom_domain ?? source.customDomain),
    tempMail: normalizeRegisterServiceGroup(source.temp_mail ?? source.tempMail),
  };
}

function normalizeRegisterTaskSnapshot(value: unknown): RegisterTaskSnapshot {
  const source = asRecord(value) ?? {};
  return {
    taskUuid: typeof source.taskUuid === "string"
      ? source.taskUuid
      : typeof source.task_uuid === "string"
        ? source.task_uuid
        : "",
    status: typeof source.status === "string" ? source.status : "",
    emailServiceId: typeof source.emailServiceId === "number" && Number.isFinite(source.emailServiceId)
      ? source.emailServiceId
      : typeof source.email_service_id === "number" && Number.isFinite(source.email_service_id)
        ? source.email_service_id
        : null,
    proxy: typeof source.proxy === "string" ? source.proxy : "",
    createdAt: typeof source.createdAt === "string"
      ? source.createdAt
      : typeof source.created_at === "string"
        ? source.created_at
        : "",
    startedAt: typeof source.startedAt === "string"
      ? source.startedAt
      : typeof source.started_at === "string"
        ? source.started_at
        : "",
    completedAt: typeof source.completedAt === "string"
      ? source.completedAt
      : typeof source.completed_at === "string"
        ? source.completed_at
        : "",
    errorMessage: typeof source.errorMessage === "string"
      ? source.errorMessage
      : typeof source.error_message === "string"
        ? source.error_message
        : "",
    email: typeof source.email === "string" ? source.email : "",
    canImport: source.canImport === true || source.can_import === true,
    logs: Array.isArray(source.logs)
      ? source.logs.filter((item): item is string => typeof item === "string")
      : [],
  };
}

function normalizeRegisterImportResult(value: unknown): RegisterImportResult {
  const source = asRecord(value) ?? {};
  return {
    taskUuid: typeof source.taskUuid === "string"
      ? source.taskUuid
      : typeof source.task_uuid === "string"
        ? source.task_uuid
        : "",
    email: typeof source.email === "string" ? source.email : "",
    remoteAccountId: typeof source.remoteAccountId === "number" && Number.isFinite(source.remoteAccountId)
      ? source.remoteAccountId
      : typeof source.remote_account_id === "number" && Number.isFinite(source.remote_account_id)
        ? source.remote_account_id
        : null,
    accountId: typeof source.accountId === "string"
      ? source.accountId
      : typeof source.account_id === "string"
        ? source.account_id
        : "",
    chatgptAccountId: typeof source.chatgptAccountId === "string"
      ? source.chatgptAccountId
      : typeof source.chatgpt_account_id === "string"
        ? source.chatgpt_account_id
        : "",
    workspaceId: typeof source.workspaceId === "string"
      ? source.workspaceId
      : typeof source.workspace_id === "string"
        ? source.workspace_id
        : "",
    type: typeof source.type === "string" ? source.type : "",
  };
}

function normalizeRegisterEmailServiceField(value: unknown): RegisterEmailServiceField {
  const source = asRecord(value) ?? {};
  const name = typeof source.name === "string" ? source.name : "";
  const rawDefault = source.defaultValue ?? source.default ?? null;

  return {
    name,
    label: typeof source.label === "string" ? source.label : name,
    required: source.required === true,
    defaultValue:
      typeof rawDefault === "string" || typeof rawDefault === "number" || typeof rawDefault === "boolean"
        ? rawDefault
        : null,
    placeholder: typeof source.placeholder === "string" ? source.placeholder : "",
    secret:
      source.secret === true ||
      ["password", "api_key", "refresh_token", "access_token", "admin_password"].includes(name),
  };
}

function normalizeRegisterEmailServiceType(value: unknown): RegisterEmailServiceType {
  const source = asRecord(value) ?? {};
  const configFields = Array.isArray(source.configFields)
    ? source.configFields
    : Array.isArray(source.config_fields)
      ? source.config_fields
      : [];

  return {
    value: typeof source.value === "string" ? source.value : "",
    label: typeof source.label === "string" ? source.label : "",
    description: typeof source.description === "string" ? source.description : "",
    configFields: configFields.map(normalizeRegisterEmailServiceField),
  };
}

function normalizeRegisterEmailServiceTypeCatalog(
  value: unknown
): RegisterEmailServiceTypeCatalog {
  const source = asRecord(value) ?? {};
  const types = Array.isArray(source.types) ? source.types : [];
  return {
    types: types.map(normalizeRegisterEmailServiceType).filter((item) => item.value),
  };
}

function normalizeRegisterEmailService(value: unknown): RegisterEmailService {
  const source = asRecord(value) ?? {};
  return {
    id: typeof source.id === "number" && Number.isFinite(source.id) ? source.id : 0,
    serviceType: typeof source.serviceType === "string"
      ? source.serviceType
      : typeof source.service_type === "string"
        ? source.service_type
        : "",
    name: typeof source.name === "string" ? source.name : "",
    enabled: source.enabled === true,
    priority:
      typeof source.priority === "number" && Number.isFinite(source.priority)
        ? source.priority
        : 0,
    config: asRecord(source.config) ?? {},
    lastUsed: typeof source.lastUsed === "string"
      ? source.lastUsed
      : typeof source.last_used === "string"
        ? source.last_used
        : "",
    createdAt: typeof source.createdAt === "string"
      ? source.createdAt
      : typeof source.created_at === "string"
        ? source.created_at
        : "",
    updatedAt: typeof source.updatedAt === "string"
      ? source.updatedAt
      : typeof source.updated_at === "string"
        ? source.updated_at
        : "",
  };
}

function normalizeRegisterEmailServiceList(value: unknown): RegisterEmailServiceListResult {
  const source = asRecord(value) ?? {};
  const services = Array.isArray(source.services) ? source.services : [];
  return {
    total:
      typeof source.total === "number" && Number.isFinite(source.total)
        ? source.total
        : services.length,
    services: services.map(normalizeRegisterEmailService).filter((item) => item.id > 0),
  };
}

function normalizeRegisterEmailServiceTestResult(
  value: unknown
): RegisterEmailServiceTestResult {
  const source = asRecord(value) ?? {};
  return {
    success: source.success === true,
    message: typeof source.message === "string" ? source.message : "",
    details: asRecord(source.details),
  };
}

function normalizeRegisterOutlookBatchImportResult(
  value: unknown
): RegisterOutlookBatchImportResult {
  const source = asRecord(value) ?? {};
  return {
    total: typeof source.total === "number" && Number.isFinite(source.total) ? source.total : 0,
    success:
      typeof source.success === "number" && Number.isFinite(source.success) ? source.success : 0,
    failed:
      typeof source.failed === "number" && Number.isFinite(source.failed) ? source.failed : 0,
    accounts: Array.isArray(source.accounts)
      ? source.accounts.map((item) => asRecord(item) ?? {})
      : [],
    errors: Array.isArray(source.errors)
      ? source.errors.filter((item): item is string => typeof item === "string")
      : [],
  };
}

export const accountClient = {
  async list(params?: Record<string, unknown>): Promise<AccountListResult> {
    const result = await invoke<unknown>("service_account_list", withAddr(params));
    return normalizeAccountList(result);
  },
  delete: (accountId: string) =>
    invoke("service_account_delete", withAddr({ accountId })),
  deleteMany: (accountIds: string[]) =>
    invoke("service_account_delete_many", withAddr({ accountIds })),
  deleteUnavailableFree: () =>
    invoke<DeleteUnavailableFreeResult>("service_account_delete_unavailable_free", withAddr()),
  updateSort: (accountId: string, sort: number) =>
    invoke("service_account_update", withAddr({ accountId, sort })),
  disableAccount: (accountId: string) =>
    invoke("service_account_update", withAddr({ accountId, status: "disabled" })),
  enableAccount: (accountId: string) =>
    invoke("service_account_update", withAddr({ accountId, status: "active" })),
  import: (contents: string[]) =>
    invoke<AccountImportResult>("service_account_import", withAddr({ contents })),
  async getRegisterAvailableServices(): Promise<RegisterAvailableServicesResult> {
    const result = await invoke<unknown>(
      "service_account_register_available_services",
      withAddr()
    );
    return normalizeRegisterAvailableServices(result);
  },
  async getRegisterEmailServiceTypes(): Promise<RegisterEmailServiceTypeCatalog> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_types",
      withAddr()
    );
    return normalizeRegisterEmailServiceTypeCatalog(result);
  },
  async listRegisterEmailServices(
    params?: RegisterEmailServiceListPayload
  ): Promise<RegisterEmailServiceListResult> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_list",
      withAddr({
        serviceType: params?.serviceType ?? null,
        enabledOnly: params?.enabledOnly ?? false,
      })
    );
    return normalizeRegisterEmailServiceList(result);
  },
  async readRegisterEmailServiceFull(serviceId: number): Promise<RegisterEmailService> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_read_full",
      withAddr({ serviceId })
    );
    return normalizeRegisterEmailService(result);
  },
  async createRegisterEmailService(
    params: RegisterEmailServiceCreatePayload
  ): Promise<RegisterEmailService> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_create",
      withAddr({
        serviceType: params.serviceType,
        name: params.name,
        enabled: params.enabled ?? true,
        priority: params.priority ?? 0,
        config: params.config ?? {},
      })
    );
    return normalizeRegisterEmailService(result);
  },
  async updateRegisterEmailService(
    params: RegisterEmailServiceUpdatePayload
  ): Promise<RegisterEmailService> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_update",
      withAddr({
        serviceId: params.serviceId,
        name: params.name ?? null,
        enabled: params.enabled,
        priority: params.priority ?? null,
        config: params.config ?? {},
      })
    );
    return normalizeRegisterEmailService(result);
  },
  deleteRegisterEmailService: (serviceId: number) =>
    invoke("service_account_register_email_services_delete", withAddr({ serviceId })),
  async testRegisterEmailService(
    serviceId: number
  ): Promise<RegisterEmailServiceTestResult> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_test",
      withAddr({ serviceId })
    );
    return normalizeRegisterEmailServiceTestResult(result);
  },
  setRegisterEmailServiceEnabled: (serviceId: number, enabled: boolean) =>
    invoke(
      "service_account_register_email_services_set_enabled",
      withAddr({ serviceId, enabled })
    ),
  async outlookBatchImportRegisterEmailServices(
    params: RegisterOutlookBatchImportPayload
  ): Promise<RegisterOutlookBatchImportResult> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_outlook_batch_import",
      withAddr({
        data: params.data,
        enabled: params.enabled ?? true,
        priority: params.priority ?? 0,
      })
    );
    return normalizeRegisterOutlookBatchImportResult(result);
  },
  async startRegisterTask(params: RegisterStartPayload): Promise<RegisterTaskSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_start",
      withAddr({
        emailServiceType: params.emailServiceType,
        emailServiceId: params.emailServiceId ?? null,
        proxy: params.proxy ?? null,
      })
    );
    return normalizeRegisterTaskSnapshot(result);
  },
  async getRegisterTask(taskUuid: string): Promise<RegisterTaskSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_task",
      withAddr({ taskUuid })
    );
    return normalizeRegisterTaskSnapshot(result);
  },
  async importRegisterTask(taskUuid: string): Promise<RegisterImportResult> {
    const result = await invoke<unknown>(
      "service_account_register_import",
      withAddr({ taskUuid })
    );
    return normalizeRegisterImportResult(result);
  },
  async importByDirectory(): Promise<AccountImportResult> {
    const picked = await invoke<AccountImportResult>(
      "service_account_import_by_directory",
      withAddr()
    );
    if (picked?.canceled || !Array.isArray(picked?.contents) || picked.contents.length === 0) {
      return picked;
    }

    const imported = await invoke<AccountImportResult>(
      "service_account_import",
      withAddr({ contents: picked.contents })
    );
    return {
      ...imported,
      canceled: false,
      directoryPath: picked.directoryPath || "",
      fileCount: picked.fileCount || picked.contents.length,
    };
  },
  async importByFile(): Promise<AccountImportResult> {
    const picked = await invoke<AccountImportResult>(
      "service_account_import_by_file",
      withAddr()
    );
    if (picked?.canceled || !Array.isArray(picked?.contents) || picked.contents.length === 0) {
      return picked;
    }

    const imported = await invoke<AccountImportResult>(
      "service_account_import",
      withAddr({ contents: picked.contents })
    );
    return {
      ...imported,
      canceled: false,
      fileCount: picked.fileCount || picked.contents.length,
    };
  },
  export: () =>
    invoke<AccountExportResult>("service_account_export_by_account_files", withAddr()),

  async getUsage(accountId: string): Promise<AccountUsage | null> {
    const result = await invoke<unknown>("service_usage_read", withAddr({ accountId }));
    const source =
      result && typeof result === "object" && "snapshot" in result
        ? (result as { snapshot?: unknown }).snapshot
        : result;
    return normalizeUsageSnapshot(source);
  },
  async listUsage(): Promise<AccountUsage[]> {
    const result = await invoke<unknown>("service_usage_list", withAddr());
    return normalizeUsageList(result);
  },
  refreshUsage: (accountId?: string) =>
    invoke(
      "service_usage_refresh",
      withAddr(accountId ? { accountId } : {})
    ),
  async aggregateUsage(): Promise<UsageAggregateSummary> {
    const result = await invoke<unknown>("service_usage_aggregate", withAddr());
    return normalizeUsageAggregateSummary(result);
  },

  async startLogin(params: LoginStartPayload): Promise<LoginStartResult> {
    const result = await invoke<unknown>(
      "service_login_start",
      withAddr({
        loginType: params?.loginType || "chatgpt",
        openBrowser: params?.openBrowser ?? true,
        note: params?.note || null,
        tags: Array.isArray(params?.tags)
          ? params.tags
              .map((item: string) => String(item || "").trim())
              .filter(Boolean)
              .join(",")
          : params?.tags || null,
        groupName: params?.group || params?.groupName || null,
        workspaceId: params?.workspaceId || null,
      })
    );
    return normalizeLoginStartResult(result);
  },
  async getLoginStatus(loginId: string): Promise<LoginStatusResult> {
    const result = await invoke<unknown>("service_login_status", withAddr({ loginId }));
    const source =
      result && typeof result === "object" && !Array.isArray(result)
        ? (result as Record<string, unknown>)
        : {};
    return {
      status: typeof source.status === "string" ? source.status.trim() : "",
      error: typeof source.error === "string" ? source.error.trim() : "",
    };
  },
  completeLogin: (state: string, code: string, redirectUri: string) =>
    invoke("service_login_complete", withAddr({ state, code, redirectUri })),
  loginWithChatgptAuthTokens: (params: ChatgptAuthTokensLoginPayload) =>
    invoke("service_login_chatgpt_auth_tokens", withAddr({
      accessToken: params.accessToken,
      refreshToken: params.refreshToken || null,
      idToken: params.idToken || null,
      chatgptAccountId: params.chatgptAccountId || null,
      workspaceId: params.workspaceId || null,
      chatgptPlanType: params.chatgptPlanType || null,
    })),
  async readCurrentAccessTokenAccount(
    refreshToken = false
  ): Promise<CurrentAccessTokenAccountReadResult> {
    const result = await invoke<unknown>(
      "service_account_read",
      withAddr({ refreshToken })
    );
    const source =
      result && typeof result === "object" && !Array.isArray(result)
        ? (result as Record<string, unknown>)
        : {};
    return {
      account:
        source.account && typeof source.account === "object" && !Array.isArray(source.account)
          ? (source.account as CurrentAccessTokenAccountReadResult["account"])
          : null,
      authMode: typeof source.authMode === "string" ? source.authMode : null,
      requiresOpenaiAuth: Boolean(source.requiresOpenaiAuth),
    };
  },
  logoutCurrentAccessTokenAccount: () =>
    invoke("service_account_logout", withAddr()),
  async refreshChatgptAuthTokens(
    previousAccountId?: string
  ): Promise<ChatgptAuthTokensRefreshResult> {
    const result = await invoke<unknown>(
      "service_chatgpt_auth_tokens_refresh",
      withAddr({ previousAccountId: previousAccountId || null })
    );
    const source =
      result && typeof result === "object" && !Array.isArray(result)
        ? (result as Record<string, unknown>)
        : {};
    return {
      accountId: String(source.accountId || "").trim(),
      accessToken: String(source.accessToken || "").trim(),
      chatgptAccountId: String(source.chatgptAccountId || "").trim(),
      chatgptPlanType:
        typeof source.chatgptPlanType === "string"
          ? source.chatgptPlanType.trim()
          : null,
      chatgptPlanTypeRaw:
        typeof source.chatgptPlanTypeRaw === "string"
          ? source.chatgptPlanTypeRaw.trim()
          : null,
    };
  },

  async listApiKeys(): Promise<ApiKey[]> {
    const result = await invoke<unknown>("service_apikey_list", withAddr());
    return normalizeApiKeyList(result);
  },
  async createApiKey(params: ApiKeyPayload): Promise<ApiKeyCreateResult> {
    const result = await invoke<unknown>(
      "service_apikey_create",
      withAddr({
        name: params.name || null,
        modelSlug: params.modelSlug || null,
        reasoningEffort: params.reasoningEffort || null,
        protocolType: params.protocolType || null,
        upstreamBaseUrl: params.upstreamBaseUrl || null,
        staticHeadersJson: params.staticHeadersJson || null,
      })
    );
    return normalizeApiKeyCreateResult(result);
  },
  async listApiKeyUsageStats(): Promise<ApiKeyUsageStat[]> {
    const result = await invoke<unknown>("service_apikey_usage_stats", withAddr());
    return normalizeApiKeyUsageStats(result);
  },
  deleteApiKey: (keyId: string) =>
    invoke("service_apikey_delete", withAddr({ keyId })),
  updateApiKey: (keyId: string, params: ApiKeyPayload) =>
    invoke(
      "service_apikey_update_model",
      withAddr({
        keyId,
        modelSlug: params.modelSlug || null,
        reasoningEffort: params.reasoningEffort || null,
        protocolType: params.protocolType || null,
        upstreamBaseUrl: params.upstreamBaseUrl || null,
        staticHeadersJson: params.staticHeadersJson || null,
      })
    ),
  disableApiKey: (keyId: string) =>
    invoke("service_apikey_disable", withAddr({ keyId })),
  enableApiKey: (keyId: string) =>
    invoke("service_apikey_enable", withAddr({ keyId })),
  async listModels(refreshRemote?: boolean): Promise<ModelOption[]> {
    const result = await invoke<unknown>(
      "service_apikey_models",
      withAddr({ refreshRemote })
    );
    return normalizeModelOptions(result);
  },
  async readApiKeySecret(keyId: string): Promise<string> {
    const result = await invoke<{ key?: string }>(
      "service_apikey_read_secret",
      withAddr({ keyId })
    );
    return String(result?.key || "").trim();
  },
};
