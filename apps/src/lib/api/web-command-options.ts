export type WebCommandRequestOptions = {
  timeoutMs?: number;
  retries?: number;
};

const WEB_COMMAND_REQUEST_OPTIONS: Record<string, WebCommandRequestOptions> = {
  service_account_auth_recover: {
    timeoutMs: 120000,
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
