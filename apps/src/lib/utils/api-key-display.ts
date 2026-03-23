"use client";

import type { ApiKey } from "@/types";

export function buildApiKeyNameMap(apiKeys?: ApiKey[] | null): Map<string, string> {
  return new Map(
    (apiKeys || [])
      .map((item) => {
        const id = String(item.id || "").trim();
        const name = String(item.name || "").trim();
        if (!id) return null;
        return [id, name] as const;
      })
      .filter((entry): entry is readonly [string, string] => Boolean(entry))
  );
}

export function resolveApiKeyName(
  keyId: string,
  apiKeyNameMap: Map<string, string>,
): string {
  const normalized = String(keyId || "").trim();
  if (!normalized) return "";
  return String(apiKeyNameMap.get(normalized) || "").trim();
}

export function formatCompactApiKeyId(keyId: string): string {
  const normalized = String(keyId || "").trim();
  if (!normalized) return "-";
  if (normalized.length <= 12) return normalized;
  return `${normalized.slice(0, 8)}...`;
}

export function formatApiKeyInlineLabel(
  keyId: string,
  apiKeyNameMap: Map<string, string>,
): string {
  const normalized = String(keyId || "").trim();
  if (!normalized) return "-";
  const name = resolveApiKeyName(normalized, apiKeyNameMap);
  if (name && name !== normalized) {
    return `${name} · ${formatCompactApiKeyId(normalized)}`;
  }
  return formatCompactApiKeyId(normalized);
}

export function formatApiKeyDetailLabel(
  keyId: string,
  apiKeyNameMap: Map<string, string>,
): string {
  const normalized = String(keyId || "").trim();
  if (!normalized) return "-";
  const name = resolveApiKeyName(normalized, apiKeyNameMap);
  if (name && name !== normalized) {
    return `${name} (${normalized})`;
  }
  return normalized;
}
