import { invoke, withAddr } from "./transport";
import { BackgroundTaskSettings, RequestLog } from "../../types";

export const serviceClient = {
  start: (addr?: string) => invoke("service_start", { addr }),
  stop: () => invoke("service_stop"),
  initialize: () => invoke("service_initialize", withAddr()),
  getStartupSnapshot: (params?: any) => invoke<any>("service_startup_snapshot", withAddr(params)),
  
  // Gateway Settings
  getGatewayTransport: () => invoke<any>("service_gateway_transport_get", withAddr()),
  setGatewayTransport: (settings: any) => invoke("service_gateway_transport_set", withAddr({ settings })),
  getUpstreamProxy: () => invoke<string>("service_gateway_upstream_proxy_get", withAddr()),
  setUpstreamProxy: (proxyUrl: string) => invoke("service_gateway_upstream_proxy_set", withAddr({ proxyUrl })),
  getRouteStrategy: () => invoke<string>("service_gateway_route_strategy_get", withAddr()),
  setRouteStrategy: (strategy: string) => invoke("service_gateway_route_strategy_set", withAddr({ strategy })),
  getHeaderPolicy: () => invoke<string>("service_gateway_header_policy_get", withAddr()),
  setHeaderPolicy: (cpaNoCookieHeaderModeEnabled: boolean) => 
    invoke("service_gateway_header_policy_set", withAddr({ cpaNoCookieHeaderModeEnabled })),
  
  // Background Tasks
  getBackgroundTasks: () => invoke<BackgroundTaskSettings>("service_gateway_background_tasks_get", withAddr()),
  setBackgroundTasks: (settings: BackgroundTaskSettings) => 
    invoke("service_gateway_background_tasks_set", withAddr(settings)),
    
  // Request Logs
  listRequestLogs: (query: string, limit: number) => 
    invoke<RequestLog[]>("service_requestlog_list", withAddr({ query, limit })),
  clearRequestLogs: () => invoke("service_requestlog_clear", withAddr()),
  getTodaySummary: () => invoke<any>("service_requestlog_today_summary", withAddr()),
  
  // Listen Config
  getListenConfig: () => invoke<any>("service_listen_config_get", withAddr()),
  setListenConfig: (mode: string) => invoke("service_listen_config_set", withAddr({ mode })),

  // Env Overrides
  getEnvOverrides: () => invoke<any>("app_window_unsaved_draft_sections_set", withAddr()), // Note: Check actual command name
};

