import { renderDashboard } from "./dashboard";
import { renderAccounts } from "./accounts";
import { renderApiKeys } from "./apikeys";
import { renderRequestLogs } from "./requestlogs";

export function renderAccountsOnly(handlers) {
  renderAccounts({
    onUpdateSort: handlers.onUpdateSort,
    onOpenUsage: handlers.onOpenUsage,
    onDelete: handlers.onDeleteAccount,
  });
}

export function renderAllViews(handlers) {
  renderDashboard();
  renderAccountsOnly(handlers);
  renderApiKeys({
    onToggleStatus: handlers.onToggleApiKeyStatus,
    onDelete: handlers.onDeleteApiKey,
    onUpdateModel: handlers.onUpdateApiKeyModel,
  });
  renderRequestLogs();
}
