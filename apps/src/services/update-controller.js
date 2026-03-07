function getPathValue(source, path) {
  const steps = String(path || "").split(".");
  let cursor = source;
  for (const step of steps) {
    if (!cursor || typeof cursor !== "object" || !(step in cursor)) {
      return undefined;
    }
    cursor = cursor[step];
  }
  return cursor;
}

function pickFirstValue(source, paths) {
  for (const path of paths || []) {
    const value = getPathValue(source, path);
    if (value !== undefined && value !== null && String(value) !== "") {
      return value;
    }
  }
  return null;
}

function pickBooleanValue(source, paths) {
  const value = pickFirstValue(source, paths);
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value !== 0;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (["1", "true", "yes", "on"].includes(normalized)) {
      return true;
    }
    if (["0", "false", "no", "off"].includes(normalized)) {
      return false;
    }
  }
  return null;
}

export function normalizeUpdateInfo(source) {
  const payload = source && typeof source === "object" ? source : {};
  const explicitAvailable = pickBooleanValue(payload, [
    "hasUpdate",
    "available",
    "updateAvailable",
    "has_upgrade",
    "has_update",
    "needUpdate",
    "need_update",
    "result.hasUpdate",
    "result.available",
    "result.updateAvailable",
  ]);
  const explicitlyLatest = pickBooleanValue(payload, [
    "isLatest",
    "upToDate",
    "noUpdate",
    "result.isLatest",
    "result.upToDate",
  ]);
  const hintedVersion = pickFirstValue(payload, [
    "targetVersion",
    "latestVersion",
    "newVersion",
    "release.version",
    "manifest.version",
    "result.targetVersion",
    "result.latestVersion",
  ]);
  let available = explicitAvailable;
  if (available == null) {
    available = explicitlyLatest === true ? false : hintedVersion != null;
  }

  const packageTypeValue = pickFirstValue(payload, [
    "packageType",
    "package_type",
    "distributionType",
    "distribution_type",
    "updateType",
    "update_type",
    "installType",
    "install_type",
    "release.packageType",
    "result.packageType",
  ]);
  const packageType = packageTypeValue == null ? "" : String(packageTypeValue).toLowerCase();
  const portableFlag = pickBooleanValue(payload, [
    "isPortable",
    "portable",
    "release.isPortable",
    "result.isPortable",
  ]);
  const hasPortableHint = portableFlag != null || Boolean(packageType);
  const isPortable = portableFlag === true || packageType.includes("portable");
  const versionValue = pickFirstValue(payload, [
    "latestVersion",
    "targetVersion",
    "newVersion",
    "version",
    "release.version",
    "manifest.version",
    "result.latestVersion",
    "result.targetVersion",
    "result.version",
  ]);
  const downloaded = pickBooleanValue(payload, [
    "downloaded",
    "isDownloaded",
    "readyToInstall",
    "ready",
    "result.downloaded",
    "result.readyToInstall",
  ]) === true;
  const canPrepareValue = pickBooleanValue(payload, [
    "canPrepare",
    "result.canPrepare",
  ]);
  const reasonValue = pickFirstValue(payload, [
    "reason",
    "message",
    "error",
    "result.reason",
    "result.message",
  ]);
  return {
    available: Boolean(available),
    version: versionValue == null ? "" : String(versionValue).trim(),
    isPortable,
    hasPortableHint,
    downloaded,
    canPrepare: canPrepareValue !== false,
    reason: reasonValue == null ? "" : String(reasonValue),
  };
}

export function buildVersionLabel(version) {
  if (!version) {
    return "";
  }
  const clean = String(version).trim();
  if (!clean) {
    return "";
  }
  return clean.startsWith("v") ? ` ${clean}` : ` v${clean}`;
}

export function createUpdateController(deps = {}) {
  const {
    dom = {},
    showToast = () => {},
    showConfirmDialog = async () => false,
    normalizeErrorMessage = (err) => String(err?.message || err || ""),
    isTauriRuntime = () => false,
    readUpdateAutoCheckSetting = () => false,
    updateCheck = async () => ({}),
    updateDownload = async () => ({}),
    updateInstall = async () => {},
    updateRestart = async () => {},
    updateStatus = async () => ({}),
    withButtonBusy = async (_button, _text, task) => task(),
    nextPaintTick = () => Promise.resolve(),
    updateCheckDelayMs = 1200,
    setTimeoutFn = setTimeout,
  } = deps;

  let updateCheckInFlight = null;
  let pendingUpdateCandidate = null;

  function setUpdateStatusText(message) {
    if (!dom.updateStatusText) return;
    dom.updateStatusText.textContent = message || "尚未检查更新";
  }

  function setCurrentVersionText(version) {
    if (!dom.updateCurrentVersion) return;
    const clean = version == null ? "" : String(version).trim();
    dom.updateCurrentVersion.textContent = clean
      ? (clean.startsWith("v") ? clean : `v${clean}`)
      : "--";
  }

  function setCheckUpdateButtonLabel() {
    if (!dom.checkUpdate) return;
    if (pendingUpdateCandidate && pendingUpdateCandidate.version && pendingUpdateCandidate.canPrepare) {
      const version = String(pendingUpdateCandidate.version).trim();
      const display = version.startsWith("v") ? version : `v${version}`;
      dom.checkUpdate.textContent = `更新到 ${display}`;
      return;
    }
    dom.checkUpdate.textContent = "检查更新";
  }

  async function promptUpdateReady(info) {
    const versionLabel = buildVersionLabel(info.version);
    if (info.isPortable) {
      const shouldRestart = await showConfirmDialog({
        title: "更新已下载",
        message: `新版本${versionLabel}已下载完成，重启应用即可更新。是否现在重启？`,
        confirmText: "立即重启",
        cancelText: "稍后",
      });
      if (!shouldRestart) {
        return;
      }
      try {
        await updateRestart();
      } catch (err) {
        console.error("[update] restart failed", err);
        showToast(`重启更新失败：${normalizeErrorMessage(err)}`, "error");
      }
      return;
    }

    const shouldInstall = await showConfirmDialog({
      title: "更新已下载",
      message: `新版本${versionLabel}已下载完成，是否立即安装更新？`,
      confirmText: "立即安装",
      cancelText: "稍后",
    });
    if (!shouldInstall) {
      return;
    }
    try {
      await updateInstall();
    } catch (err) {
      console.error("[update] install failed", err);
      showToast(`安装更新失败：${normalizeErrorMessage(err)}`, "error");
    }
  }

  async function runUpdateCheckFlow({ silentIfLatest = false } = {}) {
    if (!isTauriRuntime()) {
      if (!silentIfLatest) {
        showToast("仅桌面端支持检查更新");
      }
      return false;
    }
    if (updateCheckInFlight) {
      return updateCheckInFlight;
    }
    updateCheckInFlight = (async () => {
      try {
        const checkResult = await updateCheck();
        const checkInfo = normalizeUpdateInfo(checkResult);
        if (!checkInfo.available) {
          pendingUpdateCandidate = null;
          setCheckUpdateButtonLabel();
          setUpdateStatusText("当前已是最新版本");
          if (!silentIfLatest) {
            showToast("当前已是最新版本");
          }
          return false;
        }

        if (!checkInfo.canPrepare) {
          pendingUpdateCandidate = null;
          setCheckUpdateButtonLabel();
          const message = checkInfo.reason
            || `发现新版本${buildVersionLabel(checkInfo.version)}，当前仅可查看版本`;
          setUpdateStatusText(message);
          if (!silentIfLatest) {
            showToast(message);
          }
          return true;
        }

        pendingUpdateCandidate = {
          version: checkInfo.version,
          isPortable: checkInfo.isPortable,
          canPrepare: true,
        };
        setCheckUpdateButtonLabel();

        const tip = `发现新版本${buildVersionLabel(checkInfo.version)}，再次点击可更新`;
        setUpdateStatusText(tip);
        if (!silentIfLatest) {
          showToast(tip);
        }
        return true;
      } catch (err) {
        console.error("[update] check/download failed", err);
        pendingUpdateCandidate = null;
        setCheckUpdateButtonLabel();
        setUpdateStatusText(`检查失败：${normalizeErrorMessage(err)}`);
        showToast(`检查更新失败：${normalizeErrorMessage(err)}`, "error");
        return false;
      }
    })();

    try {
      return await updateCheckInFlight;
    } finally {
      updateCheckInFlight = null;
    }
  }

  async function runUpdateApplyFlow() {
    if (!pendingUpdateCandidate || !pendingUpdateCandidate.canPrepare) {
      showToast("当前更新只支持版本检查，请稍后重试");
      return false;
    }
    const checkVersionLabel = buildVersionLabel(pendingUpdateCandidate.version);
    try {
      showToast(`正在下载新版本${checkVersionLabel}...`);
      const downloadResult = await updateDownload();
      const downloadInfo = normalizeUpdateInfo(downloadResult);
      const finalInfo = {
        version: downloadInfo.version || pendingUpdateCandidate.version,
        isPortable: downloadInfo.hasPortableHint ? downloadInfo.isPortable : pendingUpdateCandidate.isPortable,
      };
      setUpdateStatusText(`新版本 ${finalInfo.version || ""} 已下载，等待安装`);
      await promptUpdateReady(finalInfo);
      pendingUpdateCandidate = null;
      setCheckUpdateButtonLabel();
      return true;
    } catch (err) {
      console.error("[update] apply failed", err);
      setUpdateStatusText(`更新失败：${normalizeErrorMessage(err)}`);
      showToast(`更新失败：${normalizeErrorMessage(err)}`, "error");
      return false;
    }
  }

  async function handleCheckUpdateClick() {
    const hasPreparedCheck = Boolean(
      pendingUpdateCandidate && pendingUpdateCandidate.version && pendingUpdateCandidate.canPrepare
    );
    const busyText = hasPreparedCheck ? "更新中..." : "检查中...";
    await withButtonBusy(dom.checkUpdate, busyText, async () => {
      await nextPaintTick();
      if (hasPreparedCheck) {
        await runUpdateApplyFlow();
        return;
      }
      await runUpdateCheckFlow({ silentIfLatest: false });
    });
    setCheckUpdateButtonLabel();
  }

  function scheduleStartupUpdateCheck() {
    if (!readUpdateAutoCheckSetting()) {
      return;
    }
    setTimeoutFn(() => {
      void runUpdateCheckFlow({ silentIfLatest: true });
    }, updateCheckDelayMs);
  }

  async function bootstrapUpdateStatus() {
    if (!isTauriRuntime()) {
      setCurrentVersionText("--");
      setUpdateStatusText("仅桌面端支持更新");
      return;
    }
    try {
      const status = await updateStatus();
      const current = status && status.currentVersion ? String(status.currentVersion) : "";
      setCurrentVersionText(current);
      setUpdateStatusText("尚未检查更新");
      setCheckUpdateButtonLabel();
    } catch {
      setCurrentVersionText("--");
      setUpdateStatusText("尚未检查更新");
      setCheckUpdateButtonLabel();
    }
  }

  return {
    handleCheckUpdateClick,
    scheduleStartupUpdateCheck,
    bootstrapUpdateStatus,
  };
}
