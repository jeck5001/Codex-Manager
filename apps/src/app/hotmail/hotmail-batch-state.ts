export type HotmailBatchStatusLike = {
  total: number;
  completed: number;
  finished?: boolean;
  cancelled?: boolean;
  status?: string;
  actionRequiredReason?: string;
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
