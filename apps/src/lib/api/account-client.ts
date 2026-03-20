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
  AccountBulkStatusUpdateResult,
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
  RegisterBatchSnapshot,
  RegisterBatchStartResult,
  RegisterEmailServiceBatchDeleteResult,
  RegisterEmailService,
  RegisterEmailServiceField,
  RegisterEmailServiceListResult,
  RegisterEmailServiceStats,
  RegisterEmailServiceTestResult,
  RegisterEmailServiceType,
  RegisterEmailServiceTypeCatalog,
  RegisterImportResult,
  RegisterOutlookAccount,
  RegisterOutlookAccountsResult,
  RegisterOutlookBatchSnapshot,
  RegisterOutlookBatchStartResult,
  RegisterOutlookBatchImportResult,
  RegisterServiceGroup,
  RegisterStats,
  RegisterTaskListResult,
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

interface RegisterBatchStartPayload extends RegisterStartPayload {
  count: number;
  intervalMin: number;
  intervalMax: number;
  concurrency: number;
  mode: "pipeline" | "parallel";
}

interface RegisterOutlookBatchStartPayload {
  serviceIds: number[];
  skipRegistered?: boolean;
  proxy?: string | null;
  intervalMin: number;
  intervalMax: number;
  concurrency: number;
  mode: "pipeline" | "parallel";
}

interface RegisterTaskListPayload {
  page?: number;
  pageSize?: number;
  status?: string | null;
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

interface RegisterEmailServiceReorderPayload {
  serviceIds: number[];
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
  const result = asRecord(source.result);
  const emailFromResult = typeof result?.email === "string" ? result.email : "";
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
    email: typeof source.email === "string" && source.email
      ? source.email
      : emailFromResult,
    canImport:
      source.canImport === true ||
      source.can_import === true ||
      (typeof emailFromResult === "string" &&
        emailFromResult.trim().length > 0 &&
        String(source.status || "").trim().toLowerCase() === "completed"),
    logs: Array.isArray(source.logs)
      ? source.logs.filter((item): item is string => typeof item === "string")
      : [],
  };
}

function normalizeRegisterBatchStartResult(value: unknown): RegisterBatchStartResult {
  const source = asRecord(value) ?? {};
  const tasks = Array.isArray(source.tasks) ? source.tasks : [];
  const taskUuids = Array.isArray(source.taskUuids)
    ? source.taskUuids
    : Array.isArray(source.task_uuids)
      ? source.task_uuids
      : tasks.map((item) => asRecord(item)?.task_uuid ?? asRecord(item)?.taskUuid);

  return {
    batchId: typeof source.batchId === "string"
      ? source.batchId
      : typeof source.batch_id === "string"
        ? source.batch_id
        : "",
    count:
      typeof source.count === "number" && Number.isFinite(source.count)
        ? source.count
        : Array.isArray(taskUuids)
          ? taskUuids.length
          : 0,
    taskUuids: Array.isArray(taskUuids)
      ? taskUuids.filter((item): item is string => typeof item === "string" && item.trim().length > 0)
      : [],
  };
}

function normalizeRegisterBatchSnapshot(value: unknown): RegisterBatchSnapshot {
  const source = asRecord(value) ?? {};
  return {
    batchId: typeof source.batchId === "string"
      ? source.batchId
      : typeof source.batch_id === "string"
        ? source.batch_id
        : "",
    total: typeof source.total === "number" && Number.isFinite(source.total) ? source.total : 0,
    completed:
      typeof source.completed === "number" && Number.isFinite(source.completed)
        ? source.completed
        : 0,
    success:
      typeof source.success === "number" && Number.isFinite(source.success) ? source.success : 0,
    failed:
      typeof source.failed === "number" && Number.isFinite(source.failed) ? source.failed : 0,
    currentIndex:
      typeof source.currentIndex === "number" && Number.isFinite(source.currentIndex)
        ? source.currentIndex
        : typeof source.current_index === "number" && Number.isFinite(source.current_index)
          ? source.current_index
          : 0,
    cancelled: source.cancelled === true,
    finished: source.finished === true,
    progress: typeof source.progress === "string" ? source.progress : "",
    logs: Array.isArray(source.logs)
      ? source.logs.filter((item): item is string => typeof item === "string")
      : [],
  };
}

function normalizeRegisterTaskListResult(value: unknown): RegisterTaskListResult {
  const source = asRecord(value) ?? {};
  const tasks = Array.isArray(source.tasks) ? source.tasks : [];
  return {
    total:
      typeof source.total === "number" && Number.isFinite(source.total) ? source.total : tasks.length,
    tasks: tasks.map(normalizeRegisterTaskSnapshot).filter((task) => task.taskUuid),
  };
}

function normalizeRegisterStats(value: unknown): RegisterStats {
  const source = asRecord(value) ?? {};
  const rawByStatus = asRecord(source.byStatus ?? source.by_status) ?? {};
  const byStatus = Object.entries(rawByStatus).reduce<Record<string, number>>((result, [key, rawValue]) => {
    if (typeof rawValue === "number" && Number.isFinite(rawValue)) {
      result[key] = rawValue;
    }
    return result;
  }, {});
  return {
    byStatus,
    todayCount:
      typeof source.todayCount === "number" && Number.isFinite(source.todayCount)
        ? source.todayCount
        : typeof source.today_count === "number" && Number.isFinite(source.today_count)
          ? source.today_count
          : 0,
  };
}

function normalizeRegisterOutlookAccount(value: unknown): RegisterOutlookAccount {
  const source = asRecord(value) ?? {};
  return {
    id: typeof source.id === "number" && Number.isFinite(source.id) ? source.id : 0,
    email: typeof source.email === "string" ? source.email : "",
    name: typeof source.name === "string" ? source.name : "",
    hasOauth: source.hasOauth === true || source.has_oauth === true,
    isRegistered: source.isRegistered === true || source.is_registered === true,
    registeredAccountId:
      typeof source.registeredAccountId === "number" && Number.isFinite(source.registeredAccountId)
        ? source.registeredAccountId
        : typeof source.registered_account_id === "number" &&
            Number.isFinite(source.registered_account_id)
          ? source.registered_account_id
          : null,
  };
}

function normalizeRegisterOutlookAccountsResult(
  value: unknown
): RegisterOutlookAccountsResult {
  const source = asRecord(value) ?? {};
  const accounts = Array.isArray(source.accounts) ? source.accounts : [];
  return {
    total: typeof source.total === "number" && Number.isFinite(source.total) ? source.total : 0,
    registeredCount:
      typeof source.registeredCount === "number" && Number.isFinite(source.registeredCount)
        ? source.registeredCount
        : typeof source.registered_count === "number" &&
            Number.isFinite(source.registered_count)
          ? source.registered_count
          : 0,
    unregisteredCount:
      typeof source.unregisteredCount === "number" && Number.isFinite(source.unregisteredCount)
        ? source.unregisteredCount
        : typeof source.unregistered_count === "number" &&
            Number.isFinite(source.unregistered_count)
          ? source.unregistered_count
          : 0,
    accounts: accounts.map(normalizeRegisterOutlookAccount).filter((item) => item.id > 0),
  };
}

function normalizeRegisterOutlookBatchStartResult(
  value: unknown
): RegisterOutlookBatchStartResult {
  const source = asRecord(value) ?? {};
  const serviceIds = Array.isArray(source.serviceIds)
    ? source.serviceIds
    : Array.isArray(source.service_ids)
      ? source.service_ids
      : [];
  return {
    batchId: typeof source.batchId === "string"
      ? source.batchId
      : typeof source.batch_id === "string"
        ? source.batch_id
        : "",
    total: typeof source.total === "number" && Number.isFinite(source.total) ? source.total : 0,
    skipped:
      typeof source.skipped === "number" && Number.isFinite(source.skipped) ? source.skipped : 0,
    toRegister:
      typeof source.toRegister === "number" && Number.isFinite(source.toRegister)
        ? source.toRegister
        : typeof source.to_register === "number" && Number.isFinite(source.to_register)
          ? source.to_register
          : 0,
    serviceIds: serviceIds.filter(
      (item): item is number => typeof item === "number" && Number.isFinite(item)
    ),
  };
}

function normalizeRegisterOutlookBatchSnapshot(
  value: unknown
): RegisterOutlookBatchSnapshot {
  const source = asRecord(value) ?? {};
  return {
    ...normalizeRegisterBatchSnapshot(value),
    skipped:
      typeof source.skipped === "number" && Number.isFinite(source.skipped) ? source.skipped : 0,
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

function normalizeRegisterEmailServiceStats(value: unknown): RegisterEmailServiceStats {
  const source = asRecord(value) ?? {};
  return {
    outlookCount:
      typeof source.outlookCount === "number" && Number.isFinite(source.outlookCount)
        ? source.outlookCount
        : typeof source.outlook_count === "number" && Number.isFinite(source.outlook_count)
          ? source.outlook_count
          : 0,
    customCount:
      typeof source.customCount === "number" && Number.isFinite(source.customCount)
        ? source.customCount
        : typeof source.custom_count === "number" && Number.isFinite(source.custom_count)
          ? source.custom_count
          : 0,
    tempMailCount:
      typeof source.tempMailCount === "number" && Number.isFinite(source.tempMailCount)
        ? source.tempMailCount
        : typeof source.temp_mail_count === "number" && Number.isFinite(source.temp_mail_count)
          ? source.temp_mail_count
          : 0,
    tempmailAvailable:
      source.tempmailAvailable === true || source.tempmail_available === true,
    enabledCount:
      typeof source.enabledCount === "number" && Number.isFinite(source.enabledCount)
        ? source.enabledCount
        : typeof source.enabled_count === "number" && Number.isFinite(source.enabled_count)
          ? source.enabled_count
          : 0,
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

function normalizeRegisterEmailServiceBatchDeleteResult(
  value: unknown
): RegisterEmailServiceBatchDeleteResult {
  const source = asRecord(value) ?? {};
  return {
    success: source.success === true,
    deleted:
      typeof source.deleted === "number" && Number.isFinite(source.deleted)
        ? source.deleted
        : 0,
    message: typeof source.message === "string" ? source.message : "",
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
  updateManyStatus: (accountIds: string[], status: "active" | "disabled") =>
    invoke<AccountBulkStatusUpdateResult>(
      "service_account_update_many",
      withAddr({ accountIds, status })
    ),
  import: (contents: string[]) =>
    invoke<AccountImportResult>("service_account_import", withAddr({ contents })),
  async getRegisterAvailableServices(): Promise<RegisterAvailableServicesResult> {
    const result = await invoke<unknown>(
      "service_account_register_available_services",
      withAddr()
    );
    return normalizeRegisterAvailableServices(result);
  },
  async getRegisterOutlookAccounts(): Promise<RegisterOutlookAccountsResult> {
    const result = await invoke<unknown>(
      "service_account_register_outlook_accounts",
      withAddr()
    );
    return normalizeRegisterOutlookAccountsResult(result);
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
  async getRegisterEmailServiceStats(): Promise<RegisterEmailServiceStats> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_stats",
      withAddr()
    );
    return normalizeRegisterEmailServiceStats(result);
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
  async batchDeleteRegisterOutlookEmailServices(
    serviceIds: number[]
  ): Promise<RegisterEmailServiceBatchDeleteResult> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_outlook_batch_delete",
      withAddr({ serviceIds })
    );
    return normalizeRegisterEmailServiceBatchDeleteResult(result);
  },
  reorderRegisterEmailServices: (params: RegisterEmailServiceReorderPayload) =>
    invoke(
      "service_account_register_email_services_reorder",
      withAddr({ serviceIds: params.serviceIds })
    ),
  async testRegisterTempmail(apiUrl?: string | null): Promise<RegisterEmailServiceTestResult> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_test_tempmail",
      withAddr({ apiUrl: apiUrl ?? null })
    );
    return normalizeRegisterEmailServiceTestResult(result);
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
  async startRegisterBatch(
    params: RegisterBatchStartPayload
  ): Promise<RegisterBatchStartResult> {
    const result = await invoke<unknown>(
      "service_account_register_batch_start",
      withAddr({
        emailServiceType: params.emailServiceType,
        emailServiceId: params.emailServiceId ?? null,
        proxy: params.proxy ?? null,
        count: params.count,
        intervalMin: params.intervalMin,
        intervalMax: params.intervalMax,
        concurrency: params.concurrency,
        mode: params.mode,
      })
    );
    return normalizeRegisterBatchStartResult(result);
  },
  async getRegisterBatch(batchId: string): Promise<RegisterBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_batch_read",
      withAddr({ batchId })
    );
    return normalizeRegisterBatchSnapshot(result);
  },
  cancelRegisterBatch: (batchId: string) =>
    invoke("service_account_register_batch_cancel", withAddr({ batchId })),
  async listRegisterTasks(params?: RegisterTaskListPayload): Promise<RegisterTaskListResult> {
    const result = await invoke<unknown>(
      "service_account_register_tasks_list",
      withAddr({
        page: params?.page ?? 1,
        pageSize: params?.pageSize ?? 20,
        status: params?.status ?? null,
      })
    );
    return normalizeRegisterTaskListResult(result);
  },
  async getRegisterStats(): Promise<RegisterStats> {
    const result = await invoke<unknown>(
      "service_account_register_stats",
      withAddr()
    );
    return normalizeRegisterStats(result);
  },
  cancelRegisterTask: (taskUuid: string) =>
    invoke("service_account_register_task_cancel", withAddr({ taskUuid })),
  deleteRegisterTask: (taskUuid: string) =>
    invoke("service_account_register_task_delete", withAddr({ taskUuid })),
  async startRegisterOutlookBatch(
    params: RegisterOutlookBatchStartPayload
  ): Promise<RegisterOutlookBatchStartResult> {
    const result = await invoke<unknown>(
      "service_account_register_outlook_batch_start",
      withAddr({
        serviceIds: params.serviceIds,
        skipRegistered: params.skipRegistered ?? true,
        proxy: params.proxy ?? null,
        intervalMin: params.intervalMin,
        intervalMax: params.intervalMax,
        concurrency: params.concurrency,
        mode: params.mode,
      })
    );
    return normalizeRegisterOutlookBatchStartResult(result);
  },
  async getRegisterOutlookBatch(batchId: string): Promise<RegisterOutlookBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_outlook_batch_read",
      withAddr({ batchId })
    );
    return normalizeRegisterOutlookBatchSnapshot(result);
  },
  cancelRegisterOutlookBatch: (batchId: string) =>
    invoke("service_account_register_outlook_batch_cancel", withAddr({ batchId })),
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
  async importRegisterAccountByEmail(email: string): Promise<RegisterImportResult> {
    const result = await invoke<unknown>(
      "service_account_register_import_by_email",
      withAddr({ email })
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
