import { state } from "../state";
import { dom } from "../ui/dom";
import { calcAvailability, formatResetLabel, formatTs } from "../utils/format";
import { buildUsageRows } from "./usage-table";

// 渲染仪表盘视图
export function renderDashboard() {
  let okCount = 0;
  let warnCount = 0;
  let badCount = 0;
  let latestCapturedAt = null;

  const usageMap = new Map(
    state.usageList.map((item) => [item.accountId, item]),
  );

  state.accountList.forEach((account) => {
    const usage = usageMap.get(account.id);
    const status = calcAvailability(usage);
    if (status.level === "ok") okCount += 1;
    if (status.level === "warn") warnCount += 1;
    if (status.level === "bad") badCount += 1;
  });

  state.usageList.forEach((usage) => {
    if (
      usage.capturedAt &&
      (!latestCapturedAt || usage.capturedAt > latestCapturedAt)
    ) {
      latestCapturedAt = usage.capturedAt;
    }
  });

  dom.metricAvailable.textContent = okCount;
  dom.metricUnavailable.textContent = warnCount + badCount;
  dom.legendOk.textContent = okCount;
  dom.legendWarn.textContent = warnCount;
  dom.legendBad.textContent = badCount;

  const total = okCount + warnCount + badCount || 1;
  const okPercent = Math.round((okCount / total) * 100);
  const warnPercent = Math.round(((okCount + warnCount) / total) * 100);
  dom.statusDonut.style.setProperty("--ok-percent", `${okPercent}%`);
  dom.statusDonut.style.setProperty("--warn-percent", `${warnPercent}%`);
  dom.statusDonut.style.setProperty("--bad-percent", "100%");

  dom.usageBars.innerHTML = "";
  const header = document.createElement("div");
  header.className = "usage-row usage-header";
  header.innerHTML = "<div>账号</div><div>5小时</div><div>7天</div>";
  dom.usageBars.appendChild(header);

  const rows = buildUsageRows(state.accountList, state.usageList);
  rows.forEach((row) => {
    const line = document.createElement("div");
    line.className = "usage-row";
    line.appendChild(renderAccountCell(row));
    line.appendChild(
      renderUsageCell(row.primaryRemain, row.primaryResetsAt, false),
    );
    line.appendChild(
      renderUsageCell(row.secondaryRemain, row.secondaryResetsAt, true),
    );
    dom.usageBars.appendChild(line);
  });

  if (latestCapturedAt) {
    dom.statusUpdated.textContent = `最近刷新: ${formatTs(latestCapturedAt)}`;
  }
}

function renderAccountCell(row) {
  const cell = document.createElement("div");
  cell.className = "usage-account";
  const name = document.createElement("div");
  name.className = "usage-account-name";
  name.textContent = row.accountLabel;
  const sub = document.createElement("div");
  sub.className = "hint";
  sub.textContent = row.accountSub;
  cell.appendChild(name);
  cell.appendChild(sub);
  return cell;
}

function renderUsageCell(remain, resetsAt, isSecondary) {
  const cell = document.createElement("div");
  cell.className = "usage-cell";
  const track = document.createElement("div");
  track.className = "bar-track";
  const fill = document.createElement("div");
  fill.className = "bar-fill";
  if (isSecondary) {
    fill.classList.add("bar-fill-secondary");
  }
  fill.style.width = remain == null ? "0%" : `${remain}%`;
  track.appendChild(fill);
  const meta = document.createElement("div");
  meta.className = "usage-meta";
  const remainLabel = remain == null ? "n/a" : `${remain}% left`;
  meta.textContent = `${remainLabel} · ${formatResetLabel(resetsAt)}`;
  cell.appendChild(track);
  cell.appendChild(meta);
  return cell;
}
