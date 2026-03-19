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
  usage: AccountUsage | null;
}

export interface AccountListResult {
  items: Account[];
  total: number;
  page: number;
  pageSize: number;
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

export interface RegisterEmailServiceTestResult {
  success: boolean;
  message: string;
  details: Record<string, unknown> | null;
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
