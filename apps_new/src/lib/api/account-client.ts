import { invoke, withAddr } from "./transport";
import { Account, AccountUsage, ApiKey } from "../../types";

export const accountClient = {
  list: (params?: any) => invoke<Account[]>("service_account_list", withAddr(params)),
  delete: (accountId: string) => invoke("service_account_delete", withAddr({ accountId })),
  deleteMany: (accountIds: string[]) => invoke("service_account_delete_many", withAddr({ accountIds })),
  deleteUnavailableFree: () => invoke("service_account_delete_unavailable_free", withAddr()),
  update: (accountId: string, sort: number) => invoke("service_account_update", withAddr({ accountId, sort })),
  import: (contents: any[]) => invoke("service_account_import", withAddr({ contents })),
  importByDirectory: () => invoke("service_account_import_by_directory", withAddr()),
  export: () => invoke<any>("service_account_export_by_account_files", withAddr()),
  
  // Usage
  getUsage: (accountId: string) => invoke<AccountUsage>("service_usage_read", withAddr({ accountId })),
  listUsage: () => invoke<AccountUsage[]>("service_usage_list", withAddr()),
  refreshUsage: (accountId: string) => invoke("service_usage_refresh", withAddr({ accountId })),
  aggregateUsage: () => invoke<any>("service_usage_aggregate", withAddr()),

  // Login
  startLogin: (params: any) => invoke<string>("service_login_start", withAddr(params)),
  getLoginStatus: (loginId: string) => invoke<any>("service_login_status", withAddr({ loginId })),
  completeLogin: (state: string, code: string, redirectUri: string) => 
    invoke("service_login_complete", withAddr({ state, code, redirectUri })),

  // API Keys (Fixed underscores and param names)
  listApiKeys: () => invoke<ApiKey[]>("service_apikey_list", withAddr()),
  createApiKey: (params: any) => invoke<ApiKey>("service_apikey_create", withAddr({
    name: params.name,
    model_slug: params.modelSlug,
    reasoning_effort: params.reasoningEffort,
    protocol_type: params.protocolType,
    upstream_base_url: params.upstreamBaseUrl,
    static_headers_json: params.staticHeadersJson,
  })),
  deleteApiKey: (keyId: string) => invoke("service_apikey_delete", withAddr({ keyId })),
  updateApiKey: (keyId: string, params: any) => invoke("service_apikey_update_model", withAddr({
    key_id: keyId,
    model_slug: params.modelSlug,
    reasoning_effort: params.reasoningEffort,
    protocol_type: params.protocolType,
    upstream_base_url: params.upstreamBaseUrl,
    static_headers_json: params.staticHeadersJson,
  })),
  disableApiKey: (keyId: string) => invoke("service_apikey_disable", withAddr({ keyId })),
  enableApiKey: (keyId: string) => invoke("service_apikey_enable", withAddr({ keyId })),
  listModels: (refreshRemote?: boolean) => invoke<string[]>("service_apikey_models", withAddr({ refreshRemote })),
  readApiKeySecret: (keyId: string) => invoke<string>("service_apikey_read_secret", withAddr({ key_id: keyId })),
};
