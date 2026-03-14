import type {
  AppSettingsSnapshot,
  BackgroundTasksSettings,
  EnvOverrideCatalogItem,
  GatewayTransportSettings,
  ThemeId,
} from "@/lib/codex/types";
import { normalizeEnvOverrideCatalogItem } from "@/lib/codex/env-overrides";
import { normalizeServiceAddress } from "@/lib/codex/service-address";

const DEFAULT_BACKGROUND_TASKS_SETTINGS: BackgroundTasksSettings = {
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

const DEFAULT_GATEWAY_TRANSPORT_SETTINGS: GatewayTransportSettings = {
  sseKeepaliveIntervalMs: 15_000,
  upstreamStreamTimeoutMs: 1_800_000,
};

function normalizeBoolean(value: unknown, fallback = false) {
  if (value == null) {
    return fallback;
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
  return fallback;
}

function normalizePositiveInteger(value: unknown, fallback: number, min = 1) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  const normalized = Math.floor(parsed);
  if (normalized < min) {
    return min;
  }
  return normalized;
}

function normalizeTheme(value: unknown): ThemeId {
  const normalized = String(value || "").trim().toLowerCase() as ThemeId;
  return normalized || "tech";
}

function normalizeStringArray(value: unknown) {
  if (!Array.isArray(value)) {
    return [] as string[];
  }
  return value.map((item) => String(item || "").trim()).filter(Boolean);
}

function normalizeEnvOverrideCatalog(value: unknown) {
  if (!Array.isArray(value)) {
    return [] as EnvOverrideCatalogItem[];
  }

  return value
    .map<EnvOverrideCatalogItem | null>((item) => normalizeEnvOverrideCatalogItem(item))
    .filter((item): item is EnvOverrideCatalogItem => item !== null);
}

function normalizeEnvOverrides(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {} as Record<string, string>;
  }
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [key, String(item ?? "")]),
  );
}

export function createDefaultSettingsSnapshot(): AppSettingsSnapshot {
  return {
    updateAutoCheck: true,
    closeToTrayOnClose: false,
    closeToTraySupported: false,
    lightweightModeOnCloseToTray: false,
    lowTransparency: false,
    theme: "tech",
    serviceAddr: "localhost:48760",
    serviceListenMode: "loopback",
    routeStrategy: "balanced",
    cpaNoCookieHeaderModeEnabled: false,
    upstreamProxyUrl: "",
    ...DEFAULT_GATEWAY_TRANSPORT_SETTINGS,
    backgroundTasks: { ...DEFAULT_BACKGROUND_TASKS_SETTINGS },
    envOverrides: {},
    envOverrideCatalog: [],
    envOverrideReservedKeys: [],
    envOverrideUnsupportedKeys: [],
    webAccessPasswordConfigured: false,
  };
}

export function normalizeSettingsSnapshot(source: unknown): AppSettingsSnapshot {
  const defaults = createDefaultSettingsSnapshot();
  const payload = source && typeof source === "object" ? (source as Record<string, unknown>) : {};

  let serviceAddr = defaults.serviceAddr;
  try {
    serviceAddr = normalizeServiceAddress(String(payload.serviceAddr || defaults.serviceAddr));
  } catch {
    serviceAddr = defaults.serviceAddr;
  }

  return {
    updateAutoCheck: normalizeBoolean(payload.updateAutoCheck, defaults.updateAutoCheck),
    closeToTrayOnClose: normalizeBoolean(payload.closeToTrayOnClose, defaults.closeToTrayOnClose),
    closeToTraySupported: normalizeBoolean(payload.closeToTraySupported, defaults.closeToTraySupported),
    lightweightModeOnCloseToTray: normalizeBoolean(
      payload.lightweightModeOnCloseToTray,
      defaults.lightweightModeOnCloseToTray,
    ),
    lowTransparency: normalizeBoolean(payload.lowTransparency, defaults.lowTransparency),
    theme: normalizeTheme(payload.theme),
    serviceAddr,
    serviceListenMode: String(payload.serviceListenMode || defaults.serviceListenMode),
    routeStrategy: String(payload.routeStrategy || defaults.routeStrategy),
    cpaNoCookieHeaderModeEnabled: normalizeBoolean(
      payload.cpaNoCookieHeaderModeEnabled,
      defaults.cpaNoCookieHeaderModeEnabled,
    ),
    upstreamProxyUrl: String(payload.upstreamProxyUrl || ""),
    sseKeepaliveIntervalMs: normalizePositiveInteger(
      payload.sseKeepaliveIntervalMs,
      defaults.sseKeepaliveIntervalMs,
      1,
    ),
    upstreamStreamTimeoutMs: normalizePositiveInteger(
      payload.upstreamStreamTimeoutMs,
      defaults.upstreamStreamTimeoutMs,
      0,
    ),
    backgroundTasks: {
      usagePollingEnabled: normalizeBoolean(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.usagePollingEnabled,
        defaults.backgroundTasks.usagePollingEnabled,
      ),
      usagePollIntervalSecs: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.usagePollIntervalSecs,
        defaults.backgroundTasks.usagePollIntervalSecs,
        1,
      ),
      gatewayKeepaliveEnabled: normalizeBoolean(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.gatewayKeepaliveEnabled,
        defaults.backgroundTasks.gatewayKeepaliveEnabled,
      ),
      gatewayKeepaliveIntervalSecs: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.gatewayKeepaliveIntervalSecs,
        defaults.backgroundTasks.gatewayKeepaliveIntervalSecs,
        1,
      ),
      tokenRefreshPollingEnabled: normalizeBoolean(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.tokenRefreshPollingEnabled,
        defaults.backgroundTasks.tokenRefreshPollingEnabled,
      ),
      tokenRefreshPollIntervalSecs: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.tokenRefreshPollIntervalSecs,
        defaults.backgroundTasks.tokenRefreshPollIntervalSecs,
        1,
      ),
      usageRefreshWorkers: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.usageRefreshWorkers,
        defaults.backgroundTasks.usageRefreshWorkers,
        1,
      ),
      httpWorkerFactor: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.httpWorkerFactor,
        defaults.backgroundTasks.httpWorkerFactor,
        1,
      ),
      httpWorkerMin: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.httpWorkerMin,
        defaults.backgroundTasks.httpWorkerMin,
        1,
      ),
      httpStreamWorkerFactor: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.httpStreamWorkerFactor,
        defaults.backgroundTasks.httpStreamWorkerFactor,
        1,
      ),
      httpStreamWorkerMin: normalizePositiveInteger(
        (payload.backgroundTasks as Record<string, unknown> | undefined)?.httpStreamWorkerMin,
        defaults.backgroundTasks.httpStreamWorkerMin,
        1,
      ),
    },
    envOverrides: normalizeEnvOverrides(payload.envOverrides),
    envOverrideCatalog: normalizeEnvOverrideCatalog(payload.envOverrideCatalog),
    envOverrideReservedKeys: normalizeStringArray(payload.envOverrideReservedKeys),
    envOverrideUnsupportedKeys: normalizeStringArray(payload.envOverrideUnsupportedKeys),
    webAccessPasswordConfigured: normalizeBoolean(
      payload.webAccessPasswordConfigured,
      defaults.webAccessPasswordConfigured,
    ),
  };
}

export function toPositiveInt(value: string, label: string, min = 1) {
  const normalizedValue = value.trim();
  if (!normalizedValue) {
    throw new Error(`${label} 需填写${min > 0 ? "正整数" : "非负整数"}`);
  }
  const parsed = Number(normalizedValue);
  if (!Number.isFinite(parsed) || parsed < min || Math.floor(parsed) !== parsed) {
    throw new Error(`${label} 需填写${min > 0 ? "正整数" : "非负整数"}`);
  }
  return parsed;
}
