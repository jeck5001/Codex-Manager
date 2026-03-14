import type { RequestLogRecord } from "@/lib/codex/types";

export const REASONING_OPTIONS = [
  { value: "", label: "跟随请求等级" },
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
  { value: "xhigh", label: "极高" },
] as const;

export const PROTOCOL_OPTIONS = [
  { value: "openai_compat", label: "OpenAI 兼容" },
  { value: "anthropic_native", label: "Claude Code 兼容" },
  { value: "azure_openai", label: "Azure OpenAI 兼容" },
] as const;

export function getProtocolLabel(protocolType: string | null | undefined) {
  if (protocolType === "azure_openai") {
    return "Azure OpenAI 兼容";
  }
  if (protocolType === "anthropic_native") {
    return "Claude Code 兼容";
  }
  return "OpenAI 兼容";
}

export function getApiKeyStatusMeta(status: string | null | undefined) {
  const normalizedStatus = String(status || "").trim().toLowerCase();
  if (normalizedStatus === "active") {
    return { label: "启用", tone: "ok", disabled: false } as const;
  }
  if (normalizedStatus === "disabled") {
    return { label: "禁用", tone: "bad", disabled: true } as const;
  }
  return { label: status || "未知", tone: "unknown", disabled: false } as const;
}

export function getRequestStatusTone(statusCode: number | null | undefined) {
  if (!statusCode) {
    return "unknown" as const;
  }
  if (statusCode >= 200 && statusCode < 300) {
    return "ok" as const;
  }
  if (statusCode >= 400 && statusCode < 500) {
    return "warn" as const;
  }
  if (statusCode >= 500) {
    return "bad" as const;
  }
  return "unknown" as const;
}

export function isMissingCommandError(error: unknown) {
  const message = String(error instanceof Error ? error.message : error || "").toLowerCase();
  return (
    message.includes("not found")
    || message.includes("unknown command")
    || message.includes("no such command")
    || message.includes("does not exist")
    || message.includes("not managed")
    || message.includes("unknown_method")
    || (message.includes("invalid args") && message.includes("for command"))
  );
}

export async function copyText(value: string) {
  const text = String(value || "");
  if (!text.trim()) {
    return false;
  }

  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      return false;
    }
  }

  return false;
}

function pickImportTokenField(record: Record<string, unknown>, keys: string[]) {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return "";
}

function normalizeSingleImportRecord(record: unknown) {
  if (!record || typeof record !== "object" || Array.isArray(record)) {
    return record;
  }

  const normalizedRecord = record as Record<string, unknown>;
  const tokens = normalizedRecord.tokens;
  if (tokens && typeof tokens === "object" && !Array.isArray(tokens)) {
    return record;
  }

  const accessToken = pickImportTokenField(normalizedRecord, ["access_token", "accessToken"]);
  const idToken = pickImportTokenField(normalizedRecord, ["id_token", "idToken"]);
  const refreshToken = pickImportTokenField(normalizedRecord, ["refresh_token", "refreshToken"]);
  if (!accessToken || !idToken || !refreshToken) {
    return record;
  }

  const accountId = pickImportTokenField(normalizedRecord, [
    "account_id",
    "accountId",
    "chatgpt_account_id",
    "chatgptAccountId",
  ]);

  return {
    ...normalizedRecord,
    tokens: {
      access_token: accessToken,
      id_token: idToken,
      refresh_token: refreshToken,
      ...(accountId ? { account_id: accountId } : {}),
    },
  };
}

export function normalizeImportContentForCompatibility(rawContent: string) {
  const text = String(rawContent || "").trim();
  if (!text) {
    return text;
  }

  try {
    const parsed = JSON.parse(text) as unknown;
    if (Array.isArray(parsed)) {
      return JSON.stringify(parsed.map((item) => normalizeSingleImportRecord(item)));
    }
    if (parsed && typeof parsed === "object") {
      return JSON.stringify(normalizeSingleImportRecord(parsed));
    }
    return text;
  } catch {
    return text;
  }
}

export function parseLoginCallbackUrl(raw: string) {
  const value = String(raw || "").trim();
  if (!value) {
    return { error: "请粘贴回调链接" } as const;
  }

  let url: URL;
  try {
    url = new URL(value);
  } catch {
    try {
      url = new URL(`http://${value}`);
    } catch {
      return { error: "回调链接格式不正确" } as const;
    }
  }

  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  if (!code || !state) {
    return { error: "回调链接缺少 code/state" } as const;
  }

  return {
    payload: {
      code,
      state,
      redirectUri: `${url.origin}${url.pathname}`,
    },
  } as const;
}

export function resolveRequestLogAccountLabel(item: RequestLogRecord, accountLabelById: Map<string, string>) {
  const directLabel = String(item.accountLabel || "").trim();
  if (directLabel) {
    return directLabel;
  }

  const accountId = String(item.accountId || "").trim();
  if (accountId) {
    return accountLabelById.get(accountId) || fallbackAccountLabelFromId(accountId);
  }

  const keyId = String(item.keyId || "").trim();
  if (!keyId) {
    return "-";
  }
  return `Key ${keyId.slice(0, 10)}`;
}

export function resolveRequestDisplayPath(item: RequestLogRecord) {
  return String(item.originalPath || item.requestPath || item.mappedPath || item.path || item.adaptedPath || "-").trim() || "-";
}

function sanitizeUpstreamDisplay(upstreamUrl: string | null | undefined) {
  const raw = String(upstreamUrl || "").trim().toLowerCase();
  if (!raw) {
    return "";
  }
  if (
    raw.includes("localhost")
    || raw.includes("127.0.0.1")
    || raw.includes("0.0.0.0")
    || raw.includes("[::1]")
  ) {
    return "本地";
  }
  if (
    raw.includes("chatgpt.com")
    || raw.includes("chat.openai.com")
    || raw.includes("api.openai.com")
    || raw.includes("/backend-api/codex")
  ) {
    return "默认";
  }
  return "自定义";
}

export function buildRequestRouteMeta(item: RequestLogRecord) {
  const parts: string[] = [];
  const displayPath = resolveRequestDisplayPath(item);
  const adaptedPath = String(item.adaptedPath || "").trim();
  if (adaptedPath && adaptedPath !== displayPath) {
    parts.push(`转发 ${adaptedPath}`);
  }

  const responseAdapter = String(item.responseAdapter || "").trim();
  if (responseAdapter) {
    parts.push(`适配 ${responseAdapter}`);
  }

  const upstreamDisplay = sanitizeUpstreamDisplay(item.upstreamUrl);
  if (upstreamDisplay) {
    parts.push(`上游 ${upstreamDisplay}`);
  }

  return parts;
}

function fallbackAccountLabelFromId(accountId: string) {
  const separator = accountId.indexOf("::");
  if (separator < 0) {
    return accountId;
  }
  return accountId.slice(separator + 2).trim() || accountId;
}
