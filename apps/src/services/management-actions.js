import * as api from "../api";

export function createManagementActions({
  dom,
  state,
  ensureConnected,
  withButtonBusy,
  showToast,
  showConfirmDialog,
  clearRequestLogs,
  refreshRequestLogs,
  renderRequestLogs,
  refreshAll,
  openUsageModal,
  renderUsageSnapshot,
  refreshApiModels,
  refreshApiKeys,
  populateApiKeyModelSelect,
  renderApiKeys,
}) {
  const renderApiKeyList = () => {
    renderApiKeys({
      onToggleStatus: toggleApiKeyStatus,
      onDelete: deleteApiKey,
      onUpdateModel: updateApiKeyModel,
    });
  };

  async function handleClearRequestLogs() {
    const confirmed = await showConfirmDialog({
      title: "清空请求日志",
      message: "确定清空请求日志吗？该操作不可撤销。",
      confirmText: "清空",
      cancelText: "取消",
    });
    if (!confirmed) return;
    await withButtonBusy(dom.clearRequestLogs, "清空中...", async () => {
      const ok = await ensureConnected();
      if (!ok) return;
      const res = await clearRequestLogs();
      if (res && res.ok === false) {
        showToast(res.error || "清空日志失败", "error");
        return;
      }
      await refreshRequestLogs(state.requestLogQuery);
      renderRequestLogs();
      showToast("请求日志已清空");
    });
  }

  async function updateAccountSort(accountId, sort) {
    const ok = await ensureConnected();
    if (!ok) return;
    await api.serviceAccountUpdate(accountId, sort);
    await refreshAll();
  }

  async function deleteAccount(account) {
    if (!account || !account.id) return;
    const confirmed = await showConfirmDialog({
      title: "删除账号",
      message: `确定删除账号 ${account.label} 吗？删除后不可恢复。`,
      confirmText: "删除",
      cancelText: "取消",
    });
    if (!confirmed) return;
    const ok = await ensureConnected();
    if (!ok) return;
    const res = await api.serviceAccountDelete(account.id);
    if (res && res.error === "unknown_method") {
      const fallback = await api.localAccountDelete(account.id);
      if (fallback && fallback.ok) {
        await refreshAll();
        return;
      }
      const msg = fallback && fallback.error ? fallback.error : "删除失败";
      showToast(msg, "error");
      return;
    }
    if (res && res.ok) {
      await refreshAll();
      showToast("账号已删除");
    } else {
      const msg = res && res.error ? res.error : "删除失败";
      showToast(msg, "error");
    }
  }

  async function handleOpenUsageModal(account) {
    openUsageModal(account);
    await refreshUsageForAccount();
  }

  async function refreshUsageForAccount() {
    if (!state.currentUsageAccount) return;
    const ok = await ensureConnected();
    if (!ok) return;
    dom.refreshUsageSingle.disabled = true;
    try {
      await api.serviceUsageRefresh(state.currentUsageAccount.id);
      const res = await api.serviceUsageRead(state.currentUsageAccount.id);
      const snap = res ? res.snapshot : null;
      renderUsageSnapshot(snap);
    } catch (err) {
      dom.usageDetail.textContent = String(err);
    }
    dom.refreshUsageSingle.disabled = false;
  }

  async function createApiKey() {
    await withButtonBusy(dom.submitApiKey, "创建中...", async () => {
      const ok = await ensureConnected();
      if (!ok) return;
      const modelSlug = dom.inputApiKeyModel.value || null;
      const reasoningEffort = modelSlug ? (dom.inputApiKeyReasoning.value || null) : null;
      const res = await api.serviceApiKeyCreate(
        dom.inputApiKeyName.value.trim() || null,
        modelSlug,
        reasoningEffort,
      );
      if (res && res.error) {
        showToast(res.error, "error");
        return;
      }
      dom.apiKeyValue.value = res && res.key ? res.key : "";
      await refreshApiModels();
      await refreshApiKeys();
      populateApiKeyModelSelect();
      renderApiKeyList();
      showToast("平台 Key 创建成功");
    });
  }

  async function deleteApiKey(item) {
    if (!item || !item.id) return;
    const confirmed = await showConfirmDialog({
      title: "删除平台 Key",
      message: `确定删除平台 Key ${item.id} 吗？`,
      confirmText: "删除",
      cancelText: "取消",
    });
    if (!confirmed) return;
    const ok = await ensureConnected();
    if (!ok) return;
    await api.serviceApiKeyDelete(item.id);
    await refreshApiKeys();
    renderApiKeyList();
    showToast("平台 Key 已删除");
  }

  async function toggleApiKeyStatus(item) {
    if (!item || !item.id) return;
    const ok = await ensureConnected();
    if (!ok) return;
    const isDisabled = String(item.status || "").toLowerCase() === "disabled";
    if (isDisabled) {
      await api.serviceApiKeyEnable(item.id);
    } else {
      await api.serviceApiKeyDisable(item.id);
    }
    await refreshApiKeys();
    renderApiKeyList();
    showToast(isDisabled ? "平台 Key 已启用" : "平台 Key 已禁用");
  }

  async function updateApiKeyModel(item, modelSlug, reasoningEffort) {
    if (!item || !item.id) return;
    const ok = await ensureConnected();
    if (!ok) return;
    const normalizedModel = modelSlug || null;
    const normalizedEffort = normalizedModel ? (reasoningEffort || null) : null;
    const res = await api.serviceApiKeyUpdateModel(item.id, normalizedModel, normalizedEffort);
    if (res && res.ok === false) {
      showToast(res.error || "模型配置保存失败", "error");
      return;
    }
    await refreshApiKeys();
    renderApiKeyList();
  }

  return {
    handleClearRequestLogs,
    updateAccountSort,
    deleteAccount,
    handleOpenUsageModal,
    refreshUsageForAccount,
    createApiKey,
    deleteApiKey,
    toggleApiKeyStatus,
    updateApiKeyModel,
  };
}
