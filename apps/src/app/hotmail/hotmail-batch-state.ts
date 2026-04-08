export type HotmailBatchStatusLike = {
  total: number;
  completed: number;
  finished?: boolean;
  cancelled?: boolean;
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
