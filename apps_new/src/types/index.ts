export interface ServiceStatus {
  connected: boolean;
  version: string;
  uptime: number;
  addr: string;
}

export interface Account {
  id: string;
  name: string;
  group: string;
  priority: number;
  tags: string[];
  note: string;
  is_available: boolean;
  is_low_quota: boolean;
  last_refresh_at?: string;
  usage?: AccountUsage;
}

export interface AccountUsage {
  total: number;
  used: number;
  remaining: number;
  refresh_at?: string;
}

export interface ApiKey {
  id: string;
  name: string;
  protocol: "openai_compat" | "azure_openai" | "anthropic_native";
  model: string;
  reasoning_level?: string;
  status: "enabled" | "disabled";
  secret?: string;
  endpoint?: string;
}

export interface RequestLog {
  id: string;
  timestamp: string;
  account_name: string;
  api_key_id: string;
  method: string;
  path: string;
  model: string;
  level: string;
  status: number;
  error?: string;
}

export interface AppSettings {
  updateAutoCheck: boolean;
  closeToTrayOnClose: boolean;
  lowTransparency: boolean;
  lightweightModeOnCloseToTray: boolean;
  webAccessPasswordConfigured: boolean;
  serviceAddr: string;
  [key: string]: any;
}

export interface GatewaySettings {
  route_strategy: "ordered" | "balanced";
  header_policy: "converged" | "passthrough";
  upstream_proxy?: string;
  sse_keepalive_interval_ms: number;
  upstream_stream_timeout_ms: number;
}

export interface BackgroundTaskSettings {
  usage_polling_enabled: boolean;
  usage_poll_interval_secs: number;
  gateway_keepalive_enabled: boolean;
  gateway_keepalive_interval_secs: number;
  token_refresh_polling_enabled: boolean;
  token_refresh_poll_interval_secs: number;
  usage_refresh_workers: number;
  http_worker_factor: number;
  http_worker_min: number;
  http_stream_worker_factor: number;
  http_stream_worker_min: number;
}
