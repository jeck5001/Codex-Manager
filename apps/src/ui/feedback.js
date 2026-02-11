export function createFeedbackHandlers({ dom }) {
  let toastTimer = null;
  let toastQueue = [];
  let toastActive = false;

  function showToast(message, type = "info") {
    if (!message) return;
    if (!dom.appToast) {
      return;
    }
    toastQueue.push({ message: String(message), type });
    if (toastActive) return;
    const flushNext = () => {
      const item = toastQueue.shift();
      if (!item) {
        toastActive = false;
        return;
      }
      toastActive = true;
      dom.appToast.textContent = item.message;
      dom.appToast.classList.toggle("is-error", item.type === "error");
      dom.appToast.classList.add("active");
      if (toastTimer) {
        clearTimeout(toastTimer);
      }
      toastTimer = setTimeout(() => {
        dom.appToast.classList.remove("active");
        setTimeout(flushNext, 180);
      }, 2400);
    };
    flushNext();
  }

  function showConfirmDialog({
    title = "确认操作",
    message = "请确认是否继续。",
    confirmText = "确定",
    cancelText = "取消",
  } = {}) {
    if (
      !dom.modalConfirm
      || !dom.confirmTitle
      || !dom.confirmMessage
      || !dom.confirmOk
      || !dom.confirmCancel
    ) {
      return Promise.resolve(window.confirm(message));
    }
    dom.confirmTitle.textContent = title;
    dom.confirmMessage.textContent = message;
    dom.confirmOk.textContent = confirmText;
    dom.confirmCancel.textContent = cancelText;
    dom.modalConfirm.classList.add("active");
    return new Promise((resolve) => {
      let settled = false;
      const cleanup = () => {
        if (settled) return;
        settled = true;
        dom.confirmOk.removeEventListener("click", onOk);
        dom.confirmCancel.removeEventListener("click", onCancel);
        dom.modalConfirm.removeEventListener("click", onBackdropClick);
        document.removeEventListener("keydown", onKeydown);
        dom.modalConfirm.classList.remove("active");
      };
      const onOk = () => {
        cleanup();
        resolve(true);
      };
      const onCancel = () => {
        cleanup();
        resolve(false);
      };
      const onBackdropClick = (event) => {
        if (event.target === dom.modalConfirm) {
          onCancel();
        }
      };
      const onKeydown = (event) => {
        if (event.key === "Escape") {
          onCancel();
        }
      };
      dom.confirmOk.addEventListener("click", onOk, { once: true });
      dom.confirmCancel.addEventListener("click", onCancel, { once: true });
      dom.modalConfirm.addEventListener("click", onBackdropClick);
      document.addEventListener("keydown", onKeydown);
    });
  }

  return {
    showToast,
    showConfirmDialog,
  };
}
