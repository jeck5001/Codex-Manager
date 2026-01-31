import { state } from "./state.js";

// 统一 Tauri 调用入口
export async function invoke(method, params) {
  const tauri = window.__TAURI__;
  if (!tauri || !tauri.core || !tauri.core.invoke) {
    throw new Error("Tauri API 不可用（请在桌面端运行）");
  }
  const res = await tauri.core.invoke(method, params || {});
  if (res && Object.prototype.hasOwnProperty.call(res, "result")) {
    return res.result;
  }
  return res;
}

function withAddr(extra) {
  return {
    addr: state.serviceAddr || null,
    ...(extra || {}),
  };
}

// service 生命周期
export async function serviceStart(addr) {
  return invoke("service_start", { addr });
}

export async function serviceStop() {
  return invoke("service_stop", {});
}

export async function serviceInitialize() {
  return invoke("service_initialize", withAddr());
}

// 账号
export async function serviceAccountList() {
  return invoke("service_account_list", withAddr());
}

export async function serviceAccountDelete(accountId) {
  return invoke("service_account_delete", withAddr({ accountId }));
}

export async function serviceAccountUpdate(accountId, sort) {
  return invoke("service_account_update", withAddr({ accountId, sort }));
}

export async function localAccountDelete(accountId) {
  return invoke("local_account_delete", { accountId });
}

// 用量
export async function serviceUsageRead(accountId) {
  return invoke("service_usage_read", withAddr({ accountId }));
}

export async function serviceUsageList() {
  return invoke("service_usage_list", withAddr());
}

export async function serviceUsageRefresh(accountId) {
  return invoke("service_usage_refresh", withAddr({ accountId }));
}

// 登录
export async function serviceLoginStart(payload) {
  return invoke("service_login_start", withAddr(payload));
}

export async function serviceLoginStatus(loginId) {
  return invoke("service_login_status", withAddr({ loginId }));
}

export async function serviceLoginComplete(state, code, redirectUri) {
  return invoke("service_login_complete", withAddr({ state, code, redirectUri }));
}

// API Key
export async function serviceApiKeyList() {
  return invoke("service_apikey_list", withAddr());
}

export async function serviceApiKeyCreate(name) {
  return invoke("service_apikey_create", withAddr({ name }));
}

export async function serviceApiKeyDelete(keyId) {
  return invoke("service_apikey_delete", withAddr({ keyId }));
}

export async function serviceApiKeyDisable(keyId) {
  return invoke("service_apikey_disable", withAddr({ keyId }));
}

// 打开浏览器
export async function openInBrowser(url) {
  return invoke("open_in_browser", { url });
}
