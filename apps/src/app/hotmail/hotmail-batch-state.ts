export type HotmailBatchStatusLike = {
  total: number;
  completed: number;
  finished?: boolean;
  cancelled?: boolean;
  status?: string;
  executionMode?: string;
  actionRequiredReason?: string;
  handoffId?: string;
  handoffUrl?: string;
  currentTask?: {
    status?: string;
    currentStep?: string;
    manualActionRequired?: boolean;
  } | null;
  localHandoff?: {
    handoffId?: string;
    url?: string;
  } | null;
};

export type HotmailArtifactLike = {
  filename: string;
  path: string;
  size: number | null;
};

export function getHotmailBatchProgress(batch: Pick<HotmailBatchStatusLike, "total" | "completed">) {
  if (!Number.isFinite(batch.total) || batch.total <= 0) {
    return "0%";
  }
  const completed = Number.isFinite(batch.completed) ? Math.max(0, batch.completed) : 0;
  return `${Math.min(100, Math.floor((completed / batch.total) * 100))}%`;
}

export function shouldPollHotmailBatch(batch: Pick<HotmailBatchStatusLike, "finished" | "cancelled"> | null) {
  return Boolean(batch && !batch.finished && !batch.cancelled);
}

export function mergeHotmailBatchArtifacts<T extends HotmailArtifactLike>(previous: T[], next: T[]) {
  return next.length > 0 ? next : previous;
}

export function classifyHotmailLogLine(line: string) {
  const normalized = String(line || "").toLowerCase();
  if (
    normalized.includes("unsupported_challenge")
    || normalized.includes("let's prove you're human")
    || normalized.includes("press and hold the button")
    || normalized.includes("微软要求人工验证")
  ) {
    return "challenge";
  }
  return "default";
}

export function formatHotmailBatchStatus(
  batch: Pick<HotmailBatchStatusLike, "finished" | "cancelled" | "status" | "actionRequiredReason"> | null
) {
  if (!batch) {
    return {
      label: "未开始",
      className: "border-border bg-muted/40 text-muted-foreground",
    };
  }
  if (batch.cancelled) {
    return {
      label: "已取消",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    };
  }
  if (batch.status === "action_required" || batch.actionRequiredReason === "unsupported_challenge") {
    return {
      label: "等待人工处理",
      className: "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    };
  }
  if (batch.finished) {
    return {
      label: "已完成",
      className: "border-green-500/20 bg-green-500/10 text-green-600 dark:text-green-400",
    };
  }
  return {
    label: "运行中",
    className: "border-blue-500/20 bg-blue-500/10 text-blue-600 dark:text-blue-400",
  };
}

export function hasHotmailPendingHandoff(
  batch: Pick<HotmailBatchStatusLike, "status" | "actionRequiredReason" | "handoffId"> | null,
) {
  if (!batch) {
    return false;
  }
  const actionRequired =
    batch.status === "action_required" || batch.actionRequiredReason === "unsupported_challenge";
  return actionRequired && Boolean(String(batch.handoffId || "").trim());
}

export function hasHotmailPendingLocalHandoff(
  batch: Pick<
    HotmailBatchStatusLike,
    "status" | "actionRequiredReason" | "handoffId" | "localHandoff"
  > | null,
) {
  return hasHotmailPendingHandoff(batch)
    && Boolean(String(batch?.localHandoff?.handoffId || "").trim());
}

export function buildHotmailLocalHandoffActionState(
  batch: Pick<
    HotmailBatchStatusLike,
    "status" | "actionRequiredReason" | "handoffId" | "localHandoff"
  > | null,
  isDesktopRuntime: boolean,
) {
  if (!hasHotmailPendingLocalHandoff(batch)) {
    return {
      enabled: false,
      reason: "当前批次没有可用的本地接管数据",
    };
  }
  if (!isDesktopRuntime) {
    return {
      enabled: false,
      reason: "本地接管仅在桌面版可用",
    };
  }
  return {
    enabled: true,
    reason: "",
  };
}

export function buildHotmailWebLocalHandoffActionState(
  batch: Pick<
    HotmailBatchStatusLike,
    "status" | "actionRequiredReason" | "handoffId" | "localHandoff"
  > | null,
  isDesktopRuntime: boolean,
) {
  if (!hasHotmailPendingLocalHandoff(batch)) {
    return {
      enabled: false,
      reason: "当前批次没有可用的本机接管数据",
    };
  }
  if (isDesktopRuntime) {
    return {
      enabled: false,
      reason: "桌面版请使用本地接管入口",
    };
  }
  return {
    enabled: true,
    reason: "",
  };
}

export function buildHotmailWebLocalHelperUrl(path: string) {
  return `http://127.0.0.1:16788${path}`;
}

export function buildHotmailBackendCallbackBase(currentUrl: string) {
  try {
    const current = new URL(currentUrl);
    return `${current.protocol}//${current.hostname}:9000/api/hotmail`;
  } catch {
    return "http://127.0.0.1:9000/api/hotmail";
  }
}

export function buildHotmailBatchStatusText(
  batch: Pick<HotmailBatchStatusLike, "status" | "executionMode" | "currentTask"> | null,
) {
  if (!batch || batch.executionMode !== "local_first") {
    return "";
  }
  if (batch.currentTask?.manualActionRequired) {
    return "请在已打开的本机浏览器窗口中继续处理微软验证";
  }
  if (batch.status === "running" || batch.status === "pending_local_start") {
    return "正在本机执行";
  }
  return "";
}

export function buildHotmailHandoffAccessUrl(
  batch: Pick<HotmailBatchStatusLike, "handoffId" | "handoffUrl" | "status" | "actionRequiredReason"> | null,
  currentUrl: string,
) {
  if (!hasHotmailPendingHandoff(batch)) {
    return "";
  }
  const configuredUrl = String(batch?.handoffUrl || "").trim();
  if (configuredUrl) {
    return configuredUrl;
  }
  try {
    const current = new URL(currentUrl);
    return `${current.protocol}//${current.hostname}:7900/vnc.html?autoconnect=1&resize=scale`;
  } catch {
    return "";
  }
}

export function buildHotmailNativeVncEndpoint(
  batch: Pick<HotmailBatchStatusLike, "handoffId" | "handoffUrl" | "status" | "actionRequiredReason"> | null,
  currentUrl: string,
) {
  if (!hasHotmailPendingHandoff(batch)) {
    return "";
  }
  const configuredUrl = String(batch?.handoffUrl || "").trim();
  try {
    const source = configuredUrl ? new URL(configuredUrl) : new URL(currentUrl);
    return `${source.hostname}:5900`;
  } catch {
    return "";
  }
}
