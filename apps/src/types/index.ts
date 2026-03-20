export type AvailabilityLevel = "ok" | "warn" | "bad" | "unknown";

export interface ServiceStatus {
  connected: boolean;
  version: string;
  uptime: number;
  addr: string;
}

export interface AccountUsage {
  accountId: string;
  availabilityStatus: string;
  usedPercent: number | null;
  windowMinutes: number | null;
  resetsAt: number | null;
  secondaryUsedPercent: number | null;
  secondaryWindowMinutes: number | null;
  secondaryResetsAt: number | null;
  creditsJson: string | null;
  capturedAt: number | null;
}

export interface Account {
  id: string;
  name: string;
  group: string;
  priority: number;
  label: string;
  groupName: string;
  sort: number;
  status: string;
  isAvailable: boolean;
  isLowQuota: boolean;
  lastRefreshAt: number | null;
  availabilityText: string;
  availabilityLevel: AvailabilityLevel;
  primaryRemainPercent: number | null;
  secondaryRemainPercent: number | null;
  subscriptionPlanType: string | null;
  subscriptionUpdatedAt: number | null;
  teamManagerUploadedAt: number | null;
  officialPromoLink: string | null;
  officialPromoLinkUpdatedAt: number | null;
  usage: AccountUsage | null;
}

export interface AccountListResult {
  items: Account[];
  total: number;
  page: number;
  pageSize: number;
}

export interface AccountPaymentLinkResult {
  accountId: string;
  accountName: string;
  planType: string;
  link: string;
}

export interface AccountSubscriptionCheckResult {
  accountId: string;
  accountName?: string;
  success: boolean;
  planType?: string | null;
  subscriptionUpdatedAt?: number | null;
  rawPlanType?: string | null;
  error?: string;
}

export interface AccountOfficialPromoLinkResult {
  accountId: string;
  accountName?: string;
  success: boolean;
  officialPromoLink?: string | null;
  officialPromoLinkUpdatedAt?: number | null;
  error?: string;
}

export interface AccountSubscriptionCheckManyResult {
  successCount: number;
  failedCount: number;
  details: AccountSubscriptionCheckResult[];
}

export interface AccountTeamManagerUploadResult {
  accountId: string;
  accountName?: string;
  success: boolean;
  message?: string;
  uploadedAt?: number | null;
  error?: string;
}

export interface AccountTeamManagerUploadManyResult {
  successCount: number;
  failedCount: number;
  skippedCount: number;
  details: AccountTeamManagerUploadResult[];
}

export interface AccountBulkStatusUpdateError {
  accountId: string;
  message: string;
}

export interface AccountBulkStatusUpdateResult {
  requested: number;
  updated: number;
  skipped: number;
  failed: number;
  targetStatus: string;
  updatedAccountIds: string[];
  skippedAccountIds: string[];
  errors: AccountBulkStatusUpdateError[];
}

export interface UsageAggregateSummary {
  primaryBucketCount: number;
  primaryKnownCount: number;
  primaryUnknownCount: number;
  primaryRemainPercent: number | null;
  secondaryBucketCount: number;
  secondaryKnownCount: number;
  secondaryUnknownCount: number;
  secondaryRemainPercent: number | null;
}

export interface ApiKey {
  id: string;
  name: string;
  model: string;
  modelSlug: string;
  reasoningEffort: string;
  protocol: string;
  clientType: string;
  authScheme: string;
  upstreamBaseUrl: string;
  staticHeadersJson: string;
  status: string;
  createdAt: number | null;
  lastUsedAt: number | null;
}

export interface ApiKeyCreateResult {
  id: string;
  key: string;
}

export interface ApiKeyUsageStat {
  keyId: string;
  totalTokens: number;
}

export interface ModelOption {
  slug: string;
  displayName: string;
}

export interface RequestLog {
  id: string;
  traceId: string;
  keyId: string;
  accountId: string;
  initialAccountId: string;
  attemptedAccountIds: string[];
  requestPath: string;
  originalPath: string;
  adaptedPath: string;
  method: string;
  path: string;
  model: string;
  reasoningEffort: string;
  responseAdapter: string;
  upstreamUrl: string;
  statusCode: number | null;
  inputTokens: number | null;
  cachedInputTokens: number | null;
  outputTokens: number | null;
  totalTokens: number | null;
  reasoningOutputTokens: number | null;
  estimatedCostUsd: number | null;
  durationMs: number | null;
  error: string;
  createdAt: number | null;
}

export interface RequestLogListResult {
  items: RequestLog[];
  total: number;
  page: number;
  pageSize: number;
}

export interface RequestLogFilterSummary {
  totalCount: number;
  filteredCount: number;
  successCount: number;
  errorCount: number;
  totalTokens: number;
}

export interface LoginStatusResult {
  status: string;
  error: string;
}

export interface RequestLogTodaySummary {
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
  todayTokens: number;
  estimatedCost: number;
}

export interface DeviceAuthInfo {
  userCodeUrl: string;
  tokenUrl: string;
  verificationUrl: string;
  redirectUri: string;
}

export interface LoginStartResult {
  authUrl: string;
  loginId: string;
  loginType: string;
  issuer: string;
  clientId: string;
  redirectUri: string;
  warning: string;
  device: DeviceAuthInfo | null;
}

export interface CurrentAccessTokenAccount {
  type: string;
  accountId: string;
  email: string;
  planType: string;
  planTypeRaw?: string | null;
  chatgptAccountId: string | null;
  workspaceId: string | null;
  status: string;
}

export interface CurrentAccessTokenAccountReadResult {
  account: CurrentAccessTokenAccount | null;
  authMode: string | null;
  requiresOpenaiAuth: boolean;
}

export interface ChatgptAuthTokensRefreshResult {
  accountId: string;
  accessToken: string;
  chatgptAccountId: string;
  chatgptPlanType: string | null;
  chatgptPlanTypeRaw?: string | null;
}

export interface RegisterServiceItem {
  id: number | null;
  name: string;
  type: string;
  description: string;
}

export interface RegisterEmailServiceField {
  name: string;
  label: string;
  required: boolean;
  defaultValue: string | number | boolean | null;
  placeholder: string;
  secret: boolean;
}

export interface RegisterEmailServiceType {
  value: string;
  label: string;
  description: string;
  configFields: RegisterEmailServiceField[];
}

export interface RegisterEmailServiceTypeCatalog {
  types: RegisterEmailServiceType[];
}

export interface RegisterEmailService {
  id: number;
  serviceType: string;
  name: string;
  enabled: boolean;
  priority: number;
  config: Record<string, unknown>;
  lastUsed: string;
  createdAt: string;
  updatedAt: string;
}

export interface RegisterEmailServiceListResult {
  total: number;
  services: RegisterEmailService[];
}

export interface RegisterEmailServiceStats {
  outlookCount: number;
  customCount: number;
  tempMailCount: number;
  tempmailAvailable: boolean;
  enabledCount: number;
}

export interface RegisterEmailServiceTestResult {
  success: boolean;
  message: string;
  details: Record<string, unknown> | null;
}

export interface RegisterEmailServiceBatchDeleteResult {
  success: boolean;
  deleted: number;
  message: string;
}

export interface RegisterOutlookBatchImportResult {
  total: number;
  success: number;
  failed: number;
  accounts: Array<Record<string, unknown>>;
  errors: string[];
}

export interface RegisterServiceGroup {
  available: boolean;
  count: number;
  services: RegisterServiceItem[];
}

export interface RegisterAvailableServicesResult {
  serviceUrl: string;
  tempmail: RegisterServiceGroup;
  outlook: RegisterServiceGroup;
  customDomain: RegisterServiceGroup;
  tempMail: RegisterServiceGroup;
}

export interface RegisterTaskSnapshot {
  taskUuid: string;
  status: string;
  emailServiceId: number | null;
  proxy: string;
  createdAt: string;
  startedAt: string;
  completedAt: string;
  errorMessage: string;
  email: string;
  canImport: boolean;
  logs: string[];
}

export interface RegisterBatchStartResult {
  batchId: string;
  count: number;
  taskUuids: string[];
}

export interface RegisterBatchSnapshot {
  batchId: string;
  total: number;
  completed: number;
  success: number;
  failed: number;
  currentIndex: number;
  cancelled: boolean;
  finished: boolean;
  progress: string;
  logs: string[];
}

export interface RegisterTaskListResult {
  total: number;
  tasks: RegisterTaskSnapshot[];
}

export interface RegisterStats {
  byStatus: Record<string, number>;
  todayCount: number;
}

export interface RegisterOutlookAccount {
  id: number;
  email: string;
  name: string;
  hasOauth: boolean;
  isRegistered: boolean;
  registeredAccountId: number | null;
}

export interface RegisterOutlookAccountsResult {
  total: number;
  registeredCount: number;
  unregisteredCount: number;
  accounts: RegisterOutlookAccount[];
}

export interface RegisterOutlookBatchStartResult {
  batchId: string;
  total: number;
  skipped: number;
  toRegister: number;
  serviceIds: number[];
}

export interface RegisterOutlookBatchSnapshot {
  batchId: string;
  total: number;
  completed: number;
  success: number;
  failed: number;
  skipped: number;
  currentIndex: number;
  cancelled: boolean;
  finished: boolean;
  progress: string;
  logs: string[];
}

export interface RegisterImportResult {
  taskUuid: string;
  email: string;
  remoteAccountId: number | null;
  accountId: string;
  chatgptAccountId: string;
  workspaceId: string;
  type: string;
}

export interface EnvOverrideCatalogItem {
  key: string;
  label: string;
  defaultValue: string;
  scope: string;
  applyMode: string;
}

export interface BackgroundTaskSettings {
  usagePollingEnabled: boolean;
  usagePollIntervalSecs: number;
  gatewayKeepaliveEnabled: boolean;
  gatewayKeepaliveIntervalSecs: number;
  tokenRefreshPollingEnabled: boolean;
  tokenRefreshPollIntervalSecs: number;
  usageRefreshWorkers: number;
  httpWorkerFactor: number;
  httpWorkerMin: number;
  httpStreamWorkerFactor: number;
  httpStreamWorkerMin: number;
  autoRegisterPoolEnabled: boolean;
  autoRegisterReadyAccountCount: number;
  autoRegisterReadyRemainPercent: number;
}

export interface FreeProxySyncResult {
  sourceUrl: string;
  sourceUpdatedAt: string | null;
  fetchedCount: number;
  matchedCount: number;
  appliedCount: number;
  protocol: string;
  anonymity: string;
  countryFilter: string[];
  limit: number;
  clearedUpstreamProxyUrl: boolean;
  singleProxyStillConfigured: boolean;
  previousUpstreamProxyUrl: string | null;
  proxyListValue: string;
  proxies: string[];
  registerProxySyncEnabled: boolean;
  registerProxyCreatedCount: number;
  registerProxyUpdatedCount: number;
  registerProxyTotalCount: number;
}

export interface AppSettings {
  updateAutoCheck: boolean;
  closeToTrayOnClose: boolean;
  closeToTraySupported: boolean;
  lowTransparency: boolean;
  lightweightModeOnCloseToTray: boolean;
  webAccessPasswordConfigured: boolean;
  serviceAddr: string;
  serviceListenMode: string;
  serviceListenModeOptions: string[];
  routeStrategy: string;
  routeStrategyOptions: string[];
  freeAccountMaxModel: string;
  freeAccountMaxModelOptions: string[];
  quotaProtectionEnabled: boolean;
  quotaProtectionThresholdPercent: number;
  requestCompressionEnabled: boolean;
  gatewayOriginator: string;
  gatewayResidencyRequirement: string;
  gatewayResidencyRequirementOptions: string[];
  cpaNoCookieHeaderModeEnabled: boolean;
  upstreamProxyUrl: string;
  upstreamStreamTimeoutMs: number;
  sseKeepaliveIntervalMs: number;
  teamManagerEnabled: boolean;
  teamManagerApiUrl: string;
  teamManagerHasApiKey: boolean;
  teamManagerApiKey?: string;
  backgroundTasks: BackgroundTaskSettings;
  envOverrides: Record<string, string>;
  envOverrideCatalog: EnvOverrideCatalogItem[];
  envOverrideReservedKeys: string[];
  envOverrideUnsupportedKeys: string[];
  theme: string;
  appearancePreset: string;
  [key: string]: unknown;
}

export interface ServiceInitializationResult {
  serverName: string;
  version: string;
  userAgent: string;
}

export interface StartupSnapshot {
  accounts: Account[];
  usageSnapshots: AccountUsage[];
  usageAggregateSummary: UsageAggregateSummary;
  apiKeys: ApiKey[];
  apiModelOptions: ModelOption[];
  manualPreferredAccountId: string;
  requestLogTodaySummary: RequestLogTodaySummary;
  requestLogs: RequestLog[];
}
