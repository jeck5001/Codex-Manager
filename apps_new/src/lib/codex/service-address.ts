const LOOPBACK_HOSTS = new Set(["127.0.0.1", "0.0.0.0"]);

export function normalizeServiceAddress(raw: string) {
  const trimmed = String(raw || "").trim();
  if (!trimmed) {
    throw new Error("请输入端口或地址");
  }

  let value = trimmed;
  if (value.startsWith("http://")) {
    value = value.slice("http://".length);
  }
  if (value.startsWith("https://")) {
    value = value.slice("https://".length);
  }

  value = value.split("/")[0] || value;
  if (/^\d+$/.test(value)) {
    return `localhost:${value}`;
  }

  const [host, port] = value.split(":");
  if (!port) {
    return value;
  }

  if (LOOPBACK_HOSTS.has(host)) {
    return `localhost:${port}`;
  }

  return value;
}
