import { state } from "../state.js";
import { dom } from "../ui/dom.js";
import { calcAvailability, formatTs, remainingPercent } from "../utils/format.js";
import { findUsage } from "./usage.js";

// 渲染账号列表
export function filterAccounts(accounts, usageList, query, filter) {
  const keyword = String(query || "").trim().toLowerCase();
  const usageMap = new Map(usageList.map((item) => [item.accountId, item]));
  return accounts.filter((account) => {
    if (keyword) {
      const label = String(account.label || "").toLowerCase();
      const id = String(account.id || "").toLowerCase();
      if (!label.includes(keyword) && !id.includes(keyword)) return false;
    }
    if (filter === "active" || filter === "low") {
      const usage = usageMap.get(account.id);
      const primaryRemain = remainingPercent(usage ? usage.usedPercent : null);
      const secondaryRemain = remainingPercent(
        usage ? usage.secondaryUsedPercent : null,
      );
      const status = calcAvailability(usage);
      if (filter === "active" && status.level !== "ok") return false;
      if (
        filter === "low" &&
        !(
          (primaryRemain != null && primaryRemain <= 20) ||
          (secondaryRemain != null && secondaryRemain <= 20)
        )
      ) {
        return false;
      }
    }
    return true;
  });
}

export function renderAccounts({ onUpdateSort, onOpenUsage, onDelete }) {
  dom.accountRows.innerHTML = "";
  const filtered = filterAccounts(
    state.accountList,
    state.usageList,
    state.accountSearch,
    state.accountFilter,
  );
  if (filtered.length === 0) {
    const empty = document.createElement("div");
    empty.className = "cell";
    empty.textContent = "暂无账号";
    dom.accountRows.appendChild(empty);
    return;
  }

  filtered.forEach((account) => {
    const usage = findUsage(account.id);
    const status = calcAvailability(usage);

    const cellAccount = document.createElement("div");
    cellAccount.className = "cell";
    const workspaceLabel = account.workspaceName
      ? ` · ${account.workspaceName}`
      : "";
    const primaryRemain = remainingPercent(usage ? usage.usedPercent : null);
    const secondaryRemain = remainingPercent(
      usage ? usage.secondaryUsedPercent : null,
    );
    cellAccount.innerHTML = `<strong>${account.label}</strong><small>${account.id}${workspaceLabel}</small>`;
    const mini = document.createElement("div");
    mini.className = "mini-usage";
    mini.appendChild(
      renderMiniUsageLine("5小时", primaryRemain, false),
    );
    mini.appendChild(
      renderMiniUsageLine("7天", secondaryRemain, true),
    );
    cellAccount.appendChild(mini);

    const cellGroup = document.createElement("div");
    cellGroup.className = "cell";
    cellGroup.textContent = account.groupName || "-";

    const cellTags = document.createElement("div");
    cellTags.className = "cell";
    cellTags.textContent = account.tags || "-";

    const cellSort = document.createElement("div");
    cellSort.className = "cell";
    const sortInput = document.createElement("input");
    sortInput.className = "sort-input";
    sortInput.type = "number";
    sortInput.value = account.sort != null ? String(account.sort) : "0";
    sortInput.addEventListener("change", async (event) => {
      const value = Number(event.target.value || 0);
      onUpdateSort?.(account.id, value);
    });
    cellSort.appendChild(sortInput);

    const cellStatus = document.createElement("div");
    cellStatus.className = "cell";
    const statusTag = document.createElement("span");
    statusTag.className = "status-tag";
    statusTag.textContent = status.text;
    if (status.level === "ok") statusTag.classList.add("status-ok");
    if (status.level === "warn") statusTag.classList.add("status-warn");
    if (status.level === "bad") statusTag.classList.add("status-bad");
    if (status.level === "unknown") statusTag.classList.add("status-unknown");
    cellStatus.appendChild(statusTag);

    const cellUpdated = document.createElement("div");
    cellUpdated.className = "cell";
    cellUpdated.innerHTML = `<strong>${usage && usage.capturedAt ? formatTs(usage.capturedAt) : "未知"}</strong>`;

    const cellActions = document.createElement("div");
    cellActions.className = "cell";
    const btn = document.createElement("button");
    btn.className = "secondary";
    btn.textContent = "用量查询";
    btn.addEventListener("click", () => onOpenUsage?.(account));
    cellActions.appendChild(btn);

    const del = document.createElement("button");
    del.className = "danger";
    del.textContent = "删除";
    del.addEventListener("click", () => onDelete?.(account));
    cellActions.appendChild(del);

    dom.accountRows.appendChild(cellAccount);
    dom.accountRows.appendChild(cellGroup);
    dom.accountRows.appendChild(cellTags);
    dom.accountRows.appendChild(cellSort);
    dom.accountRows.appendChild(cellStatus);
    dom.accountRows.appendChild(cellUpdated);
    dom.accountRows.appendChild(cellActions);
  });
}

function renderMiniUsageLine(label, remain, secondary) {
  const line = document.createElement("div");
  line.className = "progress-line";
  if (secondary) line.classList.add("secondary");
  const text = document.createElement("span");
  text.textContent = `${label} ${remain == null ? "--" : `${remain}%`}`;
  const track = document.createElement("div");
  track.className = "track";
  const fill = document.createElement("div");
  fill.className = "fill";
  fill.style.width = remain == null ? "0%" : `${remain}%`;
  track.appendChild(fill);
  line.appendChild(text);
  line.appendChild(track);
  return line;
}

// 打开账号登录弹窗
export function openAccountModal() {
  dom.modalAccount.classList.add("active");
  dom.loginUrl.value = "";
  if (dom.manualCallbackUrl) {
    dom.manualCallbackUrl.value = "";
  }
  dom.loginHint.textContent = "点击登录后会打开浏览器完成授权。";
  dom.inputNote.value = "";
  dom.inputTags.value = "";
  dom.inputGroup.value = "TEAM";
}

// 关闭账号登录弹窗
export function closeAccountModal() {
  dom.modalAccount.classList.remove("active");
}
