export type AvailabilityLevel = "ok" | "warn" | "bad" | "unknown";
export type AccountHealthTier = "healthy" | "warning" | "risky";

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
  tags: string[];
  sort: number;
  status: string;
  healthScore: number;
  healthTier: AccountHealthTier;
  lastStatusReason: string | null;
  lastStatusChangedAt: number | null;
  lastGovernanceReason: string | null;
  lastGovernanceAt: number | null;
  lastIsolationReasonCode: string | null;
  lastIsolationReason: string | null;
  lastIsolationAt: number | null;
  cooldownUntil: number | null;
  cooldownReasonCode: string | null;
  cooldownReason: string | null;
  newAccountProtectionUntil: number | null;
  newAccountProtectionReason: string | null;
  isNewAccountProtected: boolean;
  isInCooldown: boolean;
  isIsolated: boolean;
  isAvailable: boolean;
  isLowQuota: boolean;
  isDeactivated: boolean;
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

export interface AccountCpaConnectionResult {
  success: boolean;
  message: string;
  totalFiles: number;
}

export interface AccountCpaSyncResult {
  totalFiles: number;
  eligibleFiles: number;
  downloadedFiles: number;
  created: number;
  updated: number;
  failed: number;
  importedAccountIds: string[];
  errors: string[];
}

export interface AccountCpaSyncStatusResult {
  status: string;
  scheduleEnabled: boolean;
  intervalMinutes: number;
  isRunning: boolean;
  lastTrigger: string;
  lastStartedAt: number | null;
  lastFinishedAt: number | null;
  lastSuccessAt: number | null;
  lastSummary: string;
  lastError: string;
  nextRunAt: number | null;
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

export interface FailureReasonSummaryItem {
  code: string;
  label: string;
  count: number;
  affectedAccounts: number;
  lastSeenAt: number | null;
}

export interface GovernanceSummaryItem {
  code: string;
  label: string;
  targetStatus: string;
  count: number;
  affectedAccounts: number;
  lastSeenAt: number | null;
}

export interface OperationAuditItem {
  action: string;
  label: string;
  detail: string;
  accountId: string | null;
  createdAt: number | null;
}

export interface AuditLogItem {
  id: number;
  action: string;
  objectType: string;
  objectId: string | null;
  operator: string;
  changes: Record<string, unknown> | null;
  createdAt: number;
}

export interface AuditLogListResult {
  items: AuditLogItem[];
  total: number;
  page: number;
  pageSize: number;
}

export interface AuditLogExportResult {
  format: string;
  fileName: string;
  content: string;
  recordCount: number;
}

export interface UsagePredictionSummary {
  quotaProtectionEnabled: boolean;
  quotaProtectionThresholdPercent: number;
  readyAccountCount: number;
  estimatedHoursToThreshold: number | null;
  estimatedHoursToPoolExhaustion: number | null;
  thresholdLimitedBy: string | null;
  poolLimitedBy: string | null;
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
  expiresAt: number | null;
}

export interface ApiKeyCreateResult {
  id: string;
  key: string;
}

export interface ApiKeyUsageStat {
  keyId: string;
  totalTokens: number;
}

export interface ApiKeyRateLimit {
  keyId: string;
  rpm: number | null;
  tpm: number | null;
  dailyLimit: number | null;
}

export interface ApiKeyModelFallback {
  keyId: string;
  modelChain: string[];
}

export interface ApiKeyAllowedModelsConfig {
  keyId: string;
  allowedModels: string[];
}

export interface ApiKeyResponseCacheConfig {
  keyId: string;
  enabled: boolean;
}

export type AlertRuleType =
  | "token_refresh_fail"
  | "usage_threshold"
  | "error_rate"
  | "all_unavailable";

export type AlertChannelType = "webhook" | "bark" | "telegram" | "wecom";

export interface AlertRule {
  id: string;
  name: string;
  ruleType: AlertRuleType | string;
  config: Record<string, unknown>;
  enabled: boolean;
  createdAt: number | null;
  updatedAt: number | null;
}

export interface AlertChannel {
  id: string;
  name: string;
  channelType: AlertChannelType | string;
  config: Record<string, unknown>;
  enabled: boolean;
  createdAt: number | null;
  updatedAt: number | null;
}

export interface AlertHistoryItem {
  id: number;
  ruleId: string | null;
  ruleName: string | null;
  channelId: string | null;
  channelName: string | null;
  status: string;
  message: string;
  createdAt: number | null;
}

export interface AlertChannelTestResult {
  channelId: string;
  status: string;
  sentAt: number | null;
}

export type PluginRuntime = "lua";
export type PluginHookPoint = "pre_route" | "post_route" | "post_response";

export interface PluginItem {
  id: string;
  name: string;
  description: string | null;
  runtime: PluginRuntime | string;
  hookPoints: PluginHookPoint[] | string[];
  scriptContent: string;
  enabled: boolean;
  timeoutMs: number;
  createdAt: number | null;
  updatedAt: number | null;
}

export interface ModelPricingItem {
  modelSlug: string;
  inputPricePer1k: number;
  outputPricePer1k: number;
  updatedAt: number | null;
}

export interface CostUsageSummary {
  requestCount: number;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number;
}

export interface CostSummaryKeyItem extends CostUsageSummary {
  keyId: string;
}

export interface CostSummaryModelItem extends CostUsageSummary {
  model: string;
}

export interface CostSummaryDayItem extends CostUsageSummary {
  day: string;
}

export interface CostSummaryResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  total: CostUsageSummary;
  byKey: CostSummaryKeyItem[];
  byModel: CostSummaryModelItem[];
  byDay: CostSummaryDayItem[];
}

export interface CostExportResult {
  fileName: string;
  content: string;
}

export interface RequestTrendItem {
  bucket: string;
  requestCount: number;
  successCount: number;
  successRate: number;
}

export interface RequestTrendResult {
  preset: string;
  granularity: "day" | "week" | "month" | string;
  rangeStart: number;
  rangeEnd: number;
  items: RequestTrendItem[];
}

export interface ModelTrendItem {
  model: string;
  requestCount: number;
  successCount: number;
  successRate: number;
}

export interface ModelTrendResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: ModelTrendItem[];
}

export interface HeatmapCellItem {
  weekday: number;
  hour: number;
  requestCount: number;
  successCount: number;
  successRate: number;
}

export interface HeatmapTrendResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: HeatmapCellItem[];
}

// -- Consumer Analytics --

export interface ConsumerOverviewResult {
  keyId: string;
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  requestCount: number;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number;
  successRate: number;
  avgDurationMs: number | null;
}

export interface ConsumerTrendDayItem {
  day: string;
  requestCount: number;
  inputTokens: number;
  outputTokens: number;
  estimatedCostUsd: number;
}

export interface ConsumerTrendResult {
  keyId: string;
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: ConsumerTrendDayItem[];
}

export interface ConsumerModelItem {
  model: string;
  requestCount: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number;
}

export interface ConsumerModelBreakdownResult {
  keyId: string;
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: ConsumerModelItem[];
}

export interface ConsumerRankingResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: CostSummaryKeyItem[];
}

// -- Cache Analytics --

export interface CacheAnalyticsSummaryResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  totalRequests: number;
  cachedRequests: number;
  hitRate: number;
  totalInputTokens: number;
  cachedInputTokens: number;
  cacheTokenRatio: number;
  estimatedSavingsUsd: number;
}

export interface CacheAnalyticsTrendDayItem {
  day: string;
  totalRequests: number;
  cachedRequests: number;
  hitRate: number;
  totalInputTokens: number;
  cachedInputTokens: number;
}

export interface CacheAnalyticsTrendResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: CacheAnalyticsTrendDayItem[];
}

export interface CacheAnalyticsModelItem {
  model: string;
  totalRequests: number;
  cachedRequests: number;
  hitRate: number;
  totalInputTokens: number;
  cachedInputTokens: number;
  estimatedSavingsUsd: number;
}

export interface CacheAnalyticsByModelResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: CacheAnalyticsModelItem[];
}

export interface CacheAnalyticsKeyItem {
  keyId: string;
  totalRequests: number;
  cachedRequests: number;
  hitRate: number;
  totalInputTokens: number;
  cachedInputTokens: number;
  estimatedSavingsUsd: number;
}

export interface CacheAnalyticsByKeyResult {
  preset: string;
  rangeStart: number;
  rangeEnd: number;
  items: CacheAnalyticsKeyItem[];
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
  candidateCount: number | null;
  attemptedCount: number | null;
  skippedCount: number | null;
  skippedCooldownCount: number | null;
  skippedInflightCount: number | null;
  routeStrategy: string;
  requestedModel: string;
  modelFallbackPath: string[];
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

export interface RequestLogExportResult {
  format: string;
  fileName: string;
  content: string;
  recordCount: number;
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

export interface DashboardAccountStatusBucket {
  key: string;
  label: string;
  count: number;
  percent: number;
}

export interface DashboardGatewayMetrics {
  windowMinutes: number;
  totalRequests: number;
  successRequests: number;
  errorRequests: number;
  qps: number;
  successRate: number;
  p50LatencyMs: number | null;
  p95LatencyMs: number | null;
  p99LatencyMs: number | null;
}

export interface HealthcheckFailedAccount {
  accountId: string;
  label: string | null;
  reason: string;
}

export interface HealthcheckRunResult {
  startedAt: number | null;
  finishedAt: number | null;
  totalAccounts: number;
  sampledAccounts: number;
  successCount: number;
  failureCount: number;
  failedAccounts: HealthcheckFailedAccount[];
}

export interface HealthcheckConfig {
  enabled: boolean;
  intervalSecs: number;
  sampleSize: number;
  recentRun: HealthcheckRunResult | null;
}

export interface DashboardHealth {
  generatedAt: number | null;
  accountStatusBuckets: DashboardAccountStatusBucket[];
  gatewayMetrics: DashboardGatewayMetrics;
  recentHealthcheck: HealthcheckRunResult | null;
}

export interface DashboardTrendPoint {
  bucketTs: number;
  requestCount: number;
  errorCount: number;
  errorRate: number;
}

export interface DashboardTrend {
  generatedAt: number | null;
  bucketMinutes: number;
  points: DashboardTrendPoint[];
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

export interface AccountAuthRecoveryResult {
  status: string;
  accountId: string;
  loginId: string | null;
  authUrl: string | null;
  warning: string | null;
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
  readOnly: boolean;
  description: string;
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
  mail33ImapCount: number;
  generatorEmailCount: number;
  tempmailAvailable: boolean;
  enabledCount: number;
}

export interface RegisterEmailServiceTestResult {
  success: boolean;
  message: string;
  details: Record<string, unknown> | null;
}

export interface RegisterTempMailDomainConfig {
  id: string;
  name: string;
  zoneId: string;
  domainBase: string;
  subdomainMode: string;
  subdomainLength: number;
  subdomainPrefix: string;
  syncCloudflareEnabled: boolean;
  requireCloudflareSync: boolean;
}

export interface RegisterTempMailCloudflareSettings {
  hasApiToken: boolean;
  cloudflareApiEmail: string;
  hasGlobalApiKey: boolean;
  cloudflareAccountId: string;
  cloudflareZoneId: string;
  cloudflareWorkerName: string;
  tempMailBaseUrl: string;
  hasTempMailAdminPassword: boolean;
  domainConfigs: RegisterTempMailDomainConfig[];
  tempMailDomainBase: string;
  tempMailSubdomainMode: string;
  tempMailSubdomainLength: number;
  tempMailSubdomainPrefix: string;
  tempMailSyncCloudflareEnabled: boolean;
  tempMailRequireCloudflareSync: boolean;
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
  domainConfigs: RegisterTempMailDomainConfig[];
}

export interface RegisterAvailableServicesResult {
  serviceUrl: string;
  tempmail: RegisterServiceGroup;
  outlook: RegisterServiceGroup;
  customDomain: RegisterServiceGroup;
  tempMail: RegisterServiceGroup;
  mail33Imap: RegisterServiceGroup;
  generatorEmail: RegisterServiceGroup;
}

export interface RegisterBrowserbaseConfig {
  id: number;
  name: string;
  enabled: boolean;
  priority: number;
  config: Record<string, unknown>;
  lastUsed: string;
  createdAt: string;
  updatedAt: string;
}

export interface RegisterBrowserbaseConfigListResult {
  total: number;
  configs: RegisterBrowserbaseConfig[];
}

export interface RegisterTaskSnapshot {
  taskUuid: string;
  status: string;
  registerMode: string;
  emailServiceId: number | null;
  browserbaseConfigId: number | null;
  proxy: string;
  createdAt: string;
  startedAt: string;
  completedAt: string;
  errorMessage: string;
  failureCode: string;
  failureLabel: string;
  email: string;
  canImport: boolean;
  importedAccountId: string | null;
  isImported: boolean;
  requiresManualImport: boolean;
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

export interface RegisterTaskBatchDeleteResult {
  success: boolean;
  deletedCount: number;
  failedCount: number;
  errors: Array<{
    taskUuid: string;
    error: string;
  }>;
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

export interface RegisterHotmailArtifact {
  filename: string;
  path: string;
  size: number | null;
}

export interface RegisterHotmailLocalHandoffCookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  expires: number | null;
  httpOnly: boolean;
  secure: boolean;
  sameSite: string;
}

export interface RegisterHotmailLocalHandoffOriginEntry {
  name: string;
  value: string;
}

export interface RegisterHotmailLocalHandoffOrigin {
  origin: string;
  localStorage: RegisterHotmailLocalHandoffOriginEntry[];
}

export interface RegisterHotmailLocalHandoff {
  handoffId: string;
  url: string;
  title: string;
  userAgent: string;
  proxyUrl: string;
  state: string;
  cookies: RegisterHotmailLocalHandoffCookie[];
  origins: RegisterHotmailLocalHandoffOrigin[];
}

export interface HotmailLocalHelperHealth {
  ok: boolean;
  service: string;
  version: string;
  playwrightReady: boolean;
}

export interface HotmailLocalHelperLaunchResult {
  ok: boolean;
  handoffId: string;
  profileDir: string;
  message: string;
  error?: string;
}

export interface HotmailLocalHelperTaskStartResult {
  ok: boolean;
  taskId: string;
  message: string;
  error?: string;
}

export interface RegisterHotmailTaskSnapshot {
  taskId: string;
  batchId: string;
  status: string;
  currentStep: string;
  manualActionRequired: boolean;
  failureCode: string;
  failureMessage: string;
  verificationEmail: string;
  targetEmail: string;
  artifactPath: string;
}

export interface RegisterHotmailLocalFirstTaskPayload {
  batchId: string;
  taskId: string;
  profile: Record<string, unknown>;
  targetDomains: string[];
  proxy: string;
  verificationMailbox: Record<string, unknown> | null;
  backendCallbackBase: string;
  backendCallbackToken: string;
}

export interface RegisterHotmailAccountRecord {
  email: string;
  password: string;
  targetDomain: string;
  verificationEmail: string;
}

export interface RegisterHotmailBatchSnapshot {
  batchId: string;
  total: number;
  completed: number;
  success: number;
  failed: number;
  status?: string;
  actionRequiredReason?: string;
  handoffId?: string;
  handoffUrl?: string;
  handoffTitle?: string;
  handoffInstructions?: string;
  localHandoff?: RegisterHotmailLocalHandoff | null;
  executionMode?: string;
  currentTask?: RegisterHotmailTaskSnapshot | null;
  currentTaskPayload?: RegisterHotmailLocalFirstTaskPayload | null;
  cancelled: boolean;
  finished: boolean;
  logs: string[];
  artifacts: RegisterHotmailArtifact[];
  accounts: RegisterHotmailAccountRecord[];
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
  sessionProbePollingEnabled: boolean;
  sessionProbeIntervalSecs: number;
  sessionProbeSampleSize: number;
  usageRefreshWorkers: number;
  httpWorkerFactor: number;
  httpWorkerMin: number;
  httpStreamWorkerFactor: number;
  httpStreamWorkerMin: number;
  autoRegisterPoolEnabled: boolean;
  autoRegisterReadyAccountCount: number;
  autoRegisterReadyRemainPercent: number;
  autoDisableRiskyAccountsEnabled: boolean;
  autoDisableRiskyAccountsFailureThreshold: number;
  autoDisableRiskyAccountsHealthScoreThreshold: number;
  autoDisableRiskyAccountsLookbackMins: number;
  accountCooldownAuthSecs: number;
  accountCooldownRateLimitedSecs: number;
  accountCooldownServerErrorSecs: number;
  accountCooldownNetworkSecs: number;
  accountCooldownLowQuotaSecs: number;
  accountCooldownDeactivatedSecs: number;
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

export interface FreeProxyClearResult {
  previousProxyListValue: string;
  previousProxyListCount: number;
  clearedGatewayProxyCount: number;
  deletedRegisterProxyCount: number;
  failedRegisterProxyCount: number;
  remainingRegisterProxyCount: number;
}

export interface AppSettings {
  updateAutoCheck: boolean;
  closeToTrayOnClose: boolean;
  closeToTraySupported: boolean;
  lowTransparency: boolean;
  lightweightModeOnCloseToTray: boolean;
  webAccessPasswordConfigured: boolean;
  webAccessTwoFactorEnabled: boolean;
  webAccessRecoveryCodesRemaining: number;
  remoteManagementEnabled: boolean;
  remoteManagementSecretConfigured: boolean;
  remoteManagementSecret?: string;
  serviceAddr: string;
  serviceListenMode: string;
  serviceListenModeOptions: string[];
  mcpEnabled: boolean;
  mcpPort: number;
  routeStrategy: string;
  routeStrategyOptions: string[];
  freeAccountMaxModel: string;
  freeAccountMaxModelOptions: string[];
  newAccountProtectionDays: number;
  quotaProtectionEnabled: boolean;
  quotaProtectionThresholdPercent: number;
  requestCompressionEnabled: boolean;
  payloadRewriteRulesJson: string;
  modelAliasPoolsJson: string;
  retryPolicyMaxRetries: number;
  retryPolicyBackoffStrategy: string;
  retryPolicyRetryableStatusCodes: number[];
  responseCacheEnabled: boolean;
  responseCacheTtlSecs: number;
  responseCacheMaxEntries: number;
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
  cpaSyncEnabled: boolean;
  cpaSyncApiUrl: string;
  cpaSyncHasManagementKey: boolean;
  cpaSyncScheduleEnabled: boolean;
  cpaSyncScheduleIntervalMinutes: number;
  cpaSyncManagementKey?: string;
  backgroundTasks: BackgroundTaskSettings;
  envOverrides: Record<string, string>;
  envOverrideCatalog: EnvOverrideCatalogItem[];
  envOverrideReservedKeys: string[];
  envOverrideUnsupportedKeys: string[];
  theme: string;
  appearancePreset: string;
  visibleMenuItems: string[];
  [key: string]: unknown;
}

export interface GatewayRetryPolicy {
  maxRetries: number;
  backoffStrategy: string;
  retryableStatusCodes: number[];
}

export interface GatewayRouteStrategyInfo {
  strategy: string;
  options: string[];
  routeAccountIds: string[];
}

export interface WebAuthTwoFactorSetupResult {
  enabled: boolean;
  secret: string;
  otpAuthUrl: string;
  qrCodeDataUrl: string;
  recoveryCodes: string[];
  setupToken: string;
}

export interface WebAuthTwoFactorStatusResult {
  enabled: boolean;
  recoveryCodesRemaining: number;
  method: string;
}

export interface GatewayResponseCacheStats {
  enabled: boolean;
  ttlSecs: number;
  maxEntries: number;
  entryCount: number;
  estimatedBytes: number;
  hitCount: number;
  missCount: number;
  hitRatePercent: number;
}

export interface ServiceInitializationResult {
  serverName: string;
  version: string;
  userAgent: string;
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
