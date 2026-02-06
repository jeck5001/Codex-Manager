import { dom } from "../ui/dom";
import { state } from "../state";

function formatTs(ts) {
  if (!ts) return "-";
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
}

export function renderRequestLogs() {
  dom.requestLogRows.innerHTML = "";
  if (!state.requestLogList.length) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 8;
    cell.textContent = "暂无请求日志";
    row.appendChild(cell);
    dom.requestLogRows.appendChild(row);
    return;
  }

  state.requestLogList.forEach((item) => {
    const row = document.createElement("tr");
    const values = [
      formatTs(item.createdAt),
      item.keyId || "-",
      item.method || "-",
      item.requestPath || "-",
      item.model || "-",
      item.reasoningEffort || "-",
      item.statusCode == null ? "-" : String(item.statusCode),
      item.error || "-",
    ];
    values.forEach((value) => {
      const cell = document.createElement("td");
      cell.textContent = value;
      row.appendChild(cell);
    });
    dom.requestLogRows.appendChild(row);
  });
}
