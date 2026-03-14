import type { RequestLogRecord } from "./types.ts";

export function buildRequestLogIdentity(item: RequestLogRecord, index: number) {
  return [
    item.id || "",
    item.traceId || "",
    item.createdAt || "",
    item.accountId || "",
    item.method || "",
    item.path || item.mappedPath || item.adaptedPath || "",
    item.statusCode || "",
    index,
  ].join("|");
}

export function isAppendOnlyRequestLogResult(previousKeys: string[], nextKeys: string[]) {
  if (previousKeys.length === 0 || nextKeys.length < previousKeys.length) {
    return false;
  }
  for (let index = 0; index < previousKeys.length; index += 1) {
    if (previousKeys[index] !== nextKeys[index]) {
      return false;
    }
  }
  return true;
}

type RequestLogRenderStateInput = {
  previousKeys: string[];
  nextKeys: string[];
  currentRenderedCount: number;
  initialBatch: number;
  appendBatch: number;
  wasNearBottom: boolean;
};

export function resolveNextRequestLogRenderedCount({
  previousKeys,
  nextKeys,
  currentRenderedCount,
  initialBatch,
  appendBatch,
  wasNearBottom,
}: RequestLogRenderStateInput) {
  if (!nextKeys.length) {
    return 0;
  }

  const appendOnly = isAppendOnlyRequestLogResult(previousKeys, nextKeys);
  if (!appendOnly) {
    return Math.min(nextKeys.length, initialBatch);
  }

  const unchanged = previousKeys.length === nextKeys.length;
  const boundedCurrent = Math.min(currentRenderedCount, previousKeys.length);
  if (unchanged) {
    return Math.min(nextKeys.length, Math.max(boundedCurrent, initialBatch));
  }

  if (boundedCurrent >= previousKeys.length || wasNearBottom) {
    return Math.min(nextKeys.length, boundedCurrent + appendBatch);
  }

  return Math.min(nextKeys.length, Math.max(boundedCurrent, initialBatch));
}

type RequestLogVirtualWindowInput = {
  renderedCount: number;
  scrollTop: number;
  viewportHeight: number;
  estimatedRowHeight: number;
  overscanRows: number;
};

export function resolveRequestLogVirtualWindow({
  renderedCount,
  scrollTop,
  viewportHeight,
  estimatedRowHeight,
  overscanRows,
}: RequestLogVirtualWindowInput) {
  if (renderedCount <= 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    };
  }

  const rowHeight = Math.max(1, Math.floor(estimatedRowHeight));
  const safeOverscanRows = Math.max(0, Math.floor(overscanRows));
  const safeScrollTop = Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0;
  const visibleRows =
    Number.isFinite(viewportHeight) && viewportHeight > 0
      ? Math.max(1, Math.ceil(viewportHeight / rowHeight))
      : Math.min(renderedCount, Math.max(24, safeOverscanRows * 2 + 1));

  const startIndex = Math.max(0, Math.min(renderedCount, Math.floor(safeScrollTop / rowHeight) - safeOverscanRows));
  const endIndex = Math.max(
    startIndex,
    Math.min(renderedCount, startIndex + visibleRows + safeOverscanRows * 2),
  );

  return {
    startIndex,
    endIndex,
    topSpacerHeight: startIndex * rowHeight,
    bottomSpacerHeight: Math.max(0, renderedCount - endIndex) * rowHeight,
  };
}
