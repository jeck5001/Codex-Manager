export type WebCommandRequestOptions = {
  timeoutMs?: number;
  retries?: number;
};

// account/auth/recover 在最慢情况下会走自动补号链路；
// 后端轮询超时窗口当前是 20 分钟，所以 Web 端不能再用 120 秒短超时，
// 否则会在恢复仍在进行时被浏览器层主动 abort。
const ACCOUNT_AUTH_RECOVERY_TIMEOUT_MS = 25 * 60 * 1000;
const ACCOUNT_CPA_SYNC_TIMEOUT_MS = 5 * 60 * 1000;

const WEB_COMMAND_REQUEST_OPTIONS: Record<string, WebCommandRequestOptions> = {
  service_account_auth_recover: {
    timeoutMs: ACCOUNT_AUTH_RECOVERY_TIMEOUT_MS,
    retries: 0,
  },
  service_account_cpa_sync: {
    timeoutMs: ACCOUNT_CPA_SYNC_TIMEOUT_MS,
    retries: 0,
  },
};

export function resolveWebCommandRequestOptions(
  method: string,
  options: WebCommandRequestOptions = {}
): WebCommandRequestOptions {
  return {
    ...(WEB_COMMAND_REQUEST_OPTIONS[method] ?? {}),
    ...options,
  };
}
