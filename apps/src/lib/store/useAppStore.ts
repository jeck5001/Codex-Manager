import { create } from "zustand";
import { AppSettings, ServiceStatus } from "../../types";

interface AppState {
  serviceStatus: ServiceStatus;
  appSettings: AppSettings;
  isSidebarOpen: boolean;
  
  setServiceStatus: (status: Partial<ServiceStatus>) => void;
  setAppSettings: (settings: Partial<AppSettings>) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  serviceStatus: {
    connected: false,
    version: "",
    uptime: 0,
    addr: "localhost:48760",
  },
  appSettings: {
    updateAutoCheck: true,
    closeToTrayOnClose: false,
    closeToTraySupported: false,
    lowTransparency: false,
    lightweightModeOnCloseToTray: false,
    webAccessPasswordConfigured: false,
    serviceAddr: "localhost:48760",
    serviceListenMode: "loopback",
    serviceListenModeOptions: ["loopback", "all_interfaces"],
    routeStrategy: "ordered",
    routeStrategyOptions: ["ordered", "balanced", "weighted", "least-latency", "cost-first"],
    freeAccountMaxModel: "auto",
    freeAccountMaxModelOptions: [
      "auto",
      "gpt-5",
      "gpt-5-codex",
      "gpt-5-codex-mini",
      "gpt-5.1",
      "gpt-5.1-codex",
      "gpt-5.1-codex-max",
      "gpt-5.1-codex-mini",
      "gpt-5.2",
      "gpt-5.2-codex",
      "gpt-5.3-codex",
      "gpt-5.4",
    ],
    quotaProtectionEnabled: false,
    quotaProtectionThresholdPercent: 10,
    requestCompressionEnabled: true,
    responseCacheEnabled: false,
    responseCacheTtlSecs: 3600,
    responseCacheMaxEntries: 256,
    gatewayOriginator: "codex_cli_rs",
    gatewayResidencyRequirement: "",
    gatewayResidencyRequirementOptions: ["", "us"],
    cpaNoCookieHeaderModeEnabled: false,
    upstreamProxyUrl: "",
    upstreamStreamTimeoutMs: 1800000,
    sseKeepaliveIntervalMs: 15000,
    teamManagerEnabled: false,
    teamManagerApiUrl: "",
    teamManagerHasApiKey: false,
    backgroundTasks: {
      usagePollingEnabled: true,
      usagePollIntervalSecs: 600,
      gatewayKeepaliveEnabled: true,
      gatewayKeepaliveIntervalSecs: 180,
      tokenRefreshPollingEnabled: true,
      tokenRefreshPollIntervalSecs: 60,
      sessionProbePollingEnabled: false,
      sessionProbeIntervalSecs: 300,
      sessionProbeSampleSize: 2,
      usageRefreshWorkers: 4,
      httpWorkerFactor: 4,
      httpWorkerMin: 8,
      httpStreamWorkerFactor: 1,
      httpStreamWorkerMin: 2,
      autoRegisterPoolEnabled: false,
      autoRegisterReadyAccountCount: 2,
      autoRegisterReadyRemainPercent: 20,
      autoDisableRiskyAccountsEnabled: false,
      autoDisableRiskyAccountsFailureThreshold: 3,
      autoDisableRiskyAccountsHealthScoreThreshold: 60,
      autoDisableRiskyAccountsLookbackMins: 60,
      accountCooldownAuthSecs: 300,
      accountCooldownRateLimitedSecs: 45,
      accountCooldownServerErrorSecs: 30,
      accountCooldownNetworkSecs: 20,
      accountCooldownLowQuotaSecs: 1800,
      accountCooldownDeactivatedSecs: 21600,
    },
    envOverrides: {},
    envOverrideCatalog: [],
    envOverrideReservedKeys: [],
    envOverrideUnsupportedKeys: [],
    theme: "tech",
    appearancePreset: "classic",
  },
  isSidebarOpen: true,

  setServiceStatus: (status) => 
    set((state) => ({ serviceStatus: { ...state.serviceStatus, ...status } })),
  
  setAppSettings: (settings) =>
    set((state) => ({ appSettings: { ...state.appSettings, ...settings } })),
    
  toggleSidebar: () => set((state) => ({ isSidebarOpen: !state.isSidebarOpen })),
  
  setSidebarOpen: (open) => set({ isSidebarOpen: open }),
}));
