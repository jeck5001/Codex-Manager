import { dom } from "./dom.js";
import { state } from "../state.js";

// 设置顶部状态提示
export function setStatus(message, ok) {
  if (!dom.statusEl) return;
  if (message) {
    dom.statusEl.textContent = message;
    return;
  }
  if (ok) {
    dom.statusEl.textContent = "已连接";
  } else if (state.serviceAddr) {
    dom.statusEl.textContent = "未连接";
  } else {
    dom.statusEl.textContent = "未启动";
  }
}

// 设置服务提示信息
export function setServiceHint(text, isError = false) {
  if (!dom.serviceHint) return;
  dom.serviceHint.textContent = text || "";
  dom.serviceHint.classList.toggle("error", Boolean(isError));
}
