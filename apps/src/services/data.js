import { state } from "../state";
import * as api from "../api";

// 刷新账号列表
export async function refreshAccounts() {
  const res = await api.serviceAccountList();
  state.accountList = res && res.items ? res.items : [];
}

// 刷新用量列表
export async function refreshUsageList() {
  const res = await api.serviceUsageList();
  state.usageList = res && res.items ? res.items : [];
}

// 刷新 API Key 列表
export async function refreshApiKeys() {
  const res = await api.serviceApiKeyList();
  state.apiKeyList = res && res.items ? res.items : [];
}

// 刷新模型下拉选项（来自平台上游 /v1/models）
export async function refreshApiModels() {
  try {
    const res = await api.serviceApiKeyModels();
    state.apiModelOptions = res && res.items ? res.items : [];
  } catch (_err) {
    state.apiModelOptions = [];
  }
}

// 刷新请求日志（按关键字过滤）
export async function refreshRequestLogs(query) {
  const res = await api.serviceRequestLogList(query || null, 300);
  state.requestLogList = res && res.items ? res.items : [];
}

export async function clearRequestLogs() {
  return api.serviceRequestLogClear();
}
