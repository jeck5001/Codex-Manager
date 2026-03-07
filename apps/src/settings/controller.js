import { appSettingsGet as defaultAppSettingsGet, appSettingsSet as defaultAppSettingsSet } from "../api.js";
import { normalizeAddr as defaultNormalizeAddr } from "../services/connection.js";
import {
  buildEnvOverrideDescription as defaultBuildEnvOverrideDescription,
  buildEnvOverrideOptionLabel as defaultBuildEnvOverrideOptionLabel,
  filterEnvOverrideCatalog as defaultFilterEnvOverrideCatalog,
  formatEnvOverrideDisplayValue as defaultFormatEnvOverrideDisplayValue,
  normalizeEnvOverrideCatalog as defaultNormalizeEnvOverrideCatalog,
  normalizeEnvOverrides as defaultNormalizeEnvOverrides,
  normalizeStringList as defaultNormalizeStringList,
} from "../ui/env-overrides.js";
import { normalizeUpstreamProxyUrl as defaultNormalizeUpstreamProxyUrl } from "../utils/upstream-proxy.js";

const ROUTE_STRATEGY_ORDERED = "ordered";
const ROUTE_STRATEGY_BALANCED = "balanced";
const SERVICE_LISTEN_MODE_LOOPBACK = "loopback";
const SERVICE_LISTEN_MODE_ALL_INTERFACES = "all_interfaces";
const UI_LOW_TRANSPARENCY_BODY_CLASS = "cm-low-transparency";
const UI_LOW_TRANSPARENCY_TOGGLE_ID = "lowTransparencyMode";
const UI_LOW_TRANSPARENCY_CARD_ID = "settingsLowTransparencyCard";
const UPSTREAM_PROXY_HINT_TEXT = "支持 http/https/socks5，留空直连，socks 会自动按 socks5h 处理。";
const DEFAULT_BACKGROUND_TASKS_SETTINGS = {
  usagePollingEnabled: true,
  usagePollIntervalSecs: 600,
  gatewayKeepaliveEnabled: true,
  gatewayKeepaliveIntervalSecs: 180,
  tokenRefreshPollingEnabled: true,
  tokenRefreshPollIntervalSecs: 60,
  usageRefreshWorkers: 4,
  httpWorkerFactor: 4,
  httpWorkerMin: 8,
  httpStreamWorkerFactor: 1,
  httpStreamWorkerMin: 2,
};
const BACKGROUND_TASKS_RESTART_KEYS_DEFAULT = [
  "usageRefreshWorkers",
  "httpWorkerFactor",
  "httpWorkerMin",
  "httpStreamWorkerFactor",
  "httpStreamWorkerMin",
];
const BACKGROUND_TASKS_RESTART_KEY_LABELS = {
  usageRefreshWorkers: "用量刷新并发线程数",
  httpWorkerFactor: "普通请求并发因子",
  httpWorkerMin: "普通请求最小并发",
  httpStreamWorkerFactor: "流式请求并发因子",
  httpStreamWorkerMin: "流式请求最小并发",
};

function defaultNormalizeRouteStrategy(strategy) {
  const raw = String(strategy || "").trim().toLowerCase();
  if (["balanced", "round_robin", "round-robin", "rr"].includes(raw)) {
    return ROUTE_STRATEGY_BALANCED;
  }
  return ROUTE_STRATEGY_ORDERED;
}

function defaultRouteStrategyLabel(strategy) {
  return defaultNormalizeRouteStrategy(strategy) === ROUTE_STRATEGY_BALANCED ? "均衡轮询" : "顺序优先";
}

function defaultNormalizeServiceListenMode(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (["all_interfaces", "all-interfaces", "all", "0.0.0.0"].includes(raw)) {
    return SERVICE_LISTEN_MODE_ALL_INTERFACES;
  }
  return SERVICE_LISTEN_MODE_LOOPBACK;
}

function defaultServiceListenModeLabel(mode) {
  return defaultNormalizeServiceListenMode(mode) === SERVICE_LISTEN_MODE_ALL_INTERFACES
    ? "全部网卡（0.0.0.0）"
    : "仅本机（localhost / 127.0.0.1）";
}

function defaultNormalizeCpaNoCookieHeaderMode(value) {
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value !== 0;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (["1", "true", "yes", "on"].includes(normalized)) {
      return true;
    }
    if (["0", "false", "no", "off"].includes(normalized)) {
      return false;
    }
  }
  return false;
}

function normalizeBooleanSetting(value, fallback = false) {
  if (value == null) {
    return Boolean(fallback);
  }
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value !== 0;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (["1", "true", "yes", "on"].includes(normalized)) {
      return true;
    }
    if (["0", "false", "no", "off"].includes(normalized)) {
      return false;
    }
  }
  return Boolean(fallback);
}

function normalizePositiveInteger(value, fallback, min = 1) {
  const fallbackValue = Math.max(min, Math.floor(Number(fallback) || min));
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallbackValue;
  }
  const intValue = Math.floor(numeric);
  if (intValue < min) {
    return min;
  }
  return intValue;
}

function normalizeThemeSetting(value) {
  const normalized = String(value || "").trim().toLowerCase();
  return normalized || "tech";
}

export function createSettingsController(deps = {}) {
  const {
    dom = {},
    state = {},
    showToast = () => {},
    normalizeErrorMessage = (err) => String(err?.message || err || ""),
    isTauriRuntime = () => false,
    appSettingsGet = defaultAppSettingsGet,
    appSettingsSet = defaultAppSettingsSet,
    normalizeAddr = defaultNormalizeAddr,
    normalizeRouteStrategy = defaultNormalizeRouteStrategy,
    routeStrategyLabel = defaultRouteStrategyLabel,
    normalizeServiceListenMode = defaultNormalizeServiceListenMode,
    serviceListenModeLabel = defaultServiceListenModeLabel,
    normalizeCpaNoCookieHeaderMode = defaultNormalizeCpaNoCookieHeaderMode,
    normalizeUpstreamProxyUrl = defaultNormalizeUpstreamProxyUrl,
    buildEnvOverrideDescription = defaultBuildEnvOverrideDescription,
    buildEnvOverrideOptionLabel = defaultBuildEnvOverrideOptionLabel,
    filterEnvOverrideCatalog = defaultFilterEnvOverrideCatalog,
    formatEnvOverrideDisplayValue = defaultFormatEnvOverrideDisplayValue,
    normalizeEnvOverrideCatalog = defaultNormalizeEnvOverrideCatalog,
    normalizeEnvOverrides = defaultNormalizeEnvOverrides,
    normalizeStringList = defaultNormalizeStringList,
    documentRef,
  } = deps;

  let serviceListenModeSyncInFlight = null;
  let envOverrideSelectedKey = "";
  let appSettingsSnapshot = buildDefaultAppSettingsSnapshot();

  function getDocumentRef() {
    if (documentRef) {
      return documentRef;
    }
    if (typeof document !== "undefined") {
      return document;
    }
    return null;
  }

  function buildDefaultAppSettingsSnapshot() {
    return {
      updateAutoCheck: true,
      closeToTrayOnClose: false,
      closeToTraySupported: isTauriRuntime(),
      lightweightModeOnCloseToTray: false,
      lowTransparency: false,
      theme: "tech",
      serviceAddr: "localhost:48760",
      serviceListenMode: normalizeServiceListenMode(null),
      routeStrategy: normalizeRouteStrategy(null),
      cpaNoCookieHeaderModeEnabled: false,
      upstreamProxyUrl: "",
      backgroundTasks: normalizeBackgroundTasksSettings(DEFAULT_BACKGROUND_TASKS_SETTINGS),
      envOverrides: {},
      envOverrideCatalog: [],
      envOverrideReservedKeys: [],
      envOverrideUnsupportedKeys: [],
      webAccessPasswordConfigured: false,
    };
  }

  function normalizeBackgroundTasksSettings(input) {
    const source = input && typeof input === "object" ? input : {};
    return {
      usagePollingEnabled: normalizeBooleanSetting(
        source.usagePollingEnabled,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.usagePollingEnabled,
      ),
      usagePollIntervalSecs: normalizePositiveInteger(
        source.usagePollIntervalSecs,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.usagePollIntervalSecs,
        1,
      ),
      gatewayKeepaliveEnabled: normalizeBooleanSetting(
        source.gatewayKeepaliveEnabled,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.gatewayKeepaliveEnabled,
      ),
      gatewayKeepaliveIntervalSecs: normalizePositiveInteger(
        source.gatewayKeepaliveIntervalSecs,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.gatewayKeepaliveIntervalSecs,
        1,
      ),
      tokenRefreshPollingEnabled: normalizeBooleanSetting(
        source.tokenRefreshPollingEnabled,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.tokenRefreshPollingEnabled,
      ),
      tokenRefreshPollIntervalSecs: normalizePositiveInteger(
        source.tokenRefreshPollIntervalSecs,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.tokenRefreshPollIntervalSecs,
        1,
      ),
      usageRefreshWorkers: normalizePositiveInteger(
        source.usageRefreshWorkers,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.usageRefreshWorkers,
        1,
      ),
      httpWorkerFactor: normalizePositiveInteger(
        source.httpWorkerFactor,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.httpWorkerFactor,
        1,
      ),
      httpWorkerMin: normalizePositiveInteger(
        source.httpWorkerMin,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.httpWorkerMin,
        1,
      ),
      httpStreamWorkerFactor: normalizePositiveInteger(
        source.httpStreamWorkerFactor,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.httpStreamWorkerFactor,
        1,
      ),
      httpStreamWorkerMin: normalizePositiveInteger(
        source.httpStreamWorkerMin,
        DEFAULT_BACKGROUND_TASKS_SETTINGS.httpStreamWorkerMin,
        1,
      ),
    };
  }

  function normalizeAppSettingsSnapshot(source) {
    const payload = source && typeof source === "object" ? source : {};
    const defaults = buildDefaultAppSettingsSnapshot();
    let serviceAddr = defaults.serviceAddr;
    try {
      serviceAddr = normalizeAddr(payload.serviceAddr || defaults.serviceAddr);
    } catch {
      serviceAddr = defaults.serviceAddr;
    }
    return {
      updateAutoCheck: normalizeBooleanSetting(payload.updateAutoCheck, defaults.updateAutoCheck),
      closeToTrayOnClose: normalizeBooleanSetting(
        payload.closeToTrayOnClose,
        defaults.closeToTrayOnClose,
      ),
      closeToTraySupported: normalizeBooleanSetting(
        payload.closeToTraySupported,
        defaults.closeToTraySupported,
      ),
      lightweightModeOnCloseToTray: normalizeBooleanSetting(
        payload.lightweightModeOnCloseToTray,
        defaults.lightweightModeOnCloseToTray,
      ),
      lowTransparency: normalizeBooleanSetting(payload.lowTransparency, defaults.lowTransparency),
      theme: normalizeThemeSetting(payload.theme),
      serviceAddr,
      serviceListenMode: normalizeServiceListenMode(payload.serviceListenMode),
      routeStrategy: normalizeRouteStrategy(payload.routeStrategy),
      cpaNoCookieHeaderModeEnabled: normalizeCpaNoCookieHeaderMode(
        payload.cpaNoCookieHeaderModeEnabled,
      ),
      upstreamProxyUrl: normalizeUpstreamProxyUrl(payload.upstreamProxyUrl),
      backgroundTasks: normalizeBackgroundTasksSettings(payload.backgroundTasks),
      envOverrides: normalizeEnvOverrides(payload.envOverrides),
      envOverrideCatalog: normalizeEnvOverrideCatalog(payload.envOverrideCatalog),
      envOverrideReservedKeys: normalizeStringList(payload.envOverrideReservedKeys),
      envOverrideUnsupportedKeys: normalizeStringList(payload.envOverrideUnsupportedKeys),
      webAccessPasswordConfigured: normalizeBooleanSetting(
        payload.webAccessPasswordConfigured,
        defaults.webAccessPasswordConfigured,
      ),
    };
  }

  function getAppSettingsSnapshot() {
    return appSettingsSnapshot;
  }

  function setAppSettingsSnapshot(snapshot) {
    appSettingsSnapshot = normalizeAppSettingsSnapshot(snapshot);
    if (state && typeof state === "object") {
      state.serviceAddr = appSettingsSnapshot.serviceAddr;
    }
    return appSettingsSnapshot;
  }

  function patchAppSettingsSnapshot(patch = {}) {
    const next = {
      ...appSettingsSnapshot,
      ...(patch && typeof patch === "object" ? patch : {}),
    };
    if (patch && Object.prototype.hasOwnProperty.call(patch, "backgroundTasks")) {
      next.backgroundTasks = patch.backgroundTasks;
    }
    if (patch && Object.prototype.hasOwnProperty.call(patch, "envOverrides")) {
      next.envOverrides = patch.envOverrides;
    }
    if (patch && Object.prototype.hasOwnProperty.call(patch, "envOverrideCatalog")) {
      next.envOverrideCatalog = patch.envOverrideCatalog;
    }
    if (patch && Object.prototype.hasOwnProperty.call(patch, "envOverrideReservedKeys")) {
      next.envOverrideReservedKeys = patch.envOverrideReservedKeys;
    }
    if (patch && Object.prototype.hasOwnProperty.call(patch, "envOverrideUnsupportedKeys")) {
      next.envOverrideUnsupportedKeys = patch.envOverrideUnsupportedKeys;
    }
    return setAppSettingsSnapshot(next);
  }

  async function loadAppSettings() {
    try {
      return setAppSettingsSnapshot(await appSettingsGet());
    } catch (err) {
      console.warn("[app-settings] load failed", err);
      return setAppSettingsSnapshot(appSettingsSnapshot);
    }
  }

  async function saveAppSettingsPatch(patch = {}) {
    const payload = patch && typeof patch === "object" ? patch : {};
    return setAppSettingsSnapshot(await appSettingsSet(payload));
  }

  function applyBrowserModeUi() {
    if (isTauriRuntime()) {
      return false;
    }
    const doc = getDocumentRef();
    if (doc?.body) {
      doc.body.classList.add("cm-browser");
    }

    const serviceSetup = dom.serviceAddrInput ? dom.serviceAddrInput.closest(".service-setup") : null;
    if (serviceSetup) {
      serviceSetup.style.display = "none";
    }
    const updateCard = dom.checkUpdate
      ? dom.checkUpdate.closest(".settings-top-item, .settings-card")
      : null;
    if (updateCard) {
      updateCard.style.display = "none";
    }
    const closeToTrayCard = dom.closeToTrayOnClose
      ? dom.closeToTrayOnClose.closest(".settings-top-item, .settings-card")
      : null;
    if (closeToTrayCard) {
      closeToTrayCard.style.display = "none";
    }
    const lightweightModeCard = dom.lightweightModeOnCloseToTray
      ? dom.lightweightModeOnCloseToTray.closest(".settings-top-item, .settings-card")
      : null;
    if (lightweightModeCard) {
      lightweightModeCard.style.display = "none";
    }

    return true;
  }

  function readUpdateAutoCheckSetting() {
    return Boolean(appSettingsSnapshot.updateAutoCheck);
  }

  function saveUpdateAutoCheckSetting(enabled) {
    patchAppSettingsSnapshot({ updateAutoCheck: Boolean(enabled) });
  }

  function initUpdateAutoCheckSetting() {
    const enabled = readUpdateAutoCheckSetting();
    if (dom.autoCheckUpdate) {
      dom.autoCheckUpdate.checked = enabled;
    }
  }

  function readCloseToTrayOnCloseSetting() {
    return Boolean(appSettingsSnapshot.closeToTrayOnClose);
  }

  function saveCloseToTrayOnCloseSetting(enabled) {
    patchAppSettingsSnapshot({ closeToTrayOnClose: Boolean(enabled) });
  }

  function setCloseToTrayOnCloseToggle(enabled) {
    if (dom.closeToTrayOnClose) {
      dom.closeToTrayOnClose.checked = Boolean(enabled);
    }
  }

  function readLightweightModeOnCloseToTraySetting() {
    return Boolean(appSettingsSnapshot.lightweightModeOnCloseToTray);
  }

  function saveLightweightModeOnCloseToTraySetting(enabled) {
    patchAppSettingsSnapshot({ lightweightModeOnCloseToTray: Boolean(enabled) });
  }

  function setLightweightModeOnCloseToTrayToggle(enabled) {
    if (dom.lightweightModeOnCloseToTray) {
      dom.lightweightModeOnCloseToTray.checked = Boolean(enabled);
    }
  }

  function syncLightweightModeOnCloseToTrayAvailability() {
    if (!dom.lightweightModeOnCloseToTray) {
      return;
    }
    dom.lightweightModeOnCloseToTray.disabled = !Boolean(appSettingsSnapshot.closeToTraySupported)
      || !Boolean(appSettingsSnapshot.closeToTrayOnClose);
  }

  async function applyCloseToTrayOnCloseSetting(enabled, { silent = true } = {}) {
    const normalized = Boolean(enabled);
    try {
      const settings = await saveAppSettingsPatch({
        closeToTrayOnClose: normalized,
      });
      const applied = Boolean(settings.closeToTrayOnClose);
      const supported = Boolean(settings.closeToTraySupported);
      if (dom.closeToTrayOnClose) {
        dom.closeToTrayOnClose.disabled = !supported;
      }
      saveCloseToTrayOnCloseSetting(applied);
      setCloseToTrayOnCloseToggle(applied);
      syncLightweightModeOnCloseToTrayAvailability();
      if (!silent) {
        if (normalized && !applied && !supported) {
          showToast("系统托盘不可用，无法启用关闭时最小化到托盘", "error");
        } else {
          showToast(applied ? "已开启：关闭窗口将最小化到托盘" : "已关闭：关闭窗口将直接退出");
        }
      }
      return Boolean(applied);
    } catch (err) {
      if (!silent) {
        showToast(`设置失败：${normalizeErrorMessage(err)}`, "error");
      }
      throw err;
    }
  }

  function initCloseToTrayOnCloseSetting() {
    const enabled = readCloseToTrayOnCloseSetting();
    setCloseToTrayOnCloseToggle(enabled);
    if (dom.closeToTrayOnClose) {
      dom.closeToTrayOnClose.disabled = !Boolean(appSettingsSnapshot.closeToTraySupported);
    }
    syncLightweightModeOnCloseToTrayAvailability();
  }

  async function applyLightweightModeOnCloseToTraySetting(enabled, { silent = true } = {}) {
    const normalized = Boolean(enabled);
    try {
      const settings = await saveAppSettingsPatch({
        lightweightModeOnCloseToTray: normalized,
      });
      const applied = Boolean(settings.lightweightModeOnCloseToTray);
      saveLightweightModeOnCloseToTraySetting(applied);
      setLightweightModeOnCloseToTrayToggle(applied);
      syncLightweightModeOnCloseToTrayAvailability();
      if (!silent) {
        showToast(
          applied
            ? "已开启：关闭到托盘时会释放窗口内存，再次打开会稍慢"
            : "已关闭：托盘隐藏时继续保留窗口内存，再次打开更快",
        );
      }
      return applied;
    } catch (err) {
      if (!silent) {
        showToast(`设置失败：${normalizeErrorMessage(err)}`, "error");
      }
      throw err;
    }
  }

  function initLightweightModeOnCloseToTraySetting() {
    const enabled = readLightweightModeOnCloseToTraySetting();
    setLightweightModeOnCloseToTrayToggle(enabled);
    syncLightweightModeOnCloseToTrayAvailability();
  }

  function readLowTransparencySetting() {
    return Boolean(appSettingsSnapshot.lowTransparency);
  }

  function saveLowTransparencySetting(enabled) {
    patchAppSettingsSnapshot({ lowTransparency: Boolean(enabled) });
  }

  function applyLowTransparencySetting(enabled) {
    const doc = getDocumentRef();
    if (!doc?.body) {
      return;
    }
    doc.body.classList.toggle(UI_LOW_TRANSPARENCY_BODY_CLASS, enabled);
  }

  function ensureLowTransparencySettingCard() {
    const doc = getDocumentRef();
    if (!doc) {
      return null;
    }
    const existing = doc.getElementById(UI_LOW_TRANSPARENCY_TOGGLE_ID);
    if (existing) {
      return existing;
    }

    const settingsGrid = doc.querySelector("#pageSettings .settings-grid");
    if (!settingsGrid) {
      return null;
    }

    const existingCard = doc.getElementById(UI_LOW_TRANSPARENCY_CARD_ID);
    if (existingCard) {
      return doc.getElementById(UI_LOW_TRANSPARENCY_TOGGLE_ID);
    }

    const card = doc.createElement("div");
    card.className = "panel settings-card settings-card-span-2";
    card.id = UI_LOW_TRANSPARENCY_CARD_ID;
    card.innerHTML = `
    <div class="panel-header">
      <div>
        <h3>视觉性能</h3>
        <p>减少模糊/透明特效，降低掉帧</p>
      </div>
    </div>
    <div class="settings-row">
      <label class="update-auto-check switch-control" for="${UI_LOW_TRANSPARENCY_TOGGLE_ID}">
        <input id="${UI_LOW_TRANSPARENCY_TOGGLE_ID}" type="checkbox" />
        <span class="switch-track" aria-hidden="true">
          <span class="switch-thumb"></span>
        </span>
        <span>性能模式/低透明度</span>
      </label>
    </div>
    <div class="hint">开启后会关闭/降级 blur、backdrop-filter 等效果（更省 GPU，但质感会更“硬”）。</div>
  `;

    const themeCard = doc.getElementById("themePanel")?.closest(".settings-card");
    if (themeCard && themeCard.parentElement === settingsGrid) {
      settingsGrid.insertBefore(card, themeCard);
    } else {
      settingsGrid.appendChild(card);
    }

    return doc.getElementById(UI_LOW_TRANSPARENCY_TOGGLE_ID);
  }

  function initLowTransparencySetting() {
    const enabled = readLowTransparencySetting();
    applyLowTransparencySetting(enabled);
    const toggle = ensureLowTransparencySettingCard();
    if (toggle) {
      toggle.checked = enabled;
    }
  }

  function buildServiceListenModeHint(mode, requiresRestart = true) {
    const normalized = normalizeServiceListenMode(mode);
    const suffix = normalized === SERVICE_LISTEN_MODE_ALL_INTERFACES
      ? "局域网访问请使用本机实际 IP。"
      : "外部设备将无法直接访问。";
    if (requiresRestart) {
      return `已保存为${serviceListenModeLabel(normalized)}，重启服务后生效；${suffix}`;
    }
    return `当前为${serviceListenModeLabel(normalized)}；${suffix}`;
  }

  function setServiceListenModeSelect(mode) {
    if (!dom.serviceListenModeSelect) {
      return;
    }
    dom.serviceListenModeSelect.value = normalizeServiceListenMode(mode);
  }

  function setServiceListenModeHint(message) {
    if (!dom.serviceListenModeHint) {
      return;
    }
    dom.serviceListenModeHint.textContent = String(message || "").trim()
      || "保存后重启服务生效；局域网访问请使用本机实际 IP。";
  }

  function readServiceListenModeSetting() {
    return normalizeServiceListenMode(appSettingsSnapshot.serviceListenMode);
  }

  function initServiceListenModeSetting() {
    const mode = readServiceListenModeSetting();
    setServiceListenModeSelect(mode);
    setServiceListenModeHint(buildServiceListenModeHint(mode, true));
  }

  async function applyServiceListenModeToService(mode, { silent = true } = {}) {
    const normalized = normalizeServiceListenMode(mode);
    if (serviceListenModeSyncInFlight) {
      return serviceListenModeSyncInFlight;
    }
    serviceListenModeSyncInFlight = (async () => {
      const settings = await saveAppSettingsPatch({
        serviceListenMode: normalized,
      });
      const resolved = {
        mode: normalizeServiceListenMode(settings.serviceListenMode),
        requiresRestart: true,
      };
      setServiceListenModeSelect(resolved.mode);
      setServiceListenModeHint(buildServiceListenModeHint(resolved.mode, resolved.requiresRestart));
      if (!silent) {
        showToast(`监听模式已保存为${serviceListenModeLabel(resolved.mode)}，重启服务后生效`);
      }
      return true;
    })();

    try {
      return await serviceListenModeSyncInFlight;
    } catch (err) {
      if (!silent) {
        showToast(`保存失败：${normalizeErrorMessage(err)}`, "error");
        setServiceListenModeHint(`保存失败：${normalizeErrorMessage(err)}`);
      }
      return false;
    } finally {
      serviceListenModeSyncInFlight = null;
    }
  }

  async function syncServiceListenModeOnStartup() {
    initServiceListenModeSetting();
  }

  function updateRouteStrategyHint(strategy) {
    if (!dom.routeStrategyHint) {
      return;
    }
    let hintText = "按账号顺序优先请求，优先使用可用账号（不可用账号不会参与选路）。";
    if (normalizeRouteStrategy(strategy) === ROUTE_STRATEGY_BALANCED) {
      hintText = "按密钥 + 模型均衡轮询起点，优先使用可用账号（不可用账号不会参与选路）。";
    }
    dom.routeStrategyHint.title = hintText;
    dom.routeStrategyHint.setAttribute("aria-label", `网关选路策略说明：${hintText}`);
  }

  function readRouteStrategySetting() {
    return normalizeRouteStrategy(appSettingsSnapshot.routeStrategy);
  }

  function saveRouteStrategySetting(strategy) {
    patchAppSettingsSnapshot({
      routeStrategy: normalizeRouteStrategy(strategy),
    });
  }

  function setRouteStrategySelect(strategy) {
    const normalized = normalizeRouteStrategy(strategy);
    if (dom.routeStrategySelect) {
      dom.routeStrategySelect.value = normalized;
    }
    updateRouteStrategyHint(normalized);
  }

  function initRouteStrategySetting() {
    const mode = readRouteStrategySetting();
    setRouteStrategySelect(mode);
  }

  function readCpaNoCookieHeaderModeSetting() {
    return normalizeCpaNoCookieHeaderMode(appSettingsSnapshot.cpaNoCookieHeaderModeEnabled);
  }

  function saveCpaNoCookieHeaderModeSetting(enabled) {
    patchAppSettingsSnapshot({
      cpaNoCookieHeaderModeEnabled: normalizeCpaNoCookieHeaderMode(enabled),
    });
  }

  function setCpaNoCookieHeaderModeToggle(enabled) {
    if (dom.cpaNoCookieHeaderMode) {
      dom.cpaNoCookieHeaderMode.checked = Boolean(enabled);
    }
  }

  function initCpaNoCookieHeaderModeSetting() {
    const enabled = readCpaNoCookieHeaderModeSetting();
    setCpaNoCookieHeaderModeToggle(enabled);
  }

  function readUpstreamProxyUrlSetting() {
    return normalizeUpstreamProxyUrl(appSettingsSnapshot.upstreamProxyUrl);
  }

  function saveUpstreamProxyUrlSetting(value) {
    patchAppSettingsSnapshot({
      upstreamProxyUrl: normalizeUpstreamProxyUrl(value),
    });
  }

  function setUpstreamProxyInput(value) {
    if (!dom.upstreamProxyUrlInput) {
      return;
    }
    dom.upstreamProxyUrlInput.value = normalizeUpstreamProxyUrl(value);
  }

  function setUpstreamProxyHint(message) {
    if (!dom.upstreamProxyHint) {
      return;
    }
    dom.upstreamProxyHint.textContent = message;
  }

  function initUpstreamProxySetting() {
    const proxyUrl = readUpstreamProxyUrlSetting();
    setUpstreamProxyInput(proxyUrl);
    setUpstreamProxyHint(UPSTREAM_PROXY_HINT_TEXT);
  }

  function readBackgroundTasksSetting() {
    return normalizeBackgroundTasksSettings(appSettingsSnapshot.backgroundTasks);
  }

  function saveBackgroundTasksSetting(settings) {
    patchAppSettingsSnapshot({
      backgroundTasks: normalizeBackgroundTasksSettings(settings),
    });
  }

  function setBackgroundTasksForm(settings) {
    const normalized = normalizeBackgroundTasksSettings(settings);
    if (dom.backgroundUsagePollingEnabled) {
      dom.backgroundUsagePollingEnabled.checked = normalized.usagePollingEnabled;
    }
    if (dom.backgroundUsagePollIntervalSecs) {
      dom.backgroundUsagePollIntervalSecs.value = String(normalized.usagePollIntervalSecs);
    }
    if (dom.backgroundGatewayKeepaliveEnabled) {
      dom.backgroundGatewayKeepaliveEnabled.checked = normalized.gatewayKeepaliveEnabled;
    }
    if (dom.backgroundGatewayKeepaliveIntervalSecs) {
      dom.backgroundGatewayKeepaliveIntervalSecs.value = String(normalized.gatewayKeepaliveIntervalSecs);
    }
    if (dom.backgroundTokenRefreshPollingEnabled) {
      dom.backgroundTokenRefreshPollingEnabled.checked = normalized.tokenRefreshPollingEnabled;
    }
    if (dom.backgroundTokenRefreshPollIntervalSecs) {
      dom.backgroundTokenRefreshPollIntervalSecs.value = String(normalized.tokenRefreshPollIntervalSecs);
    }
    if (dom.backgroundUsageRefreshWorkers) {
      dom.backgroundUsageRefreshWorkers.value = String(normalized.usageRefreshWorkers);
    }
    if (dom.backgroundHttpWorkerFactor) {
      dom.backgroundHttpWorkerFactor.value = String(normalized.httpWorkerFactor);
    }
    if (dom.backgroundHttpWorkerMin) {
      dom.backgroundHttpWorkerMin.value = String(normalized.httpWorkerMin);
    }
    if (dom.backgroundHttpStreamWorkerFactor) {
      dom.backgroundHttpStreamWorkerFactor.value = String(normalized.httpStreamWorkerFactor);
    }
    if (dom.backgroundHttpStreamWorkerMin) {
      dom.backgroundHttpStreamWorkerMin.value = String(normalized.httpStreamWorkerMin);
    }
  }

  function readBackgroundTasksForm() {
    const integerFields = [
      ["usagePollIntervalSecs", dom.backgroundUsagePollIntervalSecs, "用量轮询间隔"],
      ["gatewayKeepaliveIntervalSecs", dom.backgroundGatewayKeepaliveIntervalSecs, "网关保活间隔"],
      ["tokenRefreshPollIntervalSecs", dom.backgroundTokenRefreshPollIntervalSecs, "令牌刷新间隔"],
      ["usageRefreshWorkers", dom.backgroundUsageRefreshWorkers, "用量刷新线程数"],
      ["httpWorkerFactor", dom.backgroundHttpWorkerFactor, "普通请求线程因子"],
      ["httpWorkerMin", dom.backgroundHttpWorkerMin, "普通请求最小线程数"],
      ["httpStreamWorkerFactor", dom.backgroundHttpStreamWorkerFactor, "流式请求线程因子"],
      ["httpStreamWorkerMin", dom.backgroundHttpStreamWorkerMin, "流式请求最小线程数"],
    ];
    const numbers = {};
    for (const [key, input, label] of integerFields) {
      const raw = input ? String(input.value || "").trim() : "";
      const parsed = Number(raw);
      if (!Number.isFinite(parsed) || parsed <= 0 || Math.floor(parsed) !== parsed) {
        return { ok: false, error: `${label} 需填写正整数` };
      }
      numbers[key] = parsed;
    }
    return {
      ok: true,
      settings: normalizeBackgroundTasksSettings({
        usagePollingEnabled: dom.backgroundUsagePollingEnabled
          ? Boolean(dom.backgroundUsagePollingEnabled.checked)
          : DEFAULT_BACKGROUND_TASKS_SETTINGS.usagePollingEnabled,
        usagePollIntervalSecs: numbers.usagePollIntervalSecs,
        gatewayKeepaliveEnabled: dom.backgroundGatewayKeepaliveEnabled
          ? Boolean(dom.backgroundGatewayKeepaliveEnabled.checked)
          : DEFAULT_BACKGROUND_TASKS_SETTINGS.gatewayKeepaliveEnabled,
        gatewayKeepaliveIntervalSecs: numbers.gatewayKeepaliveIntervalSecs,
        tokenRefreshPollingEnabled: dom.backgroundTokenRefreshPollingEnabled
          ? Boolean(dom.backgroundTokenRefreshPollingEnabled.checked)
          : DEFAULT_BACKGROUND_TASKS_SETTINGS.tokenRefreshPollingEnabled,
        tokenRefreshPollIntervalSecs: numbers.tokenRefreshPollIntervalSecs,
        usageRefreshWorkers: numbers.usageRefreshWorkers,
        httpWorkerFactor: numbers.httpWorkerFactor,
        httpWorkerMin: numbers.httpWorkerMin,
        httpStreamWorkerFactor: numbers.httpStreamWorkerFactor,
        httpStreamWorkerMin: numbers.httpStreamWorkerMin,
      }),
    };
  }

  function updateBackgroundTasksHint(requiresRestartKeys) {
    if (!dom.backgroundTasksHint) {
      return;
    }
    const keys = Array.isArray(requiresRestartKeys) ? requiresRestartKeys : [];
    if (keys.length === 0) {
      dom.backgroundTasksHint.textContent = "保存后立即生效。";
      return;
    }
    const labels = keys.map((key) => BACKGROUND_TASKS_RESTART_KEY_LABELS[key] || key);
    dom.backgroundTasksHint.textContent = `已保存。以下参数需重启服务生效：${labels.join("、")}。`;
  }

  function initBackgroundTasksSetting() {
    const settings = readBackgroundTasksSetting();
    setBackgroundTasksForm(settings);
    updateBackgroundTasksHint(BACKGROUND_TASKS_RESTART_KEYS_DEFAULT);
  }

  function readEnvOverridesSetting() {
    return normalizeEnvOverrides(appSettingsSnapshot.envOverrides);
  }

  function saveEnvOverridesSetting(value) {
    patchAppSettingsSnapshot({
      envOverrides: normalizeEnvOverrides(value),
    });
  }

  function setEnvOverridesHint(message) {
    if (!dom.envOverridesHint) {
      return;
    }
    dom.envOverridesHint.textContent = String(message || "").trim()
      || "选择变量后可直接修改值；恢复默认会回退到启动时环境值或内置默认值。";
  }

  function setEnvOverrideDescription(message) {
    if (!dom.envOverrideDescription) {
      return;
    }
    dom.envOverrideDescription.textContent = String(message || "").trim()
      || "这里会显示当前变量的作用说明。";
  }

  function readEnvOverrideCatalog() {
    return normalizeEnvOverrideCatalog(appSettingsSnapshot.envOverrideCatalog);
  }

  function findEnvOverrideCatalogItem(key, catalog = readEnvOverrideCatalog()) {
    const normalizedKey = String(key || "").trim().toUpperCase();
    return catalog.find((item) => item.key === normalizedKey) || null;
  }

  function resolveEnvOverrideSelection(preferredKey) {
    const catalog = filterEnvOverrideCatalog(
      readEnvOverrideCatalog(),
      dom.envOverrideSearchInput ? dom.envOverrideSearchInput.value : "",
    );
    const nextKey = [preferredKey, envOverrideSelectedKey]
      .map((item) => String(item || "").trim().toUpperCase())
      .find((key) => key && catalog.some((item) => item.key === key))
      || (catalog[0] ? catalog[0].key : "");

    envOverrideSelectedKey = nextKey;
    return {
      catalog,
      selectedItem: catalog.find((item) => item.key === nextKey) || null,
    };
  }

  function buildEnvOverrideHint(item, currentValue, prefix = "") {
    if (!item) {
      return prefix || "请输入搜索词并从下拉中选择一个变量。";
    }
    const scopeLabel = item.scope === "web"
      ? "Web"
      : item.scope === "desktop"
        ? "桌面端"
        : "服务端";
    const parts = [];
    if (prefix) {
      parts.push(prefix);
    }
    parts.push(`默认值：${formatEnvOverrideDisplayValue(item.defaultValue)}`);
    parts.push(`当前值：${formatEnvOverrideDisplayValue(currentValue)}`);
    parts.push(`作用域：${scopeLabel}`);
    parts.push(item.applyMode === "restart" ? "保存后需重启相关进程" : "保存后热生效");
    return parts.join("；");
  }

  function renderEnvOverrideSelector(preferredKey = envOverrideSelectedKey) {
    const { catalog, selectedItem } = resolveEnvOverrideSelection(preferredKey);
    if (!dom.envOverrideSelect) {
      return selectedItem;
    }

    const doc = getDocumentRef();
    if (!doc) {
      return selectedItem;
    }

    dom.envOverrideSelect.replaceChildren();
    if (catalog.length === 0) {
      const option = doc.createElement("option");
      option.value = "";
      option.textContent = "未匹配到变量";
      dom.envOverrideSelect.appendChild(option);
      dom.envOverrideSelect.disabled = true;
      dom.envOverrideSelect.value = "";
      return null;
    }

    for (const item of catalog) {
      const option = doc.createElement("option");
      option.value = item.key;
      option.textContent = buildEnvOverrideOptionLabel(item);
      dom.envOverrideSelect.appendChild(option);
    }
    dom.envOverrideSelect.disabled = false;
    dom.envOverrideSelect.value = selectedItem ? selectedItem.key : catalog[0].key;
    return selectedItem;
  }

  function renderEnvOverrideEditor(preferredKey = envOverrideSelectedKey, hint = "") {
    const item = renderEnvOverrideSelector(preferredKey);
    const overrides = readEnvOverridesSetting();
    const currentValue = item ? (overrides[item.key] ?? item.defaultValue ?? "") : "";

    if (dom.envOverrideNameValue) {
      dom.envOverrideNameValue.textContent = item ? item.label : "未选择";
    }
    if (dom.envOverrideKeyValue) {
      dom.envOverrideKeyValue.textContent = item ? item.key : "-";
    }
    if (dom.envOverrideMeta) {
      const scopeLabel = item?.scope === "web"
        ? "Web"
        : item?.scope === "desktop"
          ? "桌面端"
          : "服务端";
      dom.envOverrideMeta.textContent = item
        ? `${scopeLabel} · ${item.applyMode === "restart" ? "重启生效" : "热生效"}`
        : "请先选择变量";
    }
    if (dom.envOverrideValueInput) {
      dom.envOverrideValueInput.disabled = !item;
      dom.envOverrideValueInput.value = item ? currentValue : "";
      dom.envOverrideValueInput.placeholder = item
        ? "留空并保存可恢复默认值"
        : "请先选择变量";
    }
    if (dom.envOverridesSave) {
      dom.envOverridesSave.disabled = !item;
    }
    if (dom.envOverrideReset) {
      dom.envOverrideReset.disabled = !item;
    }

    setEnvOverridesHint(hint || buildEnvOverrideHint(item, currentValue));
    setEnvOverrideDescription(buildEnvOverrideDescription(item));
    return item;
  }

  function initEnvOverridesSetting() {
    envOverrideSelectedKey = "";
    renderEnvOverrideEditor("", "选择变量后可直接修改值；恢复默认会回退到启动时环境值或内置默认值。");
  }

  function buildWebAccessPasswordStatusText(configured) {
    return configured
      ? "当前已启用 Web 访问密码。修改后会立即覆盖旧密码。"
      : "当前未启用 Web 访问密码。";
  }

  function updateWebAccessPasswordState(configured) {
    const enabled = Boolean(configured);
    patchAppSettingsSnapshot({ webAccessPasswordConfigured: enabled });
    const text = buildWebAccessPasswordStatusText(enabled);
    if (dom.webAccessPasswordHint) {
      dom.webAccessPasswordHint.textContent = text;
    }
    if (dom.webAccessPasswordQuickStatus) {
      dom.webAccessPasswordQuickStatus.textContent = text;
    }
  }

  function readWebAccessPasswordPair(source = "settings") {
    const useQuick = source === "quick";
    const password = useQuick
      ? (dom.webAccessPasswordQuickInput ? dom.webAccessPasswordQuickInput.value : "")
      : (dom.webAccessPasswordInput ? dom.webAccessPasswordInput.value : "");
    const confirm = useQuick
      ? (dom.webAccessPasswordQuickConfirm ? dom.webAccessPasswordQuickConfirm.value : "")
      : (dom.webAccessPasswordConfirm ? dom.webAccessPasswordConfirm.value : "");
    return {
      password: String(password || ""),
      confirm: String(confirm || ""),
    };
  }

  function syncWebAccessPasswordInputs(source = "settings") {
    const pair = readWebAccessPasswordPair(source);
    if (dom.webAccessPasswordInput) {
      dom.webAccessPasswordInput.value = pair.password;
    }
    if (dom.webAccessPasswordConfirm) {
      dom.webAccessPasswordConfirm.value = pair.confirm;
    }
    if (dom.webAccessPasswordQuickInput) {
      dom.webAccessPasswordQuickInput.value = pair.password;
    }
    if (dom.webAccessPasswordQuickConfirm) {
      dom.webAccessPasswordQuickConfirm.value = pair.confirm;
    }
  }

  function clearWebAccessPasswordInputs() {
    if (dom.webAccessPasswordInput) {
      dom.webAccessPasswordInput.value = "";
    }
    if (dom.webAccessPasswordConfirm) {
      dom.webAccessPasswordConfirm.value = "";
    }
    if (dom.webAccessPasswordQuickInput) {
      dom.webAccessPasswordQuickInput.value = "";
    }
    if (dom.webAccessPasswordQuickConfirm) {
      dom.webAccessPasswordQuickConfirm.value = "";
    }
  }

  function openWebSecurityModal() {
    if (!dom.modalWebSecurity) {
      return;
    }
    syncWebAccessPasswordInputs("settings");
    updateWebAccessPasswordState(appSettingsSnapshot.webAccessPasswordConfigured);
    dom.modalWebSecurity.classList.add("active");
  }

  function closeWebSecurityModal() {
    if (!dom.modalWebSecurity) {
      return;
    }
    dom.modalWebSecurity.classList.remove("active");
  }

  async function saveWebAccessPassword(source = "settings") {
    const pair = readWebAccessPasswordPair(source);
    const password = pair.password.trim();
    if (!password) {
      showToast("请输入 Web 访问密码；如需关闭保护请点击清除", "error");
      return false;
    }
    if (pair.password !== pair.confirm) {
      showToast("两次输入的 Web 访问密码不一致", "error");
      return false;
    }
    try {
      const settings = await saveAppSettingsPatch({
        webAccessPassword: pair.password,
      });
      updateWebAccessPasswordState(settings.webAccessPasswordConfigured);
      clearWebAccessPasswordInputs();
      if (source === "quick") {
        closeWebSecurityModal();
      }
      showToast("Web 访问密码已保存");
      return true;
    } catch (err) {
      showToast(`保存失败：${normalizeErrorMessage(err)}`, "error");
      return false;
    }
  }

  async function clearWebAccessPassword(source = "settings") {
    try {
      const settings = await saveAppSettingsPatch({
        webAccessPassword: "",
      });
      updateWebAccessPasswordState(settings.webAccessPasswordConfigured);
      clearWebAccessPasswordInputs();
      if (source === "quick") {
        closeWebSecurityModal();
      }
      showToast("Web 访问密码已清除");
      return true;
    } catch (err) {
      showToast(`清除失败：${normalizeErrorMessage(err)}`, "error");
      return false;
    }
  }

  async function persistServiceAddrInput({ silent = true } = {}) {
    if (!dom.serviceAddrInput) {
      return false;
    }
    let normalized = "";
    try {
      normalized = normalizeAddr(dom.serviceAddrInput.value || "");
    } catch (err) {
      if (!silent) {
        showToast(`服务地址格式不正确：${normalizeErrorMessage(err)}`, "error");
      }
      return false;
    }
    dom.serviceAddrInput.value = normalized;
    state.serviceAddr = normalized;
    patchAppSettingsSnapshot({ serviceAddr: normalized });
    try {
      await saveAppSettingsPatch({
        serviceAddr: normalized,
      });
      return true;
    } catch (err) {
      if (!silent) {
        showToast(`保存服务地址失败：${normalizeErrorMessage(err)}`, "error");
      }
      return false;
    }
  }

  return {
    loadAppSettings,
    saveAppSettingsPatch,
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
    getEnvOverrideSelectedKey: () => envOverrideSelectedKey,
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
    uiLowTransparencyToggleId: UI_LOW_TRANSPARENCY_TOGGLE_ID,
    upstreamProxyHintText: UPSTREAM_PROXY_HINT_TEXT,
    backgroundTasksRestartKeysDefault: BACKGROUND_TASKS_RESTART_KEYS_DEFAULT,
  };
}
