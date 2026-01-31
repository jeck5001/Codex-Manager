import { state } from "../state";
import { dom } from "../ui/dom";

// 渲染 API Key 列表
export function renderApiKeys({ onDisable, onDelete }) {
  dom.apiKeyRows.innerHTML = "";
  if (state.apiKeyList.length === 0) {
    const empty = document.createElement("div");
    empty.className = "cell";
    empty.textContent = "暂无平台 Key";
    dom.apiKeyRows.appendChild(empty);
    return;
  }

  state.apiKeyList.forEach((item) => {
    const cellId = document.createElement("div");
    cellId.className = "cell mono";
    cellId.textContent = item.id;

    const cellName = document.createElement("div");
    cellName.className = "cell";
    cellName.textContent = item.name || "-";

    const cellStatus = document.createElement("div");
    cellStatus.className = "cell";
    cellStatus.textContent = item.status || "unknown";

    const cellUsed = document.createElement("div");
    cellUsed.className = "cell";
    cellUsed.textContent = item.lastUsedAt
      ? new Date(item.lastUsedAt * 1000).toLocaleString()
      : "-";

    const cellActions = document.createElement("div");
    cellActions.className = "cell";
    const btnDisable = document.createElement("button");
    btnDisable.className = "secondary";
    btnDisable.textContent = "禁用";
    btnDisable.addEventListener("click", () => onDisable?.(item));
    const btnDelete = document.createElement("button");
    btnDelete.className = "danger";
    btnDelete.textContent = "删除";
    btnDelete.addEventListener("click", () => onDelete?.(item));
    cellActions.appendChild(btnDisable);
    cellActions.appendChild(btnDelete);

    dom.apiKeyRows.appendChild(cellId);
    dom.apiKeyRows.appendChild(cellName);
    dom.apiKeyRows.appendChild(cellStatus);
    dom.apiKeyRows.appendChild(cellUsed);
    dom.apiKeyRows.appendChild(cellActions);
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
