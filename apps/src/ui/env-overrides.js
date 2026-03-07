function normalizeEnvKey(value) {
  return String(value || "").trim().toUpperCase();
}

export function normalizeStringList(value) {
  const items = Array.isArray(value) ? value : [];
  return [...new Set(items.map((item) => String(item || "").trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b));
}

export function normalizeEnvOverrides(value) {
  const source = value && typeof value === "object" ? value : {};
  const entries = [];
  for (const [rawKey, rawValue] of Object.entries(source)) {
    const key = normalizeEnvKey(rawKey);
    const normalizedValue = String(rawValue ?? "").trim();
    if (!key || !key.startsWith("CODEXMANAGER_") || !normalizedValue) {
      continue;
    }
    entries.push([key, normalizedValue]);
  }
  entries.sort(([left], [right]) => left.localeCompare(right));
  return Object.fromEntries(entries);
}

export function parseEnvOverridesText(text) {
  const lines = String(text || "").split(/\r?\n/);
  const parsed = {};
  const errors = [];

  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index];
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }
    const separatorIndex = rawLine.indexOf("=");
    if (separatorIndex <= 0) {
      errors.push(`第 ${index + 1} 行缺少 KEY=VALUE 格式`);
      continue;
    }
    const key = normalizeEnvKey(rawLine.slice(0, separatorIndex));
    if (!key.startsWith("CODEXMANAGER_")) {
      errors.push(`第 ${index + 1} 行必须使用 CODEXMANAGER_ 前缀`);
      continue;
    }
    const value = rawLine.slice(separatorIndex + 1).trim();
    if (!value) {
      delete parsed[key];
      continue;
    }
    parsed[key] = value;
  }

  if (errors.length > 0) {
    return {
      ok: false,
      error: errors[0],
      errors,
    };
  }

  return {
    ok: true,
    overrides: normalizeEnvOverrides(parsed),
  };
}

export function formatEnvOverridesText(value) {
  const normalized = normalizeEnvOverrides(value);
  return Object.entries(normalized)
    .map(([key, envValue]) => `${key}=${envValue}`)
    .join("\n");
}

export function normalizeEnvOverrideCatalog(value) {
  const source = Array.isArray(value) ? value : [];
  const catalog = new Map();
  for (const item of source) {
    if (!item || typeof item !== "object") {
      continue;
    }
    const key = normalizeEnvKey(item.key);
    if (!key) {
      continue;
    }
    const scope = String(item.scope || "service").trim().toLowerCase() || "service";
    const applyMode = String(item.applyMode || "runtime").trim().toLowerCase() || "runtime";
    catalog.set(key, {
      key,
      scope,
      applyMode,
    });
  }
  return [...catalog.values()].sort((left, right) => left.key.localeCompare(right.key));
}
