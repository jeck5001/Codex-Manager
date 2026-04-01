export interface RequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
  retries?: number;
  retryDelayMs?: number;
  maxRetryDelayMs?: number;
  shouldRetry?: (error: unknown) => boolean;
  shouldRetryStatus?: (status: number) => boolean;
}

function isAbortLikeError(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}

function timeoutError(timeoutMs: number): Error {
  return new Error(`请求超时（${timeoutMs}ms）`);
}

function canceledError(): Error {
  return new Error("请求已取消");
}

export async function fetchWithRetry(
  url: string,
  init?: RequestInit,
  options: RequestOptions = {}
): Promise<Response> {
  const {
    timeoutMs = 10000,
    retries = 3,
    retryDelayMs = 200,
    maxRetryDelayMs = 3000,
    shouldRetryStatus = (status) => status >= 500 || status === 429,
  } = options;

  let lastError: unknown;
  for (let i = 0; i <= retries; i++) {
    const controller = new AbortController();
    let timedOut = false;
    const id = setTimeout(() => {
      timedOut = true;
      controller.abort(timeoutError(timeoutMs));
    }, timeoutMs);
    if (options.signal) {
      options.signal.addEventListener(
        "abort",
        () => controller.abort(options.signal?.reason ?? canceledError()),
        { once: true }
      );
    }

    try {
      const response = await fetch(url, {
        ...init,
        signal: controller.signal,
      });
      clearTimeout(id);

      if (response.ok || !shouldRetryStatus(response.status) || i === retries) {
        return response;
      }
    } catch (err: unknown) {
      clearTimeout(id);
      if (options.signal?.aborted) {
        throw canceledError();
      }
      if (timedOut && isAbortLikeError(err)) {
        lastError = timeoutError(timeoutMs);
        if (i === retries) {
          throw lastError;
        }
      } else {
        lastError = err;
      }
      if (!timedOut && i === retries) {
        throw err;
      }
    }

    const delay = Math.min(retryDelayMs * Math.pow(2, i), maxRetryDelayMs);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
  throw lastError || new Error("Fetch failed after retries");
}

export async function runWithControl<T>(
  fn: () => Promise<T>,
  options: RequestOptions = {}
): Promise<T> {
  const {
    retries = 0,
    retryDelayMs = 200,
    maxRetryDelayMs = 3000,
    shouldRetry = () => true,
  } = options;

  let lastError: unknown;
  for (let i = 0; i <= retries; i++) {
    try {
      return await fn();
    } catch (err: unknown) {
      lastError = err;
      if (i === retries || !shouldRetry(err)) {
        throw err;
      }
    }
    const delay = Math.min(retryDelayMs * Math.pow(2, i), maxRetryDelayMs);
    await new Promise((resolve) => setTimeout(resolve, delay));
  }
  throw lastError;
}
