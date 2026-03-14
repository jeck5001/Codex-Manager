import { readFile } from "node:fs/promises";
import path from "node:path";

import { unwrapRpcError, unwrapRpcResult } from "@/lib/codex/rpc-payload";

const DEFAULT_SERVICE_ADDR = "localhost:48760";
const DEFAULT_RPC_TOKEN_FILENAME = "codexmanager.rpc-token";
const DEFAULT_JSON_RESPONSE = "{}";
const ENV_FILE_CANDIDATES = ["codexmanager.env", "CodexManager.env", ".env"] as const;

const ENV_SERVICE_ADDR = "CODEXMANAGER_SERVICE_ADDR";
const ENV_DB_PATH = "CODEXMANAGER_DB_PATH";
const ENV_RPC_TOKEN = "CODEXMANAGER_RPC_TOKEN";
const ENV_RPC_TOKEN_FILE = "CODEXMANAGER_RPC_TOKEN_FILE";

let envFileCache: Record<string, string> | null = null;
let rpcTokenCache: string | null = null;

function readEnvTrim(name: string) {
  const value = process.env[name];
  if (typeof value !== "string") {
    return "";
  }
  return value.trim();
}

function stripInlineComment(value: string) {
  const commentIndex = value.indexOf(" #");
  if (commentIndex < 0) {
    return value;
  }
  return value.slice(0, commentIndex).trimEnd();
}

function parseDotenvValue(rawLine: string) {
  let line = rawLine.trim();
  if (!line || line.startsWith("#") || line.startsWith(";")) {
    return null;
  }

  if (line.startsWith("export ")) {
    line = line.slice("export ".length).trim();
  }

  const separatorIndex = line.indexOf("=");
  if (separatorIndex < 0) {
    return null;
  }

  const key = line.slice(0, separatorIndex).trim();
  if (!key) {
    return null;
  }

  let value = line.slice(separatorIndex + 1).trim();
  if (
    ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) &&
    value.length >= 2
  ) {
    value = value.slice(1, -1);
  } else {
    value = stripInlineComment(value);
  }

  return [key, value] as const;
}

async function loadBootstrapEnvFile() {
  if (envFileCache) {
    return envFileCache;
  }

  for (const candidate of ENV_FILE_CANDIDATES) {
    const filePath = path.join(process.cwd(), candidate);
    try {
      const text = await readFile(filePath, "utf8");
      envFileCache = Object.fromEntries(
        text
          .split(/\r?\n/)
          .map((line) => parseDotenvValue(line))
          .filter((entry): entry is readonly [string, string] => entry !== null),
      );
      return envFileCache;
    } catch {}
  }

  envFileCache = {};
  return envFileCache;
}

async function readBootstrapValue(name: string) {
  const envValue = readEnvTrim(name);
  if (envValue) {
    return envValue;
  }

  const envFile = await loadBootstrapEnvFile();
  return String(envFile[name] || "").trim();
}

function normalizeServiceAddress(raw: string) {
  let value = String(raw || "").trim();
  if (!value) {
    return DEFAULT_SERVICE_ADDR;
  }

  if (value.startsWith("http://")) {
    value = value.slice("http://".length);
  } else if (value.startsWith("https://")) {
    value = value.slice("https://".length);
  }

  value = value.split("/")[0] || value;
  if (!value) {
    return DEFAULT_SERVICE_ADDR;
  }

  if (/^\d+$/.test(value)) {
    return `localhost:${value}`;
  }

  const [host, port] = value.split(":");
  if (port && (host === "0.0.0.0" || host === "127.0.0.1")) {
    return `localhost:${port}`;
  }

  return value;
}

function resolvePathWithBase(raw: string, baseDir: string) {
  const trimmed = raw.trim();
  if (!trimmed) {
    return "";
  }

  if (path.isAbsolute(trimmed)) {
    return trimmed;
  }

  return path.join(baseDir, trimmed);
}

async function resolveDbDir() {
  const cwd = process.cwd();
  const dbPath = await readBootstrapValue(ENV_DB_PATH);
  if (!dbPath) {
    return cwd;
  }

  const resolvedPath = resolvePathWithBase(dbPath, cwd);
  return path.dirname(resolvedPath);
}

async function resolveRpcTokenFilePath() {
  const dbDir = await resolveDbDir();
  const configuredPath = await readBootstrapValue(ENV_RPC_TOKEN_FILE);
  if (configuredPath) {
    return resolvePathWithBase(configuredPath, dbDir);
  }
  return path.join(dbDir, DEFAULT_RPC_TOKEN_FILENAME);
}

async function resolveRpcToken() {
  if (rpcTokenCache) {
    return rpcTokenCache;
  }

  const envToken = await readBootstrapValue(ENV_RPC_TOKEN);
  if (envToken) {
    rpcTokenCache = envToken;
    return rpcTokenCache;
  }

  const tokenFilePath = await resolveRpcTokenFilePath();
  try {
    const token = (await readFile(tokenFilePath, "utf8")).trim();
    if (token) {
      rpcTokenCache = token;
      return rpcTokenCache;
    }
  } catch {}

  throw new Error(
    `RPC 代理未配置认证令牌，请设置 ${ENV_RPC_TOKEN} 或 ${ENV_RPC_TOKEN_FILE}。`,
  );
}

function isJsonContentType(contentType: string | null) {
  return contentType?.split(";")[0]?.trim().toLowerCase() === "application/json";
}

async function resolveRpcRequestConfig() {
  let serviceAddr = DEFAULT_SERVICE_ADDR;
  let rpcToken = "";
  serviceAddr = normalizeServiceAddress(
    (await readBootstrapValue(ENV_SERVICE_ADDR)) || DEFAULT_SERVICE_ADDR,
  );
  rpcToken = await resolveRpcToken();
  return { serviceAddr, rpcToken };
}

async function fetchUpstreamRpc(body: string, signal?: AbortSignal) {
  const { serviceAddr, rpcToken } = await resolveRpcRequestConfig();
  return fetch(`http://${serviceAddr}/rpc`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-codexmanager-rpc-token": rpcToken,
    },
    body,
    cache: "no-store",
    signal,
  });
}

export async function callCodexRpc<T>(
  method: string,
  params?: Record<string, unknown>,
  signal?: AbortSignal,
) {
  const body = JSON.stringify({
    jsonrpc: "2.0",
    id: Date.now(),
    method,
    params: params == null ? undefined : params,
  });

  let upstreamResponse: Response;
  try {
    upstreamResponse = await fetchUpstreamRpc(body, signal);
  } catch (error) {
    const message = error instanceof Error && error.message.trim() ? error.message : "unknown upstream error";
    throw new Error(`upstream error: ${message}`);
  }

  let upstreamPayload: unknown;
  try {
    upstreamPayload = (await upstreamResponse.json()) as unknown;
  } catch (error) {
    const message = error instanceof Error && error.message.trim() ? error.message : "unknown upstream read error";
    throw new Error(`upstream read error: ${message}`);
  }

  if (!upstreamResponse.ok) {
    const rpcError = unwrapRpcError(upstreamPayload);
    if (rpcError) {
      throw new Error(`RPC 请求失败（HTTP ${upstreamResponse.status}）：${rpcError}`);
    }
    throw new Error(`RPC 请求失败（HTTP ${upstreamResponse.status}）`);
  }

  return unwrapRpcResult<T>(upstreamPayload);
}

export async function proxyCodexRpc(request: Request) {
  if (!isJsonContentType(request.headers.get("content-type"))) {
    return new Response(DEFAULT_JSON_RESPONSE, {
      status: 415,
      headers: {
        "content-type": "application/json",
        "cache-control": "no-store",
      },
    });
  }

  const body = await request.text();
  if (!body.trim()) {
    return new Response(DEFAULT_JSON_RESPONSE, {
      status: 400,
      headers: {
        "content-type": "application/json",
        "cache-control": "no-store",
      },
    });
  }

  let upstreamResponse: Response;
  try {
    upstreamResponse = await fetchUpstreamRpc(body, request.signal);
  } catch (error) {
    const message = error instanceof Error && error.message.trim() ? error.message : "unknown upstream error";
    const status = message.includes("RPC 代理未配置认证令牌") ? 500 : 502;
    return new Response(message.startsWith("upstream ") ? message : `upstream error: ${message}`, {
      status,
      headers: {
        "content-type": "text/plain; charset=utf-8",
        "cache-control": "no-store",
      },
    });
  }

  let upstreamBody: ArrayBuffer;
  try {
    upstreamBody = await upstreamResponse.arrayBuffer();
  } catch (error) {
    const message = error instanceof Error && error.message.trim() ? error.message : "unknown upstream read error";
    return new Response(`upstream read error: ${message}`, {
      status: 502,
      headers: {
        "content-type": "text/plain; charset=utf-8",
        "cache-control": "no-store",
      },
    });
  }

  return new Response(upstreamBody, {
    status: upstreamResponse.status,
    headers: {
      "content-type": "application/json",
      "cache-control": "no-store",
    },
  });
}
