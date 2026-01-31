// 全局状态：尽量集中管理 UI 与数据状态
export const state = {
  serviceAddr: "",
  serviceConnected: false,
  serviceBusy: false,
  serviceProbeId: 0,
  currentPage: "dashboard",
  accountList: [],
  usageList: [],
  apiKeyList: [],
  currentUsageAccount: null,
  activeLoginId: null,
  autoRefreshTimer: null,
};
