import { dom } from "../ui/dom.js";
import { state } from "../state.js";
import { copyText } from "../utils/clipboard.js";
import {
  createRequestLogRow,
  createTopSpacerRow,
  renderEmptyRequestLogs,
} from "./requestlogs/row-render.js";
import {
  buildRequestRouteMeta,
  collectFilteredRequestLogs,
  ensureAccountLabelMap,
  fallbackAccountDisplayFromKey,
  isAppendOnlyResult,
  resolveAccountDisplayName,
  resolveDisplayRequestPath,
} from "./requestlogs/selectors.js";
import {
  appendAtLeastOneBatch,
  appendNearBottomBatches,
  appendRequestLogBatch,
  isNearBottom,
} from "./requestlogs/virtual-list.js";

const REQUEST_LOG_BATCH_SIZE = 80;
const REQUEST_LOG_DOM_LIMIT = 240;
const REQUEST_LOG_DOM_RECYCLE_TO = 180;
const REQUEST_LOG_SCROLL_BUFFER = 180;
const REQUEST_LOG_FALLBACK_ROW_HEIGHT = 54;
const REQUEST_LOG_COLUMN_COUNT = 9;
const REQUEST_LOG_NEAR_BOTTOM_MAX_BATCHES = 1;

const requestLogWindowState = {
  filter: "all",
  filtered: [],
  filteredKeys: [],
  nextIndex: 0,
  topSpacerHeight: 0,
  recycledRowHeight: REQUEST_LOG_FALLBACK_ROW_HEIGHT,
  accountListRef: null,
  accountLabelById: new Map(),
  topSpacerRow: null,
  topSpacerCell: null,
  boundRowsEl: null,
  boundScrollerEl: null,
  scrollTickHandle: null,
  scrollTickMode: "",
  hasRendered: false,
};

function createRowRenderer() {
  const accountLabelById = requestLogWindowState.accountLabelById;
  const rowRenderHelpers = {
    resolveAccountDisplayName: (item) =>
      resolveAccountDisplayName(item, accountLabelById),
    fallbackAccountDisplayFromKey,
    resolveDisplayRequestPath,
    buildRequestRouteMeta,
  };
  return (item, index) => createRequestLogRow(item, index, rowRenderHelpers);
}

function appendRequestLogBatchLocal() {
  return appendRequestLogBatch({
    rowsEl: dom.requestLogRows,
    windowState: requestLogWindowState,
    batchSize: REQUEST_LOG_BATCH_SIZE,
    createRow: createRowRenderer(),
    domLimit: REQUEST_LOG_DOM_LIMIT,
    domRecycleTo: REQUEST_LOG_DOM_RECYCLE_TO,
    fallbackRowHeight: REQUEST_LOG_FALLBACK_ROW_HEIGHT,
  });
}

function resolveRequestLogScroller(rowsEl) {
  if (!rowsEl || typeof rowsEl.closest !== "function") {
    return null;
  }
  return rowsEl.closest(".requestlog-wrap");
}

async function onRequestLogRowsClick(event) {
  const target = event?.target;
  if (!target || typeof target.closest !== "function") {
    return;
  }
  const copyBtn = target.closest("button.path-copy");
  if (!copyBtn || !dom.requestLogRows || !dom.requestLogRows.contains(copyBtn)) {
    return;
  }
  const index = Number(copyBtn.dataset.logIndex);
  if (!Number.isInteger(index)) {
    return;
  }
  const rowItem = requestLogWindowState.filtered[index];
  const textToCopy = resolveDisplayRequestPath(rowItem) || rowItem?.requestPath || "";
  if (!textToCopy) {
    return;
  }
  const ok = await copyText(textToCopy);
  copyBtn.textContent = ok ? "已复制" : "失败";
  const token = String(Date.now());
  copyBtn.dataset.copyToken = token;
  setTimeout(() => {
    if (copyBtn.dataset.copyToken !== token) return;
    copyBtn.textContent = "复制";
  }, 900);
}

function onRequestLogScroll() {
  if (requestLogWindowState.scrollTickHandle != null) {
    return;
  }
  const flush = () => {
    requestLogWindowState.scrollTickHandle = null;
    requestLogWindowState.scrollTickMode = "";
    if (!isNearBottom(requestLogWindowState.boundScrollerEl, REQUEST_LOG_SCROLL_BUFFER)) {
      return;
    }
    appendNearBottomBatches({
      scroller: requestLogWindowState.boundScrollerEl,
      maxBatches: REQUEST_LOG_NEAR_BOTTOM_MAX_BATCHES,
      scrollBuffer: REQUEST_LOG_SCROLL_BUFFER,
      appendRequestLogBatch: appendRequestLogBatchLocal,
    });
  };
  if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
    requestLogWindowState.scrollTickMode = "raf";
    requestLogWindowState.scrollTickHandle = window.requestAnimationFrame(flush);
    return;
  }
  flush();
}

function cancelPendingScrollTick() {
  if (requestLogWindowState.scrollTickHandle == null) {
    return;
  }
  if (
    requestLogWindowState.scrollTickMode === "raf"
    && typeof window !== "undefined"
    && typeof window.cancelAnimationFrame === "function"
  ) {
    window.cancelAnimationFrame(requestLogWindowState.scrollTickHandle);
  } else {
    clearTimeout(requestLogWindowState.scrollTickHandle);
  }
  requestLogWindowState.scrollTickHandle = null;
  requestLogWindowState.scrollTickMode = "";
}

function ensureRequestLogBindings() {
  const rowsEl = dom.requestLogRows;
  if (!rowsEl || typeof rowsEl.addEventListener !== "function") {
    return;
  }
  if (requestLogWindowState.boundRowsEl && requestLogWindowState.boundRowsEl !== rowsEl) {
    requestLogWindowState.boundRowsEl.removeEventListener("click", onRequestLogRowsClick);
  }
  if (requestLogWindowState.boundRowsEl !== rowsEl) {
    rowsEl.addEventListener("click", onRequestLogRowsClick);
    requestLogWindowState.boundRowsEl = rowsEl;
  }
  const scroller = resolveRequestLogScroller(rowsEl);
  if (
    requestLogWindowState.boundScrollerEl &&
    requestLogWindowState.boundScrollerEl !== scroller
  ) {
    requestLogWindowState.boundScrollerEl.removeEventListener("scroll", onRequestLogScroll);
    cancelPendingScrollTick();
  }
  if (scroller && requestLogWindowState.boundScrollerEl !== scroller) {
    scroller.addEventListener("scroll", onRequestLogScroll, { passive: true });
    requestLogWindowState.boundScrollerEl = scroller;
  } else if (!scroller) {
    cancelPendingScrollTick();
    requestLogWindowState.boundScrollerEl = null;
  }
}

export function renderRequestLogs() {
  if (!dom.requestLogRows) {
    return;
  }
  ensureRequestLogBindings();
  ensureAccountLabelMap(state.accountList, requestLogWindowState);
  const filter = state.requestLogStatusFilter || "all";
  const { filtered, filteredKeys } = collectFilteredRequestLogs(
    state.requestLogList,
    filter,
  );
  const sameFilter = filter === requestLogWindowState.filter;
  const appendOnly = sameFilter && isAppendOnlyResult(
    requestLogWindowState.filteredKeys,
    filteredKeys,
  );
  const unchanged = appendOnly && filteredKeys.length === requestLogWindowState.filteredKeys.length;
  const canReuseRenderedDom = filtered.length > 0
    ? Boolean(
      requestLogWindowState.topSpacerRow &&
      dom.requestLogRows.contains(requestLogWindowState.topSpacerRow),
    )
    : dom.requestLogRows.children.length > 0;

  if (requestLogWindowState.hasRendered && canReuseRenderedDom && unchanged) {
    requestLogWindowState.filtered = filtered;
    requestLogWindowState.filteredKeys = filteredKeys;
    return;
  }

  if (
    requestLogWindowState.hasRendered &&
    appendOnly &&
    requestLogWindowState.topSpacerRow &&
    dom.requestLogRows.contains(requestLogWindowState.topSpacerRow)
  ) {
    const previousLength = requestLogWindowState.filtered.length;
    requestLogWindowState.filtered = filtered;
    requestLogWindowState.filteredKeys = filteredKeys;
    requestLogWindowState.filter = filter;
    if (
      requestLogWindowState.nextIndex >= previousLength ||
      isNearBottom(requestLogWindowState.boundScrollerEl, REQUEST_LOG_SCROLL_BUFFER)
    ) {
      appendAtLeastOneBatch({
        scroller: requestLogWindowState.boundScrollerEl,
        scrollBuffer: REQUEST_LOG_SCROLL_BUFFER,
        nearBottomMaxBatches: REQUEST_LOG_NEAR_BOTTOM_MAX_BATCHES,
        appendRequestLogBatch: appendRequestLogBatchLocal,
      });
    }
    return;
  }

  dom.requestLogRows.innerHTML = "";
  requestLogWindowState.filtered = filtered;
  requestLogWindowState.filteredKeys = filteredKeys;
  requestLogWindowState.filter = filter;
  requestLogWindowState.nextIndex = 0;
  requestLogWindowState.topSpacerHeight = 0;
  requestLogWindowState.recycledRowHeight = REQUEST_LOG_FALLBACK_ROW_HEIGHT;
  requestLogWindowState.topSpacerRow = null;
  requestLogWindowState.topSpacerCell = null;
  requestLogWindowState.hasRendered = true;
  if (!filtered.length) {
    renderEmptyRequestLogs(dom.requestLogRows, REQUEST_LOG_COLUMN_COUNT);
    return;
  }
  dom.requestLogRows.appendChild(
    createTopSpacerRow({
      columnCount: REQUEST_LOG_COLUMN_COUNT,
      windowState: requestLogWindowState,
    }),
  );
  appendAtLeastOneBatch({
    scroller: requestLogWindowState.boundScrollerEl,
    extraMaxBatches: 1,
    scrollBuffer: REQUEST_LOG_SCROLL_BUFFER,
    nearBottomMaxBatches: REQUEST_LOG_NEAR_BOTTOM_MAX_BATCHES,
    appendRequestLogBatch: appendRequestLogBatchLocal,
  });
}
