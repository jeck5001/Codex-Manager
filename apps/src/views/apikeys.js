import { state } from "../state";
import { dom } from "../ui/dom";

// 渲染 API Key 列表
export function renderApiKeys({ onDisable, onDelete }) {
  dom.apiKeyRows.innerHTML = "";
  if (state.apiKeyList.length === 0) {
    const emptyRow = document.createElement("tr");
    const emptyCell = document.createElement("td");
    emptyCell.colSpan = 5;
    emptyCell.textContent = "暂无平台 Key";
    emptyRow.appendChild(emptyCell);
    dom.apiKeyRows.appendChild(emptyRow);
    return;
  }

  state.apiKeyList.forEach((item) => {
    const row = document.createElement("tr");
    const cellId = document.createElement("td");
    cellId.className = "mono";
    cellId.textContent = item.id;

    const cellName = document.createElement("td");
    cellName.textContent = item.name || "-";

    const cellStatus = document.createElement("td");
    cellStatus.textContent = item.status || "unknown";

    const cellUsed = document.createElement("td");
    cellUsed.textContent = item.lastUsedAt
      ? new Date(item.lastUsedAt * 1000).toLocaleString()
      : "-";

    const cellActions = document.createElement("td");
    const actionsWrap = document.createElement("div");
    actionsWrap.className = "cell-actions";
    const btnDisable = document.createElement("button");
    btnDisable.className = "secondary";
    btnDisable.textContent = "禁用";
    btnDisable.addEventListener("click", () => onDisable?.(item));
    const btnDelete = document.createElement("button");
    btnDelete.className = "danger";
    btnDelete.textContent = "删除";
    btnDelete.addEventListener("click", () => onDelete?.(item));
    actionsWrap.appendChild(btnDisable);
    actionsWrap.appendChild(btnDelete);
    cellActions.appendChild(actionsWrap);

    row.appendChild(cellId);
    row.appendChild(cellName);
    row.appendChild(cellStatus);
    row.appendChild(cellUsed);
    row.appendChild(cellActions);
    dom.apiKeyRows.appendChild(row);
  });
}

// 打开 API Key 弹窗
export function openApiKeyModal() {
  dom.modalApiKey.classList.add("active");
  dom.inputApiKeyName.value = "";
  dom.apiKeyValue.value = "";
}

// 关闭 API Key 弹窗
export function closeApiKeyModal() {
  dom.modalApiKey.classList.remove("active");
}
