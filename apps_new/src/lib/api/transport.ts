import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { fetchWithRetry, runWithControl, RequestOptions } from "../utils/request";
import { useAppStore } from "../store/useAppStore";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean((window as any).__TAURI__);
}

export function withAddr(params: Record<string, any> = {}): Record<string, any> {
  const addr = useAppStore.getState().serviceStatus.addr;
  return {
    addr: addr || null,
    ...params,
  };
}

export function isCommandMissingError(err: any): boolean {
  const msg = String(err?.message || err || "").toLowerCase();
  return (
    msg.includes("unknown command") ||
    msg.includes("not found") ||
    msg.includes("is not a registered")
  );
}

export async function invokeFirst<T>(
  methods: string[],
  params?: Record<string, any>,
  options: RequestOptions = {}
): Promise<T> {
  let lastErr: any;
  for (const method of methods) {
    try {
      return await invoke<T>(method, params, options);
    } catch (err) {
      lastErr = err;
      if (!isCommandMissingError(err)) {
        throw err;
      }
    }
  }
  throw lastErr || new Error("未配置可用命令");
}

export async function invoke<T>(
  method: string,
  params?: Record<string, any>,
  options: RequestOptions = {}
): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error("桌面接口不可用（请在桌面端运行）");
  }

  const res = await runWithControl(
    () => tauriInvoke(method, params || {}),
    options
  ) as any;

  if (res && typeof res === "object" && "error" in res) {
    const err = res.error;
    throw new Error(typeof err === "string" ? err : err?.message || JSON.stringify(err));
  }

  const throwIfBusinessError = (payload: any) => {
    const msg = resolveBusinessErrorMessage(payload);
    if (msg) throw new Error(msg);
  };

  if (res && "result" in res) {
    const payload = res.result;
    throwIfBusinessError(payload);
    return payload;
  }
  
  throwIfBusinessError(res);
  return res;
}

function resolveBusinessErrorMessage(payload: any): string {
  if (!payload || typeof payload !== "object") return "";
  const err = payload.error;
  if (payload.ok === false) {
    return typeof err === "string" ? err : err?.message || "操作失败";
  }
  if (err) {
    return typeof err === "string" ? err : err?.message || "";
  }
  return "";
}

export async function requestlogListViaHttpRpc<T>(
  query: string,
  limit: number,
  addr: string,
  options: RequestOptions = {}
): Promise<T> {
  // Desktop environment should use Tauri invoke for reliability
  if (isTauriRuntime()) {
    return invoke<T>("service_requestlog_list", { query, limit, addr }, options);
  }

  // Fallback for web mode if needed (though not primary for this app)
  const body = JSON.stringify({
    jsonrpc: "2.0",
    id: Date.now(),
    method: "requestlog/list",
    params: { query, limit },
  });

  const response = await fetchWithRetry(
    `http://${addr}/rpc`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
    },
    options
  );

  if (!response.ok) throw new Error(`RPC 请求失败（HTTP ${response.status}）`);
  const payload = await response.json();
  return payload.result ?? payload;
}
