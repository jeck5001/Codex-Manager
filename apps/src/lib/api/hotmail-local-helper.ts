import type {
  HotmailLocalHelperHealth,
  HotmailLocalHelperLaunchResult,
  RegisterHotmailLocalHandoff,
} from "../../types";

const HOTMAIL_LOCAL_HELPER_BASE = "http://127.0.0.1:16788";

function normalizeHotmailLocalHelperHealth(value: unknown): HotmailLocalHelperHealth {
  const source = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    ok: source.ok === true,
    service: typeof source.service === "string" ? source.service : "",
    version: typeof source.version === "string" ? source.version : "",
    playwrightReady:
      source.playwrightReady === true
        ? true
        : source.playwright_ready === true,
  };
}

function normalizeHotmailLocalHelperLaunchResult(value: unknown): HotmailLocalHelperLaunchResult {
  const source = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    ok: source.ok === true,
    handoffId:
      typeof source.handoffId === "string"
        ? source.handoffId
        : typeof source.handoff_id === "string"
          ? source.handoff_id
          : "",
    profileDir:
      typeof source.profileDir === "string"
        ? source.profileDir
        : typeof source.profile_dir === "string"
          ? source.profile_dir
          : "",
    message: typeof source.message === "string" ? source.message : "",
    error: typeof source.error === "string" ? source.error : undefined,
  };
}

export function buildHotmailLocalHelperUrl(path: string) {
  return `${HOTMAIL_LOCAL_HELPER_BASE}${path}`;
}

async function readJsonResponse(response: Response) {
  const payload = await response.json();
  if (response.ok) {
    return payload;
  }
  const payloadMessage =
    payload && typeof payload === "object"
      ? (payload as Record<string, unknown>).message
      : undefined;
  const message = typeof payloadMessage === "string" ? payloadMessage : `HTTP ${response.status}`;
  throw new Error(message);
}

export const hotmailLocalHelperClient = {
  async health(): Promise<HotmailLocalHelperHealth> {
    const response = await fetch(buildHotmailLocalHelperUrl("/health"));
    const payload = await readJsonResponse(response);
    return normalizeHotmailLocalHelperHealth(payload);
  },
  async openHandoff(
    payload: RegisterHotmailLocalHandoff
  ): Promise<HotmailLocalHelperLaunchResult> {
    const response = await fetch(buildHotmailLocalHelperUrl("/open-handoff"), {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });
    const json = await readJsonResponse(response);
    return normalizeHotmailLocalHelperLaunchResult(json);
  },
};
