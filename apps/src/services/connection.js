import { state } from "../state.js";
import * as api from "../api.js";
import { setStatus, setServiceHint } from "../ui/status.js";

// 规范化端口/地址输入
export function normalizeAddr(raw) {
  const trimmed = String(raw || "").trim();
  if (!trimmed) {
    throw new Error("请输入端口或地址");
  }
  let value = trimmed;
  if (value.startsWith("http://")) {
    value = value.slice("http://".length);
  }
  if (value.startsWith("https://")) {
    value = value.slice("https://".length);
  }
  value = value.split("/")[0];
  if (!value.includes(":")) {
    value = `localhost:${value}`;
  }
  const [host, port] = value.split(":");
  if (!port) return value;
  if (host === "127.0.0.1" || host === "0.0.0.0") {
    return `localhost:${port}`;
  }
  return value;
}

// 初始化连接（不负责启动 service）
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export function createConnectionService(deps) {
  const {
    api: apiClient = api,
    state: stateRef = state,
    setStatus: setStatusFn = setStatus,
    setServiceHint: setServiceHintFn = setServiceHint,
    wait = sleep,
  } = deps || {};

  async function initializeService(options = {}) {
    const {
      retries = 0,
      delayMs = 300,
      silent = false,
      wait: waitFn = wait,
    } = options;
    setStatusFn("连接中...", false);
    setServiceHintFn("", false);

    let lastError = null;
    for (let attempt = 0; attempt <= retries; attempt += 1) {
      try {
        await apiClient.serviceInitialize();
        stateRef.serviceConnected = true;
        setStatusFn("", true);
        setServiceHintFn("", false);
        return true;
      } catch (err) {
        lastError = err;
        stateRef.serviceConnected = false;
        if (attempt < retries) {
          await waitFn(delayMs);
        }
      }
    }

    setStatusFn("", false);
    if (!silent) {
      setServiceHintFn("连接失败，请检查端口或 service 状态", true);
    }
    if (lastError) {
      return false;
    }
    return false;
  }

  async function ensureConnected() {
    if (stateRef.serviceConnected) return true;
    return initializeService({ retries: 1, delayMs: 200 });
  }

  async function startService(rawAddr, options = {}) {
    const addr = normalizeAddr(rawAddr);
    stateRef.serviceAddr = addr;
    setServiceHintFn("", false);
    setStatusFn("启动中...", false);
    try {
      await apiClient.serviceStart(addr);
    } catch (err) {
      setStatusFn("", false);
      setServiceHintFn(`启动失败：${String(err)}`, true);
      return false;
    }
    const {
      retries = 8,
      delayMs = 400,
      silent = false,
      wait: waitFn,
      skipInitialize = false,
    } = options;
    if (skipInitialize) {
      return true;
    }
    return initializeService({ retries, delayMs, wait: waitFn, silent });
  }

  async function stopService() {
    setStatusFn("停止中...", false);
    try {
      await apiClient.serviceStop();
    } catch (err) {
      setServiceHintFn(`停止失败：${String(err)}`, true);
    }
    stateRef.serviceConnected = false;
    setStatusFn("", false);
  }

  return {
    initializeService,
    ensureConnected,
    startService,
    stopService,
    waitForConnection: (options = {}) => {
      const {
        retries = 8,
        delayMs = 400,
        silent = true,
        wait: waitFn,
      } = options;
      return initializeService({ retries, delayMs, silent, wait: waitFn });
    },
  };
}

// 确保已连接
const defaultService = createConnectionService();

export const initializeService = defaultService.initializeService;
export const ensureConnected = defaultService.ensureConnected;
export const startService = defaultService.startService;
export const stopService = defaultService.stopService;
export const waitForConnection = defaultService.waitForConnection;
