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
