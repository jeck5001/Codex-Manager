export function buildRenderActions({
  updateAccountSort,
  handleOpenUsageModal,
  deleteAccount,
  toggleApiKeyStatus,
  deleteApiKey,
  updateApiKeyModel,
}) {
  return {
    onUpdateSort: updateAccountSort,
    onOpenUsage: handleOpenUsageModal,
    onDeleteAccount: deleteAccount,
    onToggleApiKeyStatus: toggleApiKeyStatus,
    onDeleteApiKey: deleteApiKey,
    onUpdateApiKeyModel: updateApiKeyModel,
  };
}
