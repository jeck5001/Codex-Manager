import { invoke, withAddr } from "./transport";
import {
  normalizeAccountList,
  normalizeApiKeyCreateResult,
  normalizeApiKeyAllowedModels,
  normalizeApiKeyModelFallback,
  normalizeApiKeyResponseCache,
  normalizeApiKeyList,
  normalizeApiKeyRateLimit,
  normalizeApiKeyUsageStats,
  normalizeLoginStartResult,
  normalizeModelOptions,
  normalizeUsageAggregateSummary,
  normalizeUsageList,
  normalizeUsageSnapshot,
} from "./normalize";
import {
  AccountAuthRecoveryResult,
  AccountBulkStatusUpdateResult,
  AccountListResult,
  AccountOfficialPromoLinkResult,
  AccountPaymentLinkResult,
  AccountSubscriptionCheckManyResult,
  AccountSubscriptionCheckResult,
  AccountTeamManagerUploadManyResult,
  AccountTeamManagerUploadResult,
  AccountUsage,
  ApiKey,
  ApiKeyAllowedModelsConfig,
  ApiKeyCreateResult,
  ApiKeyModelFallback,
  ApiKeyResponseCacheConfig,
  ApiKeyRateLimit,
  ApiKeyUsageStat,
  ChatgptAuthTokensRefreshResult,
  CurrentAccessTokenAccountReadResult,
  LoginStatusResult,
  LoginStartResult,
  ModelOption,
  RegisterAvailableServicesResult,
  RegisterBatchSnapshot,
  RegisterBatchStartResult,
  RegisterBrowserbaseConfig,
  RegisterBrowserbaseConfigListResult,
  RegisterEmailServiceBatchDeleteResult,
  RegisterEmailService,
  RegisterEmailServiceField,
  RegisterEmailServiceListResult,
  RegisterEmailServiceStats,
  RegisterEmailServiceTestResult,
  RegisterEmailServiceType,
  RegisterEmailServiceTypeCatalog,
  RegisterHotmailArtifact,
  RegisterHotmailBatchSnapshot,
  RegisterHotmailLocalHandoff,
  RegisterHotmailLocalHandoffCookie,
  RegisterHotmailLocalHandoffOrigin,
  RegisterImportResult,
  RegisterOutlookAccount,
  RegisterOutlookAccountsResult,
  RegisterOutlookBatchSnapshot,
  RegisterOutlookBatchStartResult,
  RegisterOutlookBatchImportResult,
  RegisterServiceGroup,
  RegisterStats,
  RegisterTempMailCloudflareSettings,
  RegisterTempMailDomainConfig,
  RegisterTaskBatchDeleteResult,
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

interface AccountExportPayload {
  accountIds?: string[];
}

interface DeleteUnavailableFreeResult {
  deleted?: number;
}

interface DeleteBannedAccountsResult {
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
  expiresAt?: number | null;
}

interface RegisterStartPayload {
  emailServiceType: string;
  emailServiceId?: number | null;
  emailServiceConfig?: Record<string, unknown> | null;
  registerMode?: string | null;
  browserbaseConfigId?: number | null;
  proxy?: string | null;
  autoCreateTempMailService?: boolean;
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

interface RegisterHotmailBatchStartPayload {
  count: number;
  concurrency: number;
  intervalMin: number;
  intervalMax: number;
  proxy?: string | null;
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

interface RegisterTempMailCloudflareSettingsPayload {
  cloudflareApiToken?: string | null;
  cloudflareApiEmail?: string | null;
  cloudflareGlobalApiKey?: string | null;
  cloudflareAccountId?: string | null;
  cloudflareZoneId?: string | null;
  cloudflareWorkerName?: string | null;
  tempMailBaseUrl?: string | null;
  tempMailAdminPassword?: string | null;
  domainConfigs?: RegisterTempMailDomainConfig[] | null;
  tempMailDomainBase?: string | null;
  tempMailSubdomainMode?: string | null;
  tempMailSubdomainLength?: number | null;
  tempMailSubdomainPrefix?: string | null;
  tempMailSyncCloudflareEnabled?: boolean;
  tempMailRequireCloudflareSync?: boolean;
}

function normalizeRegisterTempMailDomainConfig(value: unknown): RegisterTempMailDomainConfig {
  const source = asRecord(value) ?? {};
  return {
    id: typeof source.id === "string" ? source.id : "",
    name: typeof source.name === "string" ? source.name : "",
    zoneId:
      typeof source.zoneId === "string"
        ? source.zoneId
        : typeof source.zone_id === "string"
          ? source.zone_id
          : "",
    domainBase:
      typeof source.domainBase === "string"
        ? source.domainBase
        : typeof source.domain_base === "string"
          ? source.domain_base
          : "",
    subdomainMode:
      typeof source.subdomainMode === "string"
        ? source.subdomainMode
        : typeof source.subdomain_mode === "string"
          ? source.subdomain_mode
          : "random",
    subdomainLength:
      typeof source.subdomainLength === "number" && Number.isFinite(source.subdomainLength)
        ? source.subdomainLength
        : typeof source.subdomain_length === "number" && Number.isFinite(source.subdomain_length)
          ? source.subdomain_length
          : 6,
    subdomainPrefix:
      typeof source.subdomainPrefix === "string"
        ? source.subdomainPrefix
        : typeof source.subdomain_prefix === "string"
          ? source.subdomain_prefix
          : "",
    syncCloudflareEnabled:
      source.syncCloudflareEnabled === true || source.sync_cloudflare_enabled === true,
    requireCloudflareSync:
      source.requireCloudflareSync === true || source.require_cloudflare_sync === true,
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function normalizeRegisterServiceGroup(value: unknown): RegisterServiceGroup {
  const source = asRecord(value) ?? {};
  const services = Array.isArray(source.services) ? source.services : [];
  const rawDomainConfigs = Array.isArray(source.domainConfigs)
    ? source.domainConfigs
    : Array.isArray(source.domain_configs)
      ? source.domain_configs
      : [];
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
    domainConfigs: rawDomainConfigs
      .map(normalizeRegisterTempMailDomainConfig)
      .filter((item) => item.id),
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
  const canImport =
    source.canImport === true ||
    source.can_import === true ||
    (typeof emailFromResult === "string" &&
      emailFromResult.trim().length > 0 &&
      String(source.status || "").trim().toLowerCase() === "completed");
  const importedAccountId = typeof source.importedAccountId === "string"
    ? source.importedAccountId
    : typeof source.imported_account_id === "string"
      ? source.imported_account_id
      : null;
  const isImported =
    source.isImported === true ||
    source.is_imported === true ||
    (typeof importedAccountId === "string" && importedAccountId.trim().length > 0);
  return {
    taskUuid: typeof source.taskUuid === "string"
      ? source.taskUuid
      : typeof source.task_uuid === "string"
        ? source.task_uuid
        : "",
    status: typeof source.status === "string" ? source.status : "",
    registerMode: typeof source.registerMode === "string"
      ? source.registerMode
      : typeof source.register_mode === "string"
        ? source.register_mode
        : "standard",
    emailServiceId: typeof source.emailServiceId === "number" && Number.isFinite(source.emailServiceId)
      ? source.emailServiceId
      : typeof source.email_service_id === "number" && Number.isFinite(source.email_service_id)
        ? source.email_service_id
        : null,
    browserbaseConfigId:
      typeof source.browserbaseConfigId === "number" && Number.isFinite(source.browserbaseConfigId)
        ? source.browserbaseConfigId
        : typeof source.browserbase_config_id === "number" &&
            Number.isFinite(source.browserbase_config_id)
          ? source.browserbase_config_id
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
    failureCode: typeof source.failureCode === "string"
      ? source.failureCode
      : typeof source.failure_code === "string"
        ? source.failure_code
        : "",
    failureLabel: typeof source.failureLabel === "string"
      ? source.failureLabel
      : typeof source.failure_label === "string"
        ? source.failure_label
        : "",
    email: typeof source.email === "string" && source.email
      ? source.email
      : emailFromResult,
    canImport,
    importedAccountId,
    isImported,
    requiresManualImport:
      source.requiresManualImport === true ||
      source.requires_manual_import === true ||
      (canImport && !isImported),
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

function normalizeRegisterTaskBatchDeleteResult(
  value: unknown,
): RegisterTaskBatchDeleteResult {
  const source = asRecord(value) ?? {};
  const rawErrors = Array.isArray(source.errors) ? source.errors : [];
  return {
    success: source.success === true,
    deletedCount:
      typeof source.deletedCount === "number" && Number.isFinite(source.deletedCount)
        ? source.deletedCount
        : typeof source.deleted_count === "number" && Number.isFinite(source.deleted_count)
          ? source.deleted_count
          : 0,
    failedCount:
      typeof source.failedCount === "number" && Number.isFinite(source.failedCount)
        ? source.failedCount
        : typeof source.failed_count === "number" && Number.isFinite(source.failed_count)
          ? source.failed_count
          : 0,
    errors: rawErrors
      .map((item) => {
        const error = asRecord(item) ?? {};
        const taskUuid =
          typeof error.taskUuid === "string"
            ? error.taskUuid
            : typeof error.task_uuid === "string"
              ? error.task_uuid
              : "";
        return {
          taskUuid,
          error: typeof error.error === "string" ? error.error : "",
        };
      })
      .filter((item) => item.taskUuid || item.error),
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

function normalizeRegisterHotmailArtifact(value: unknown): RegisterHotmailArtifact {
  const source = asRecord(value) ?? {};
  return {
    filename:
      typeof source.filename === "string"
        ? source.filename
        : typeof source.fileName === "string"
          ? source.fileName
          : typeof source.file_name === "string"
            ? source.file_name
            : "",
    path: typeof source.path === "string" ? source.path : "",
    size:
      typeof source.size === "number" && Number.isFinite(source.size)
        ? source.size
        : null,
  };
}

function normalizeRegisterHotmailArtifacts(value: unknown): RegisterHotmailArtifact[] {
  const source = asRecord(value);
  const rawItems = Array.isArray(source?.artifacts)
    ? source?.artifacts
    : Array.isArray(value)
      ? value
      : [];
  return rawItems
    .map(normalizeRegisterHotmailArtifact)
    .filter((item) => item.filename || item.path);
}

function normalizeRegisterHotmailLocalHandoffCookie(
  value: unknown
): RegisterHotmailLocalHandoffCookie {
  const source = asRecord(value) ?? {};
  return {
    name: typeof source.name === "string" ? source.name : "",
    value: typeof source.value === "string" ? source.value : "",
    domain: typeof source.domain === "string" ? source.domain : "",
    path: typeof source.path === "string" ? source.path : "/",
    expires:
      typeof source.expires === "number" && Number.isFinite(source.expires) ? source.expires : null,
    httpOnly:
      source.httpOnly === true
        ? true
        : source.http_only === true,
    secure: source.secure === true,
    sameSite:
      typeof source.sameSite === "string"
        ? source.sameSite
        : typeof source.same_site === "string"
          ? source.same_site
          : "",
  };
}

function normalizeRegisterHotmailLocalHandoffOrigin(
  value: unknown
): RegisterHotmailLocalHandoffOrigin {
  const source = asRecord(value) ?? {};
  const rawEntries = Array.isArray(source.localStorage)
    ? source.localStorage
    : Array.isArray(source.local_storage)
      ? source.local_storage
      : [];
  return {
    origin: typeof source.origin === "string" ? source.origin : "",
    localStorage: rawEntries
      .map((item) => {
        const entry = asRecord(item) ?? {};
        return {
          name: typeof entry.name === "string" ? entry.name : "",
          value: typeof entry.value === "string" ? entry.value : "",
        };
      })
      .filter((entry) => entry.name),
  };
}

function normalizeRegisterHotmailLocalHandoff(
  value: unknown
): RegisterHotmailLocalHandoff | null {
  const source = asRecord(value);
  if (!source) {
    return null;
  }
  const rawCookies = Array.isArray(source.cookies) ? source.cookies : [];
  const rawOrigins = Array.isArray(source.origins) ? source.origins : [];
  return {
    handoffId:
      typeof source.handoffId === "string"
        ? source.handoffId
        : typeof source.handoff_id === "string"
          ? source.handoff_id
          : "",
    url: typeof source.url === "string" ? source.url : "",
    title: typeof source.title === "string" ? source.title : "",
    userAgent:
      typeof source.userAgent === "string"
        ? source.userAgent
        : typeof source.user_agent === "string"
          ? source.user_agent
          : "",
    proxyUrl:
      typeof source.proxyUrl === "string"
        ? source.proxyUrl
        : typeof source.proxy_url === "string"
          ? source.proxy_url
          : "",
    state: typeof source.state === "string" ? source.state : "",
    cookies: rawCookies
      .map(normalizeRegisterHotmailLocalHandoffCookie)
      .filter((cookie) => cookie.name),
    origins: rawOrigins
      .map(normalizeRegisterHotmailLocalHandoffOrigin)
      .filter((origin) => origin.origin || origin.localStorage.length > 0),
  };
}

function normalizeRegisterHotmailBatchSnapshot(
  value: unknown
): RegisterHotmailBatchSnapshot {
  const source = asRecord(value) ?? {};
  return {
    batchId:
      typeof source.batchId === "string"
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
    status:
      typeof source.status === "string"
        ? source.status
        : typeof source.batchStatus === "string"
          ? source.batchStatus
          : typeof source.batch_status === "string"
            ? source.batch_status
            : "",
    actionRequiredReason:
      typeof source.actionRequiredReason === "string"
        ? source.actionRequiredReason
        : typeof source.action_required_reason === "string"
          ? source.action_required_reason
          : "",
    handoffId:
      typeof source.handoffId === "string"
        ? source.handoffId
        : typeof source.handoff_id === "string"
          ? source.handoff_id
          : "",
    handoffUrl:
      typeof source.handoffUrl === "string"
        ? source.handoffUrl
        : typeof source.handoff_url === "string"
          ? source.handoff_url
          : "",
    handoffTitle:
      typeof source.handoffTitle === "string"
        ? source.handoffTitle
        : typeof source.handoff_title === "string"
          ? source.handoff_title
          : "",
    handoffInstructions:
      typeof source.handoffInstructions === "string"
        ? source.handoffInstructions
        : typeof source.handoff_instructions === "string"
          ? source.handoff_instructions
          : "",
    localHandoff: normalizeRegisterHotmailLocalHandoff(
      source.localHandoff ?? source.local_handoff
    ),
    cancelled: source.cancelled === true,
    finished: source.finished === true,
    logs: Array.isArray(source.logs)
      ? source.logs
          .filter((item): item is string => typeof item === "string")
          .map((item) => item.trim())
          .filter(Boolean)
      : [],
    artifacts: normalizeRegisterHotmailArtifacts(source),
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
    description: typeof source.description === "string" ? source.description : "",
    readOnly: source.readOnly === true || source.read_only === true,
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

function normalizeRegisterBrowserbaseConfig(value: unknown): RegisterBrowserbaseConfig {
  const source = asRecord(value) ?? {};
  return {
    id: typeof source.id === "number" && Number.isFinite(source.id) ? source.id : 0,
    name: typeof source.name === "string" ? source.name : "",
    enabled: source.enabled === true,
    priority: typeof source.priority === "number" && Number.isFinite(source.priority) ? source.priority : 0,
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

function normalizeRegisterBrowserbaseConfigList(
  value: unknown
): RegisterBrowserbaseConfigListResult {
  const source = asRecord(value) ?? {};
  const configs = Array.isArray(source.configs) ? source.configs : [];
  return {
    total: typeof source.total === "number" && Number.isFinite(source.total) ? source.total : configs.length,
    configs: configs
      .map(normalizeRegisterBrowserbaseConfig)
      .filter((item) => item.id > 0),
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

function normalizeRegisterTempMailCloudflareSettings(
  value: unknown
): RegisterTempMailCloudflareSettings {
  const source = asRecord(value) ?? {};
  const rawDomainConfigs = Array.isArray(source.domainConfigs)
    ? source.domainConfigs
    : Array.isArray(source.temp_mail_domain_configs)
      ? source.temp_mail_domain_configs
      : [];
  return {
    hasApiToken: source.hasApiToken === true || source.has_api_token === true,
    cloudflareApiEmail:
      typeof source.cloudflareApiEmail === "string"
        ? source.cloudflareApiEmail
        : typeof source.cloudflare_api_email === "string"
          ? source.cloudflare_api_email
          : "",
    hasGlobalApiKey:
      source.hasGlobalApiKey === true || source.has_global_api_key === true,
    cloudflareAccountId:
      typeof source.cloudflareAccountId === "string"
        ? source.cloudflareAccountId
        : typeof source.cloudflare_account_id === "string"
          ? source.cloudflare_account_id
          : "",
    cloudflareZoneId:
      typeof source.cloudflareZoneId === "string"
        ? source.cloudflareZoneId
        : typeof source.cloudflare_zone_id === "string"
          ? source.cloudflare_zone_id
          : "",
    cloudflareWorkerName:
      typeof source.cloudflareWorkerName === "string"
        ? source.cloudflareWorkerName
        : typeof source.cloudflare_worker_name === "string"
          ? source.cloudflare_worker_name
          : "",
    tempMailBaseUrl:
      typeof source.tempMailBaseUrl === "string"
        ? source.tempMailBaseUrl
        : typeof source.temp_mail_base_url === "string"
          ? source.temp_mail_base_url
          : "",
    hasTempMailAdminPassword:
      source.hasTempMailAdminPassword === true ||
      source.has_temp_mail_admin_password === true,
    domainConfigs: rawDomainConfigs
      .map(normalizeRegisterTempMailDomainConfig)
      .filter((item) => item.id),
    tempMailDomainBase:
      typeof source.tempMailDomainBase === "string"
        ? source.tempMailDomainBase
        : typeof source.temp_mail_domain_base === "string"
          ? source.temp_mail_domain_base
          : "",
    tempMailSubdomainMode:
      typeof source.tempMailSubdomainMode === "string"
        ? source.tempMailSubdomainMode
        : typeof source.temp_mail_subdomain_mode === "string"
          ? source.temp_mail_subdomain_mode
          : "random",
    tempMailSubdomainLength:
      typeof source.tempMailSubdomainLength === "number" &&
      Number.isFinite(source.tempMailSubdomainLength)
        ? source.tempMailSubdomainLength
        : typeof source.temp_mail_subdomain_length === "number" &&
            Number.isFinite(source.temp_mail_subdomain_length)
          ? source.temp_mail_subdomain_length
          : 6,
    tempMailSubdomainPrefix:
      typeof source.tempMailSubdomainPrefix === "string"
        ? source.tempMailSubdomainPrefix
        : typeof source.temp_mail_subdomain_prefix === "string"
          ? source.temp_mail_subdomain_prefix
          : "",
    tempMailSyncCloudflareEnabled:
      source.tempMailSyncCloudflareEnabled === true ||
      source.temp_mail_sync_cloudflare_enabled === true ||
      source.temp_mail_sync_cloudflare_enabled === undefined,
    tempMailRequireCloudflareSync:
      source.tempMailRequireCloudflareSync === true ||
      source.temp_mail_require_cloudflare_sync === true ||
      source.temp_mail_require_cloudflare_sync === undefined,
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
  deleteBanned: () =>
    invoke<DeleteBannedAccountsResult>("service_account_delete_banned", withAddr()),
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
  updateManyTags: (accountIds: string[], tags: string[] | string | null) =>
    invoke<AccountBulkStatusUpdateResult>(
      "service_account_update_many_tags",
      withAddr({
        accountIds,
        tags: Array.isArray(tags)
          ? tags.map((item) => String(item || "").trim()).filter(Boolean).join(",")
          : tags || null,
      })
    ),
  generatePaymentLink: (payload: {
    accountId: string;
    planType: "plus" | "team";
    workspaceName?: string | null;
    priceInterval?: "month" | "year" | null;
    seatQuantity?: number | null;
    country?: string | null;
    proxy?: string | null;
  }) =>
    invoke<AccountPaymentLinkResult>(
      "service_account_payment_generate_link",
      withAddr(payload)
    ),
  checkSubscription: (accountId: string, proxy?: string | null) =>
    invoke<AccountSubscriptionCheckResult>(
      "service_account_subscription_check",
      withAddr({ accountId, proxy: proxy ?? null })
    ),
  checkSubscriptions: (accountIds: string[], proxy?: string | null) =>
    invoke<AccountSubscriptionCheckManyResult>(
      "service_account_subscription_check_many",
      withAddr({ accountIds, proxy: proxy ?? null })
    ),
  markSubscription: (accountId: string, planType: "free" | "plus" | "team") =>
    invoke<AccountSubscriptionCheckResult>(
      "service_account_subscription_mark",
      withAddr({ accountId, planType })
    ),
  setOfficialPromoLink: (accountId: string, link?: string | null) =>
    invoke<AccountOfficialPromoLinkResult>(
      "service_account_payment_official_promo_link_set",
      withAddr({ accountId, link: link ?? null })
    ),
  uploadToTeamManager: (accountId: string) =>
    invoke<AccountTeamManagerUploadResult>(
      "service_account_team_manager_upload",
      withAddr({ accountId })
    ),
  uploadManyToTeamManager: (accountIds: string[]) =>
    invoke<AccountTeamManagerUploadManyResult>(
      "service_account_team_manager_upload_many",
      withAddr({ accountIds })
    ),
  testTeamManager: (apiUrl?: string | null, apiKey?: string | null) =>
    invoke<{ success: boolean; message: string }>(
      "service_account_team_manager_test",
      withAddr({ apiUrl: apiUrl ?? null, apiKey: apiKey ?? null })
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
  async listRegisterBrowserbaseConfigs(): Promise<RegisterBrowserbaseConfigListResult> {
    const result = await invoke<unknown>(
      "service_account_register_browserbase_configs_list",
      withAddr()
    );
    return normalizeRegisterBrowserbaseConfigList(result);
  },
  async readRegisterBrowserbaseConfigFull(configId: number): Promise<RegisterBrowserbaseConfig> {
    const result = await invoke<unknown>(
      "service_account_register_browserbase_configs_read_full",
      withAddr({ configId })
    );
    return normalizeRegisterBrowserbaseConfig(result);
  },
  async createRegisterBrowserbaseConfig(params: {
    name: string;
    enabled?: boolean;
    priority?: number;
    config?: Record<string, unknown>;
  }): Promise<RegisterBrowserbaseConfig> {
    const result = await invoke<unknown>(
      "service_account_register_browserbase_configs_create",
      withAddr({
        name: params.name,
        enabled: params.enabled ?? true,
        priority: params.priority ?? 0,
        config: params.config ?? {},
      })
    );
    return normalizeRegisterBrowserbaseConfig(result);
  },
  async updateRegisterBrowserbaseConfig(params: {
    configId: number;
    name?: string | null;
    enabled?: boolean;
    priority?: number | null;
    config?: Record<string, unknown>;
  }): Promise<RegisterBrowserbaseConfig> {
    const result = await invoke<unknown>(
      "service_account_register_browserbase_configs_update",
      withAddr({
        configId: params.configId,
        name: params.name ?? null,
        enabled: params.enabled,
        priority: params.priority ?? null,
        config: params.config ?? {},
      })
    );
    return normalizeRegisterBrowserbaseConfig(result);
  },
  deleteRegisterBrowserbaseConfig: (configId: number) =>
    invoke("service_account_register_browserbase_configs_delete", withAddr({ configId })),
  async getRegisterEmailServiceStats(): Promise<RegisterEmailServiceStats> {
    const result = await invoke<unknown>(
      "service_account_register_email_services_stats",
      withAddr()
    );
    return normalizeRegisterEmailServiceStats(result);
  },
  async getRegisterTempMailCloudflareSettings(): Promise<RegisterTempMailCloudflareSettings> {
    const result = await invoke<unknown>(
      "service_account_register_temp_mail_cloudflare_settings_get",
      withAddr()
    );
    return normalizeRegisterTempMailCloudflareSettings(result);
  },
  async setRegisterTempMailCloudflareSettings(
    params: RegisterTempMailCloudflareSettingsPayload
  ): Promise<RegisterTempMailCloudflareSettings> {
    const payload: Record<string, unknown> = {
      cloudflare_api_email: params.cloudflareApiEmail ?? "",
      cloudflare_account_id: params.cloudflareAccountId ?? "",
      cloudflare_zone_id: params.cloudflareZoneId ?? "",
      cloudflare_worker_name: params.cloudflareWorkerName ?? "",
      temp_mail_base_url: params.tempMailBaseUrl ?? "",
      temp_mail_domain_configs: (params.domainConfigs ?? []).map((item) => ({
        id: item.id,
        name: item.name,
        zone_id: item.zoneId,
        domain_base: item.domainBase,
        subdomain_mode: item.subdomainMode,
        subdomain_length: item.subdomainLength,
        subdomain_prefix: item.subdomainPrefix,
        sync_cloudflare_enabled: item.syncCloudflareEnabled,
        require_cloudflare_sync: item.requireCloudflareSync,
      })),
    };
    if (typeof params.cloudflareApiToken === "string" && params.cloudflareApiToken.trim()) {
      payload.cloudflare_api_token = params.cloudflareApiToken.trim();
    }
    if (
      typeof params.cloudflareGlobalApiKey === "string" &&
      params.cloudflareGlobalApiKey.trim()
    ) {
      payload.cloudflare_global_api_key = params.cloudflareGlobalApiKey.trim();
    }
    if (
      typeof params.tempMailAdminPassword === "string" &&
      params.tempMailAdminPassword.trim()
    ) {
      payload.temp_mail_admin_password = params.tempMailAdminPassword.trim();
    }
    await invoke<unknown>(
      "service_account_register_temp_mail_cloudflare_settings_set",
      withAddr(payload)
    );
    return this.getRegisterTempMailCloudflareSettings();
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
        emailServiceConfig: params.emailServiceConfig ?? null,
        registerMode: params.registerMode ?? "standard",
        browserbaseConfigId: params.browserbaseConfigId ?? null,
        proxy: params.proxy ?? null,
        autoCreateTempMailService: params.autoCreateTempMailService === true,
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
        emailServiceConfig: params.emailServiceConfig ?? null,
        registerMode: params.registerMode ?? "standard",
        browserbaseConfigId: params.browserbaseConfigId ?? null,
        proxy: params.proxy ?? null,
        autoCreateTempMailService: params.autoCreateTempMailService === true,
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
  async startRegisterHotmailBatch(
    params: RegisterHotmailBatchStartPayload
  ): Promise<RegisterHotmailBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_hotmail_batch_start",
      withAddr({
        count: params.count,
        concurrency: params.concurrency,
        intervalMin: params.intervalMin,
        intervalMax: params.intervalMax,
        proxy: params.proxy ?? null,
      })
    );
    return normalizeRegisterHotmailBatchSnapshot(result);
  },
  async getRegisterHotmailBatch(batchId: string): Promise<RegisterHotmailBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_hotmail_batch_read",
      withAddr({ batchId })
    );
    return normalizeRegisterHotmailBatchSnapshot(result);
  },
  cancelRegisterHotmailBatch: (batchId: string) =>
    invoke("service_account_register_hotmail_batch_cancel", withAddr({ batchId })),
  async continueRegisterHotmailBatch(batchId: string): Promise<RegisterHotmailBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_hotmail_batch_continue",
      withAddr({ batchId }),
    );
    return normalizeRegisterHotmailBatchSnapshot(result);
  },
  async abandonRegisterHotmailBatch(batchId: string): Promise<RegisterHotmailBatchSnapshot> {
    const result = await invoke<unknown>(
      "service_account_register_hotmail_batch_abandon",
      withAddr({ batchId }),
    );
    return normalizeRegisterHotmailBatchSnapshot(result);
  },
  async getRegisterHotmailBatchArtifacts(batchId: string): Promise<RegisterHotmailArtifact[]> {
    const result = await invoke<unknown>(
      "service_account_register_hotmail_batch_artifacts",
      withAddr({ batchId })
    );
    return normalizeRegisterHotmailArtifacts(result);
  },
  cancelRegisterTask: (taskUuid: string) =>
    invoke("service_account_register_task_cancel", withAddr({ taskUuid })),
  retryRegisterTask: (taskUuid: string, strategy?: string | null) =>
    invoke(
      "service_account_register_task_retry",
      withAddr({ taskUuid, strategy: strategy || null }),
    ),
  deleteRegisterTask: (taskUuid: string) =>
    invoke("service_account_register_task_delete", withAddr({ taskUuid })),
  async deleteRegisterTasks(taskUuids: string[]): Promise<RegisterTaskBatchDeleteResult> {
    const result = await invoke<unknown>(
      "service_account_register_tasks_delete_many",
      withAddr({ taskUuids }),
    );
    return normalizeRegisterTaskBatchDeleteResult(result);
  },
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
  export: (accountIds?: string[]) =>
    invoke<AccountExportResult>(
      "service_account_export_by_account_files",
      withAddr({
        accountIds: Array.isArray(accountIds)
          ? accountIds
              .map((item) => String(item || "").trim())
              .filter(Boolean)
          : [],
      } satisfies AccountExportPayload)
    ),

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
  async recoverAccountAuth(
    accountId: string,
    openBrowser?: boolean
  ): Promise<AccountAuthRecoveryResult> {
    const result = await invoke<unknown>(
      "service_account_auth_recover",
      withAddr({
        accountId,
        openBrowser: openBrowser ?? false,
      })
    );
    const source =
      result && typeof result === "object" && !Array.isArray(result)
        ? (result as Record<string, unknown>)
        : {};
    return {
      status: String(source.status || "").trim(),
      accountId: String(source.accountId || "").trim(),
      loginId: typeof source.loginId === "string" ? source.loginId.trim() : null,
      authUrl: typeof source.authUrl === "string" ? source.authUrl.trim() : null,
      warning: typeof source.warning === "string" ? source.warning.trim() : null,
    };
  },
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
        expiresAt: params.expiresAt || null,
      })
    );
    return normalizeApiKeyCreateResult(result);
  },
  async listApiKeyUsageStats(): Promise<ApiKeyUsageStat[]> {
    const result = await invoke<unknown>("service_apikey_usage_stats", withAddr());
    return normalizeApiKeyUsageStats(result);
  },
  async getApiKeyRateLimit(keyId: string): Promise<ApiKeyRateLimit> {
    const result = await invoke<unknown>(
      "service_apikey_rate_limit_get",
      withAddr({ keyId })
    );
    return normalizeApiKeyRateLimit(result);
  },
  async getApiKeyModelFallback(keyId: string): Promise<ApiKeyModelFallback> {
    const result = await invoke<unknown>(
      "service_apikey_model_fallback_get",
      withAddr({ keyId })
    );
    return normalizeApiKeyModelFallback(result);
  },
  async getApiKeyAllowedModels(keyId: string): Promise<ApiKeyAllowedModelsConfig> {
    const result = await invoke<unknown>(
      "service_apikey_allowed_models_get",
      withAddr({ keyId })
    );
    return normalizeApiKeyAllowedModels(result);
  },
  async getApiKeyResponseCache(keyId: string): Promise<ApiKeyResponseCacheConfig> {
    const result = await invoke<unknown>(
      "service_apikey_response_cache_get",
      withAddr({ keyId })
    );
    return normalizeApiKeyResponseCache(result);
  },
  setApiKeyRateLimit: (
    keyId: string,
    params: { rpm?: number | null; tpm?: number | null; dailyLimit?: number | null }
  ) =>
    invoke(
      "service_apikey_rate_limit_set",
      withAddr({
        keyId,
        rpm: params.rpm ?? null,
        tpm: params.tpm ?? null,
        dailyLimit: params.dailyLimit ?? null,
      })
    ),
  setApiKeyModelFallback: (keyId: string, params: { modelChain?: string[] | null }) =>
    invoke(
      "service_apikey_model_fallback_set",
      withAddr({
        keyId,
        modelChain: params.modelChain ?? [],
      })
    ),
  setApiKeyAllowedModels: (keyId: string, params: { allowedModels?: string[] | null }) =>
    invoke(
      "service_apikey_allowed_models_set",
      withAddr({
        keyId,
        allowedModels: params.allowedModels ?? [],
      })
    ),
  setApiKeyResponseCache: (keyId: string, params: { enabled: boolean }) =>
    invoke(
      "service_apikey_response_cache_set",
      withAddr({
        keyId,
        enabled: params.enabled,
      })
    ),
  deleteApiKey: (keyId: string) =>
    invoke("service_apikey_delete", withAddr({ keyId })),
  renewApiKey: (keyId: string, expiresAt: number | null) =>
    invoke("service_apikey_renew", withAddr({ keyId, expiresAt })),
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
