"use client";

const ISO_WITHOUT_TIMEZONE_PATTERN =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?$/;

function normalizeApiDateString(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";
  if (ISO_WITHOUT_TIMEZONE_PATTERN.test(trimmed)) {
    return `${trimmed}Z`;
  }
  return trimmed;
}

export function parseApiDate(value: string | null | undefined): Date | null {
  const normalized = normalizeApiDateString(String(value || ""));
  if (!normalized) return null;
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return date;
}

export function formatApiDateTime(
  value: string | null | undefined,
  {
    emptyLabel = "--",
    invalidFallback,
    withSeconds = true,
  }: {
    emptyLabel?: string;
    invalidFallback?: string;
    withSeconds?: boolean;
  } = {}
): string {
  if (!value) return emptyLabel;
  const date = parseApiDate(value);
  if (!date) {
    return invalidFallback ?? String(value);
  }
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: withSeconds ? "2-digit" : undefined,
    hour12: false,
    timeZone: "Asia/Shanghai",
  });
}
