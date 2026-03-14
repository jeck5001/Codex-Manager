export function resolveBusinessErrorMessage(payload: unknown) {
  if (!payload || typeof payload !== "object") {
    return "";
  }

  const record = payload as Record<string, unknown>;
  if (record.ok === false) {
    if (typeof record.error === "string" && record.error.trim()) {
      return record.error;
    }
    return "操作失败";
  }

  if (typeof record.error === "string" && record.error.trim()) {
    return record.error;
  }

  return "";
}

export function unwrapRpcError(payload: unknown) {
  if (!payload || typeof payload !== "object") {
    return "";
  }
  const record = payload as Record<string, unknown>;
  const rpcError = record.error;
  if (!rpcError) {
    return "";
  }
  if (typeof rpcError === "string") {
    return rpcError;
  }
  if (typeof (rpcError as Record<string, unknown>).message === "string") {
    return String((rpcError as Record<string, unknown>).message);
  }
  return JSON.stringify(rpcError);
}

export function unwrapRpcResult<T>(payload: unknown) {
  const rpcError = unwrapRpcError(payload);
  if (rpcError) {
    throw new Error(rpcError);
  }

  const result =
    payload && typeof payload === "object" && Object.prototype.hasOwnProperty.call(payload, "result")
      ? (payload as Record<string, unknown>).result
      : payload;
  const businessError = resolveBusinessErrorMessage(result);
  if (businessError) {
    throw new Error(businessError);
  }
  return result as T;
}
