export async function withButtonBusy(button, busyText, task) {
  if (!button) {
    return task();
  }
  if (button.dataset.busy === "1") {
    return;
  }
  const originalText = button.textContent;
  button.dataset.busy = "1";
  button.disabled = true;
  button.classList.add("is-loading");
  if (busyText) {
    button.textContent = busyText;
  }
  try {
    return await task();
  } finally {
    button.dataset.busy = "0";
    button.disabled = false;
    button.classList.remove("is-loading");
    button.textContent = originalText;
  }
}
