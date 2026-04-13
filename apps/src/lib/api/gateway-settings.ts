function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function readStringField(payload: unknown, key: string, fallback = ""): string {
  const source = asRecord(payload);
  const value = source?.[key];
  return typeof value === "string" ? value.trim() : fallback;
}

function readBooleanField(
  payload: unknown,
  key: string,
  fallback = false
): boolean {
  const source = asRecord(payload);
  const value = source?.[key];
  return typeof value === "boolean" ? value : fallback;
}

function readNumberField(payload: unknown, key: string, fallback = 0): number {
  const source = asRecord(payload);
  const value = source?.[key];
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string") {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return fallback;
}

function readStringArrayField(payload: unknown, key: string): string[] {
  const source = asRecord(payload);
  const value = source?.[key];
  return Array.isArray(value)
    ? value
        .map((item) => (typeof item === "string" ? item.trim() : ""))
        .filter(Boolean)
    : [];
}

export interface GatewayTransportSettings {
  sseKeepaliveIntervalMs: number;
  upstreamStreamTimeoutMs: number;
  envKeys: string[];
  requiresRestart: boolean;
}

export interface GatewayUpstreamProxySettings {
  proxyUrl: string;
  envKey: string;
  requiresRestart: boolean;
}

export interface GatewayRouteStrategySettings {
  strategy: string;
  options: string[];
  manualPreferredAccountId: string;
}

export function readGatewayTransportSettings(
  payload: unknown
): GatewayTransportSettings {
  return {
    sseKeepaliveIntervalMs: readNumberField(payload, "sseKeepaliveIntervalMs", 15_000),
    upstreamStreamTimeoutMs: readNumberField(
      payload,
      "upstreamStreamTimeoutMs",
      600_000
    ),
    envKeys:
      readStringArrayField(payload, "envKeys").length > 0
        ? readStringArrayField(payload, "envKeys")
        : [
            "CODEXMANAGER_SSE_KEEPALIVE_INTERVAL_MS",
            "CODEXMANAGER_UPSTREAM_STREAM_TIMEOUT_MS",
          ],
    requiresRestart: readBooleanField(payload, "requiresRestart", false),
  };
}

export function readGatewayUpstreamProxySettings(
  payload: unknown
): GatewayUpstreamProxySettings {
  return {
    proxyUrl: readStringField(payload, "proxyUrl"),
    envKey: readStringField(
      payload,
      "envKey",
      "CODEXMANAGER_UPSTREAM_PROXY_URL"
    ),
    requiresRestart: readBooleanField(payload, "requiresRestart", false),
  };
}

export function readGatewayRouteStrategySettings(
  payload: unknown
): GatewayRouteStrategySettings {
  return {
    strategy: readStringField(payload, "strategy", "ordered"),
    options:
      readStringArrayField(payload, "options").length > 0
        ? readStringArrayField(payload, "options")
        : ["ordered", "balanced"],
    manualPreferredAccountId: readStringField(payload, "manualPreferredAccountId"),
  };
}
