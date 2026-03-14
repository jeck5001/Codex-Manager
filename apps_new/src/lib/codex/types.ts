export type ThemeId =
  | "tech"
  | "dark"
  | "business"
  | "mint"
  | "sunset"
  | "grape"
  | "ocean"
  | "forest"
  | "rose"
  | "slate"
  | "aurora";

export type PageId = "dashboard" | "accounts" | "apikeys" | "requestlogs" | "settings";

export type AvailabilityLevel = "ok" | "warn" | "bad" | "unknown";

export type AvailabilityStatus = {
  text: string;
  level: AvailabilityLevel;
};

export type RequestLogTodaySummary = {
  todayTokens: number;
  cachedInputTokens: number;
  reasoningOutputTokens: number;
  estimatedCost: number;
};

export type UsageAggregateSummary = {
  primaryBucketCount: number;
  primaryKnownCount: number;
  primaryUnknownCount: number;
  primaryRemainPercent: number | null;
  secondaryBucketCount: number;
  secondaryKnownCount: number;
  secondaryUnknownCount: number;
  secondaryRemainPercent: number | null;
};

export type AccountRecord = {
  id: string;
  label?: string | null;
  status?: string | null;
  groupName?: string | null;
  sort?: number | null;
  note?: string | null;
  tags?: string[] | string | null;
};

export type UsageSnapshot = {
  accountId: string;
  availabilityStatus?: string | null;
  usedPercent?: number | null;
  windowMinutes?: number | null;
  resetsAt?: number | null;
  secondaryUsedPercent?: number | null;
  secondaryWindowMinutes?: number | null;
  secondaryResetsAt?: number | null;
  capturedAt?: number | null;
  creditsJson?: string | null;
};

export type AccountUsageRefreshProgress = {
  active: boolean;
  completed: number;
  total: number;
  remaining: number;
  lastAccountLabel: string;
};

export type ApiKeyRecord = {
  id: string;
  name?: string | null;
  status?: string | null;
  modelSlug?: string | null;
  reasoningEffort?: string | null;
  protocolType?: string | null;
  upstreamBaseUrl?: string | null;
  staticHeadersJson?: string | null;
};

export type RequestLogRecord = {
  id?: string | null;
  __identity?: string;
  createdAt?: number | null;
  accountId?: string | null;
  accountLabel?: string | null;
  keyId?: string | null;
  method?: string | null;
  requestPath?: string | null;
  originalPath?: string | null;
  path?: string | null;
  mappedPath?: string | null;
  adaptedPath?: string | null;
  responseAdapter?: string | null;
  upstreamUrl?: string | null;
  model?: string | null;
  reasoningEffort?: string | null;
  statusCode?: number | null;
  error?: string | null;
  traceId?: string | null;
};

export type StartupSnapshot = {
  accounts?: AccountRecord[];
  usageSnapshots?: UsageSnapshot[];
  usageAggregateSummary?: Partial<UsageAggregateSummary> | null;
  apiKeys?: ApiKeyRecord[];
  apiModelOptions?: string[];
  manualPreferredAccountId?: string | null;
  requestLogTodaySummary?: Partial<RequestLogTodaySummary> | null;
  requestLogs?: RequestLogRecord[];
};

export type BackgroundTasksSettings = {
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
};

export type GatewayTransportSettings = {
  sseKeepaliveIntervalMs: number;
  upstreamStreamTimeoutMs: number;
};

export type EnvOverrideCatalogItem = {
  key: string;
  name?: string | null;
  description?: string | null;
  scope?: string | null;
  applyMode?: string | null;
  defaultValue?: string | null;
};

export type AppSettingsSnapshot = GatewayTransportSettings & {
  updateAutoCheck: boolean;
  closeToTrayOnClose: boolean;
  closeToTraySupported: boolean;
  lightweightModeOnCloseToTray: boolean;
  lowTransparency: boolean;
  theme: ThemeId;
  serviceAddr: string;
  serviceListenMode: string;
  routeStrategy: string;
  cpaNoCookieHeaderModeEnabled: boolean;
  upstreamProxyUrl: string;
  backgroundTasks: BackgroundTasksSettings;
  envOverrides: Record<string, string>;
  envOverrideCatalog: EnvOverrideCatalogItem[];
  envOverrideReservedKeys: string[];
  envOverrideUnsupportedKeys: string[];
  webAccessPasswordConfigured: boolean;
};

export type ActionResult = {
  ok?: boolean;
  error?: string | null;
  errorCode?: string | null;
  errorDetail?: string | null;
  [key: string]: unknown;
};

export type UpdateCheckResponse = {
  repo: string;
  mode: string;
  isPortable: boolean;
  hasUpdate: boolean;
  canPrepare: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseTag: string;
  releaseName?: string | null;
  publishedAt?: string | null;
  reason?: string | null;
  checkedAtUnixSecs: number;
};

export type UpdatePrepareResponse = {
  prepared: boolean;
  mode: string;
  isPortable: boolean;
  releaseTag: string;
  latestVersion: string;
  assetName: string;
  assetPath: string;
  downloaded: boolean;
};

export type UpdateActionResponse = {
  ok: boolean;
  message: string;
};

export type PendingUpdate = {
  mode: string;
  isPortable: boolean;
  releaseTag: string;
  latestVersion: string;
  assetName: string;
  assetPath: string;
  installerPath?: string | null;
  stagingDir?: string | null;
  preparedAtUnixSecs: number;
};

export type UpdateStatusResponse = {
  repo: string;
  mode: string;
  isPortable: boolean;
  currentVersion: string;
  currentExePath: string;
  portableMarkerPath: string;
  pending?: PendingUpdate | null;
  lastCheck?: UpdateCheckResponse | null;
  lastError?: string | null;
};
