import "./styles/base.css";
import "./styles/layout.css";
import "./styles/components.css";
import "./styles/responsive.css";
import "./styles/performance.css";

import {
  appSettingsGet,
  appSettingsSet,
  serviceGatewayBackgroundTasksSet,
  serviceGatewayHeaderPolicySet,
  serviceGatewayUpstreamProxySet,
  serviceGatewayRouteStrategySet,
  serviceUsageRefresh,
  updateCheck,
  updateDownload,
  updateInstall,
  updateRestart,
  updateStatus,
} from "./api";
import { state } from "./state";
import { dom } from "./ui/dom";
import { setStatus, setServiceHint } from "./ui/status";
import { createFeedbackHandlers } from "./ui/feedback";
import { createThemeController } from "./ui/theme";
import {
  buildEnvOverrideDescription,
  buildEnvOverrideOptionLabel,
  filterEnvOverrideCatalog,
  formatEnvOverrideDisplayValue,
  normalizeEnvOverrideCatalog,
  normalizeEnvOverrides,
  normalizeStringList,
} from "./ui/env-overrides";
import { withButtonBusy } from "./ui/button-busy";
import { createStartupMaskController } from "./ui/startup-mask";
import { normalizeUpstreamProxyUrl } from "./utils/upstream-proxy.js";
import {
  ensureConnected,
  normalizeAddr,
  startService,
  stopService,
  waitForConnection,
} from "./services/connection";
import {
  refreshAccounts,
  refreshAccountsPage,
  refreshUsageList,
  refreshApiKeys,
  refreshApiModels,
  refreshRequestLogs,
  refreshRequestLogTodaySummary,
  clearRequestLogs,
} from "./services/data";
import {
  ensureAutoRefreshTimer,
  runRefreshTasks,
  stopAutoRefreshTimer,
} from "./services/refresh";
import { createServiceLifecycle } from "./services/service-lifecycle";
import { createLoginFlow } from "./services/login-flow";
import { createManagementActions } from "./services/management-actions";
import { createUpdateController } from "./services/update-controller.js";
import { openAccountModal, closeAccountModal } from "./views/accounts";
import { renderAccountsRefreshProgress } from "./views/accounts/render";
import {
  clearRefreshAllProgress,
  setRefreshAllProgress,
} from "./services/management/account-actions";
import { renderApiKeys, openApiKeyModal, closeApiKeyModal, populateApiKeyModelSelect } from "./views/apikeys";
import { openUsageModal, closeUsageModal, renderUsageSnapshot } from "./views/usage";
import { renderRequestLogs } from "./views/requestlogs";
import { renderAccountsOnly, renderCurrentView } from "./views/renderers";
import { buildRenderActions } from "./views/render-actions";
import { createNavigationHandlers } from "./views/navigation";
import { bindMainEvents } from "./views/event-bindings";
import { bindSettingsEvents } from "./settings/bind-settings-events.js";
import { createSettingsController } from "./settings/controller.js";
import { createSettingsServiceSync } from "./settings/service-sync.js";
import { createAppRuntime } from "./runtime/app-runtime.js";
import { createBootstrapRunner } from "./runtime/app-bootstrap.js";

const { showToast, showConfirmDialog } = createFeedbackHandlers({ dom });
let settingsController = null;
let settingsServiceSync = null;

function saveAppSettingsPatch(patch = {}) {
  if (!settingsController) {
    throw new Error("settings controller is not ready");
  }
  return settingsController.saveAppSettingsPatch(patch);
}

const {
  renderThemeButtons,
  setTheme,
  restoreTheme,
  closeThemePanel,
  toggleThemePanel,
} = createThemeController({
  dom,
  onThemeChange: (theme) => saveAppSettingsPatch({ theme }),
});

function renderCurrentPageView(page = state.currentPage) {
  renderCurrentView(page, buildMainRenderActions());
}

async function reloadAccountsPage(options = {}) {
  const silent = options.silent === true;
  const render = options.render !== false;
  const ensureConnection = options.ensureConnection !== false;

  if (ensureConnection) {
    const ok = await ensureConnected();
    serviceLifecycle.updateServiceToggle();
    if (!ok) {
      return false;
    }
  }

  try {
    const applied = await refreshAccountsPage({ latestOnly: options.latestOnly !== false });
    if (applied !== false && render) {
      renderAccountsView();
    }
    return applied !== false;
  } catch (err) {
    console.error("[accounts] page refresh failed", err);
    if (!silent) {
      showToast(`账号分页刷新失败：${normalizeErrorMessage(err)}`, "error");
    }
    return false;
  }
}

const { switchPage, updateRequestLogFilterButtons } = createNavigationHandlers({
  state,
  dom,
  closeThemePanel,
  onPageActivated: (page) => {
    renderCurrentPageView(page);
    if (page === "accounts") {
      void reloadAccountsPage({ silent: true, latestOnly: true });
    }
  },
});

const { setStartupMask } = createStartupMaskController({ dom, state });
const UPDATE_CHECK_DELAY_MS = 1200;

function isTauriRuntime() {
  return Boolean(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke);
}

function normalizeErrorMessage(err) {
  const raw = String(err && err.message ? err.message : err).trim();
  if (!raw) {
    return "未知错误";
  }
  return raw.length > 120 ? `${raw.slice(0, 120)}...` : raw;
}

settingsController = createSettingsController({
  dom,
  state,
  appSettingsGet,
  appSettingsSet,
  showToast,
  normalizeErrorMessage,
  isTauriRuntime,
  normalizeAddr,
  normalizeUpstreamProxyUrl,
  buildEnvOverrideDescription,
  buildEnvOverrideOptionLabel,
  filterEnvOverrideCatalog,
  formatEnvOverrideDisplayValue,
  normalizeEnvOverrideCatalog,
  normalizeEnvOverrides,
  normalizeStringList,
});

const {
  loadAppSettings,
  getAppSettingsSnapshot,
  applyBrowserModeUi,
  readUpdateAutoCheckSetting,
  saveUpdateAutoCheckSetting,
  initUpdateAutoCheckSetting,
  readCloseToTrayOnCloseSetting,
  saveCloseToTrayOnCloseSetting,
  setCloseToTrayOnCloseToggle,
  applyCloseToTrayOnCloseSetting,
  initCloseToTrayOnCloseSetting,
  readLightweightModeOnCloseToTraySetting,
  saveLightweightModeOnCloseToTraySetting,
  setLightweightModeOnCloseToTrayToggle,
  syncLightweightModeOnCloseToTrayAvailability,
  applyLightweightModeOnCloseToTraySetting,
  initLightweightModeOnCloseToTraySetting,
  readLowTransparencySetting,
  saveLowTransparencySetting,
  applyLowTransparencySetting,
  initLowTransparencySetting,
  normalizeServiceListenMode,
  serviceListenModeLabel,
  buildServiceListenModeHint,
  setServiceListenModeSelect,
  setServiceListenModeHint,
  readServiceListenModeSetting,
  initServiceListenModeSetting,
  applyServiceListenModeToService,
  syncServiceListenModeOnStartup,
  normalizeRouteStrategy,
  routeStrategyLabel,
  readRouteStrategySetting,
  saveRouteStrategySetting,
  setRouteStrategySelect,
  initRouteStrategySetting,
  normalizeCpaNoCookieHeaderMode,
  readCpaNoCookieHeaderModeSetting,
  saveCpaNoCookieHeaderModeSetting,
  setCpaNoCookieHeaderModeToggle,
  initCpaNoCookieHeaderModeSetting,
  readUpstreamProxyUrlSetting,
  saveUpstreamProxyUrlSetting,
  setUpstreamProxyInput,
  setUpstreamProxyHint,
  initUpstreamProxySetting,
  normalizeBackgroundTasksSettings,
  readBackgroundTasksSetting,
  saveBackgroundTasksSetting,
  setBackgroundTasksForm,
  readBackgroundTasksForm,
  updateBackgroundTasksHint,
  initBackgroundTasksSetting,
  getEnvOverrideSelectedKey,
  findEnvOverrideCatalogItem,
  setEnvOverridesHint,
  readEnvOverridesSetting,
  buildEnvOverrideHint,
  saveEnvOverridesSetting,
  renderEnvOverrideEditor,
  initEnvOverridesSetting,
  updateWebAccessPasswordState,
  syncWebAccessPasswordInputs,
  saveWebAccessPassword,
  clearWebAccessPassword,
  openWebSecurityModal,
  closeWebSecurityModal,
  persistServiceAddrInput,
  uiLowTransparencyToggleId,
  upstreamProxyHintText,
  backgroundTasksRestartKeysDefault,
} = settingsController;

const serviceLifecycle = createServiceLifecycle({
  state,
  dom,
  setServiceHint,
  normalizeAddr,
  startService,
  stopService,
  waitForConnection,
  refreshAll: () => refreshAll(),
  maybeRefreshApiModelsCache: (options) => maybeRefreshApiModelsCache(options),
  ensureAutoRefreshTimer,
  stopAutoRefreshTimer,
  onStartupState: (loading, message) => setStartupMask(loading, message),
});

const {
  nextPaintTick,
  maybeRefreshApiModelsCache,
  refreshAll,
  handleRefreshAllClick,
  refreshAccountsAndUsage,
} = createAppRuntime({
  state,
  dom,
  ensureConnected,
  refreshAccounts,
  refreshAccountsPage,
  refreshUsageList,
  refreshApiKeys,
  refreshApiModels,
  refreshRequestLogs,
  refreshRequestLogTodaySummary,
  serviceUsageRefresh,
  runRefreshTasks,
  renderAccountsRefreshProgress,
  setRefreshAllProgress,
  clearRefreshAllProgress,
  renderCurrentPageView,
  showToast,
  serviceLifecycle,
  syncRuntimeSettingsForCurrentProbe,
  populateApiKeyModelSelect,
});

const {
  handleCheckUpdateClick,
  scheduleStartupUpdateCheck,
  bootstrapUpdateStatus,
} = createUpdateController({
  dom,
  showToast,
  showConfirmDialog,
  normalizeErrorMessage,
  isTauriRuntime,
  readUpdateAutoCheckSetting,
  updateCheck,
  updateDownload,
  updateInstall,
  updateRestart,
  updateStatus,
  withButtonBusy,
  nextPaintTick,
  updateCheckDelayMs: UPDATE_CHECK_DELAY_MS,
});

settingsServiceSync = createSettingsServiceSync({
  state,
  showToast,
  normalizeErrorMessage,
  isTauriRuntime,
  ensureConnected,
  serviceLifecycle,
  serviceGatewayRouteStrategySet,
  serviceGatewayHeaderPolicySet,
  serviceGatewayUpstreamProxySet,
  serviceGatewayBackgroundTasksSet,
  readRouteStrategySetting,
  saveRouteStrategySetting,
  setRouteStrategySelect,
  normalizeRouteStrategy,
  routeStrategyLabel,
  readCpaNoCookieHeaderModeSetting,
  saveCpaNoCookieHeaderModeSetting,
  setCpaNoCookieHeaderModeToggle,
  normalizeCpaNoCookieHeaderMode,
  readUpstreamProxyUrlSetting,
  saveUpstreamProxyUrlSetting,
  setUpstreamProxyInput,
  setUpstreamProxyHint,
  normalizeUpstreamProxyUrl,
  upstreamProxyHintText,
  readBackgroundTasksSetting,
  saveBackgroundTasksSetting,
  setBackgroundTasksForm,
  normalizeBackgroundTasksSettings,
  updateBackgroundTasksHint,
  backgroundTasksRestartKeysDefault,
});

function requireSettingsServiceSync() {
  if (!settingsServiceSync) {
    throw new Error("settings service sync is not ready");
  }
  return settingsServiceSync;
}

async function applyRouteStrategyToService(strategy, options) {
  return requireSettingsServiceSync().applyRouteStrategyToService(strategy, options);
}

async function applyCpaNoCookieHeaderModeToService(enabled, options) {
  return requireSettingsServiceSync().applyCpaNoCookieHeaderModeToService(enabled, options);
}

async function applyUpstreamProxyToService(proxyUrl, options) {
  return requireSettingsServiceSync().applyUpstreamProxyToService(proxyUrl, options);
}

async function applyBackgroundTasksToService(settings, options) {
  return requireSettingsServiceSync().applyBackgroundTasksToService(settings, options);
}

async function syncRuntimeSettingsForCurrentProbe() {
  return requireSettingsServiceSync().syncRuntimeSettingsForCurrentProbe();
}

async function syncRuntimeSettingsOnStartup() {
  return requireSettingsServiceSync().syncRuntimeSettingsOnStartup();
}

const loginFlow = createLoginFlow({
  dom,
  state,
  withButtonBusy,
  ensureConnected,
  refreshAll,
  closeAccountModal,
});

const managementActions = createManagementActions({
  dom,
  state,
  ensureConnected,
  withButtonBusy,
  showToast,
  showConfirmDialog,
  clearRequestLogs,
  refreshRequestLogs,
  renderRequestLogs,
  refreshAccountsAndUsage,
  renderAccountsView,
  renderCurrentPageView,
  openUsageModal,
  renderUsageSnapshot,
  refreshApiModels,
  refreshApiKeys,
  populateApiKeyModelSelect,
  renderApiKeys,
});

const {
  handleClearRequestLogs,
  updateAccountSort,
  setManualPreferredAccount,
  deleteAccount,
  importAccountsFromFiles,
  importAccountsFromDirectory,
  deleteSelectedAccounts,
  deleteUnavailableFreeAccounts,
  exportAccountsByFile,
  handleOpenUsageModal,
  refreshUsageForAccount,
  createApiKey,
  deleteApiKey,
  toggleApiKeyStatus,
  updateApiKeyModel,
  copyApiKey,
  refreshApiModelsNow,
} = managementActions;

function buildMainRenderActions() {
  return buildRenderActions({
    updateAccountSort,
    handleOpenUsageModal,
    setManualPreferredAccount,
    deleteAccount,
    refreshAccountsPage: () => reloadAccountsPage({ latestOnly: true, silent: false }),
    toggleApiKeyStatus,
    deleteApiKey,
    updateApiKeyModel,
    copyApiKey,
  });
}

function renderAccountsView() {
  renderAccountsOnly(buildMainRenderActions());
}

function bindEvents() {
  bindMainEvents({
    dom,
    state,
    switchPage,
    openAccountModal,
    openApiKeyModal,
    closeAccountModal,
    handleLogin: loginFlow.handleLogin,
    handleCancelLogin: loginFlow.handleCancelLogin,
    showToast,
    handleManualCallback: loginFlow.handleManualCallback,
    closeUsageModal,
    refreshUsageForAccount,
    closeApiKeyModal,
    createApiKey,
    handleClearRequestLogs,
    refreshRequestLogs,
    renderRequestLogs,
    refreshAll: handleRefreshAllClick,
    ensureConnected,
    refreshApiModels,
    refreshApiModelsNow,
    populateApiKeyModelSelect,
    importAccountsFromFiles,
    importAccountsFromDirectory,
    deleteSelectedAccounts,
    deleteUnavailableFreeAccounts,
    exportAccountsByFile,
    toggleThemePanel,
    closeThemePanel,
    setTheme,
    handleServiceToggle: serviceLifecycle.handleServiceToggle,
    renderAccountsView,
    refreshAccountsPage: (options) => reloadAccountsPage(options),
    updateRequestLogFilterButtons,
  });

  bindSettingsEvents({
    dom,
    showToast,
    withButtonBusy,
    normalizeErrorMessage,
    saveAppSettingsPatch,
    handleCheckUpdateClick,
    isTauriRuntime,
    readUpdateAutoCheckSetting,
    saveUpdateAutoCheckSetting,
    readCloseToTrayOnCloseSetting,
    saveCloseToTrayOnCloseSetting,
    setCloseToTrayOnCloseToggle,
    applyCloseToTrayOnCloseSetting,
    readLightweightModeOnCloseToTraySetting,
    saveLightweightModeOnCloseToTraySetting,
    setLightweightModeOnCloseToTrayToggle,
    syncLightweightModeOnCloseToTrayAvailability,
    applyLightweightModeOnCloseToTraySetting,
    readRouteStrategySetting,
    normalizeRouteStrategy,
    saveRouteStrategySetting,
    setRouteStrategySelect,
    applyRouteStrategyToService,
    routeStrategyLabel,
    readServiceListenModeSetting,
    normalizeServiceListenMode,
    setServiceListenModeSelect,
    setServiceListenModeHint,
    buildServiceListenModeHint,
    applyServiceListenModeToService,
    readCpaNoCookieHeaderModeSetting,
    saveCpaNoCookieHeaderModeSetting,
    setCpaNoCookieHeaderModeToggle,
    normalizeCpaNoCookieHeaderMode,
    applyCpaNoCookieHeaderModeToService,
    readUpstreamProxyUrlSetting,
    saveUpstreamProxyUrlSetting,
    setUpstreamProxyInput,
    setUpstreamProxyHint,
    normalizeUpstreamProxyUrl,
    applyUpstreamProxyToService,
    upstreamProxyHintText,
    readBackgroundTasksSetting,
    readBackgroundTasksForm,
    saveBackgroundTasksSetting,
    setBackgroundTasksForm,
    normalizeBackgroundTasksSettings,
    updateBackgroundTasksHint,
    applyBackgroundTasksToService,
    backgroundTasksRestartKeysDefault,
    getEnvOverrideSelectedKey,
    findEnvOverrideCatalogItem,
    setEnvOverridesHint,
    readEnvOverridesSetting,
    buildEnvOverrideHint,
    normalizeEnvOverrides,
    normalizeEnvOverrideCatalog,
    saveEnvOverridesSetting,
    renderEnvOverrideEditor,
    persistServiceAddrInput,
    uiLowTransparencyToggleId,
    readLowTransparencySetting,
    saveLowTransparencySetting,
    applyLowTransparencySetting,
    syncWebAccessPasswordInputs,
    saveWebAccessPassword,
    clearWebAccessPassword,
    openWebSecurityModal,
    closeWebSecurityModal,
  });
}

const bootstrap = createBootstrapRunner({
  setStartupMask,
  setStatus,
  loadAppSettings,
  applyBrowserModeUi,
  setServiceHint,
  renderThemeButtons,
  getAppSettingsSnapshot,
  restoreTheme,
  initLowTransparencySetting,
  initUpdateAutoCheckSetting,
  initCloseToTrayOnCloseSetting,
  initLightweightModeOnCloseToTraySetting,
  initServiceListenModeSetting,
  initRouteStrategySetting,
  initCpaNoCookieHeaderModeSetting,
  initUpstreamProxySetting,
  initBackgroundTasksSetting,
  initEnvOverridesSetting,
  updateWebAccessPasswordState,
  bootstrapUpdateStatus,
  serviceLifecycle,
  bindEvents,
  renderCurrentPageView,
  updateRequestLogFilterButtons,
  scheduleStartupUpdateCheck,
  syncServiceListenModeOnStartup,
  syncRuntimeSettingsOnStartup,
});

window.addEventListener("DOMContentLoaded", () => {
  void bootstrap();
});








