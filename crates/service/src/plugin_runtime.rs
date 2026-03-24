use bytes::Bytes;
use codexmanager_core::storage::{Account, PluginRecord, Storage};
use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, Value as LuaValue, VmState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const DEFAULT_PLUGIN_REJECT_STATUS: u16 = 403;
const LUA_HOOK_INSTRUCTION_INTERVAL: u32 = 1_000;
const RUNTIME_LUA: &str = "lua";
const PLUGIN_CACHE_UNKNOWN: u8 = 0;
const PLUGIN_CACHE_DISABLED: u8 = 1;
const PLUGIN_CACHE_ENABLED: u8 = 2;

static ENABLED_PLUGIN_CACHE: AtomicU8 = AtomicU8::new(PLUGIN_CACHE_UNKNOWN);

#[derive(Debug, Clone, Copy)]
pub(crate) enum PluginHookPoint {
    PreRoute,
    PostRoute,
    PostResponse,
}

impl PluginHookPoint {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PreRoute => "pre_route",
            Self::PostRoute => "post_route",
            Self::PostResponse => "post_response",
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedPlugin {
    id: String,
    name: String,
    script_content: String,
    timeout_ms: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginRequestPatch {
    pub(crate) body: Bytes,
    pub(crate) model_for_log: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginRejectResponse {
    pub(crate) plugin_id: String,
    pub(crate) plugin_name: String,
    pub(crate) status_code: u16,
    pub(crate) body: Value,
    pub(crate) message: String,
}

pub(crate) enum RequestHookOutcome {
    Continue(PluginRequestPatch),
    Reject(PluginRejectResponse),
}

pub(crate) struct PreRoutePluginInput<'a> {
    pub(crate) storage: &'a Storage,
    pub(crate) trace_id: &'a str,
    pub(crate) key_id: &'a str,
    pub(crate) api_key_name: Option<&'a str>,
    pub(crate) path: &'a str,
    pub(crate) method: &'a str,
    pub(crate) body: &'a Bytes,
    pub(crate) model_for_log: Option<&'a str>,
    pub(crate) is_stream: bool,
}

pub(crate) struct PostRoutePluginInput<'a> {
    pub(crate) storage: &'a Storage,
    pub(crate) trace_id: &'a str,
    pub(crate) key_id: &'a str,
    pub(crate) api_key_name: Option<&'a str>,
    pub(crate) path: &'a str,
    pub(crate) method: &'a str,
    pub(crate) body: &'a Bytes,
    pub(crate) model_for_log: Option<&'a str>,
    pub(crate) is_stream: bool,
    pub(crate) account: &'a Account,
    pub(crate) route_strategy: &'a str,
}

pub(crate) struct PostResponsePluginInput<'a> {
    pub(crate) storage: &'a Storage,
    pub(crate) trace_id: &'a str,
    pub(crate) key_id: &'a str,
    pub(crate) api_key_name: Option<&'a str>,
    pub(crate) path: &'a str,
    pub(crate) method: &'a str,
    pub(crate) body: &'a Bytes,
    pub(crate) model_for_log: Option<&'a str>,
    pub(crate) is_stream: bool,
    pub(crate) account: &'a Account,
    pub(crate) route_strategy: &'a str,
    pub(crate) status_code: u16,
    pub(crate) response_headers: &'a reqwest::header::HeaderMap,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct LuaHandleResult {
    action: Option<String>,
    status: Option<u16>,
    message: Option<String>,
    body: Option<Value>,
    model: Option<String>,
    request: Option<LuaRequestPatch>,
    annotations: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct LuaRequestPatch {
    model: Option<String>,
    body: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaRequestContext {
    path: String,
    method: String,
    model: Option<String>,
    stream: bool,
    body: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaApiKeyContext {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaRouteContext {
    strategy: String,
    selected_account_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaAccountContext {
    id: String,
    label: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaResponseContext {
    status: u16,
    headers: Map<String, Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaPreRouteContext {
    request: LuaRequestContext,
    api_key: LuaApiKeyContext,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaPostRouteContext {
    request: LuaRequestContext,
    api_key: LuaApiKeyContext,
    route: LuaRouteContext,
    account: LuaAccountContext,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaPostResponseContext {
    request: LuaRequestContext,
    api_key: LuaApiKeyContext,
    route: LuaRouteContext,
    account: LuaAccountContext,
    response: LuaResponseContext,
}

pub(crate) fn validate_plugin_script(
    runtime: &str,
    script_content: &str,
    timeout_ms: i64,
) -> Result<(), String> {
    if !runtime.eq_ignore_ascii_case(RUNTIME_LUA) {
        return Ok(());
    }
    let plugin = LoadedPlugin {
        id: "plugin_validation".to_string(),
        name: "plugin_validation".to_string(),
        script_content: script_content.to_string(),
        timeout_ms,
    };
    let context = LuaPreRouteContext {
        request: LuaRequestContext {
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            model: Some("o3".to_string()),
            stream: false,
            body: json!({ "model": "o3", "input": "ping" }),
        },
        api_key: LuaApiKeyContext {
            id: "gk_validation".to_string(),
            name: Some("验证 Key".to_string()),
        },
    };
    execute_lua_plugin(&plugin, &context)?;
    Ok(())
}

pub(crate) fn refresh_plugin_cache(storage: &Storage) {
    let has_enabled_plugins = storage
        .list_plugins()
        .map(|items| {
            items
                .into_iter()
                .any(|item| item.enabled && item.runtime.eq_ignore_ascii_case(RUNTIME_LUA))
        })
        .unwrap_or_else(|err| {
            log::warn!("refresh plugin cache failed: {}", err);
            false
        });
    ENABLED_PLUGIN_CACHE.store(
        if has_enabled_plugins {
            PLUGIN_CACHE_ENABLED
        } else {
            PLUGIN_CACHE_DISABLED
        },
        Ordering::Relaxed,
    );
}

pub(crate) fn execute_pre_route_plugins(input: PreRoutePluginInput<'_>) -> RequestHookOutcome {
    if !has_enabled_lua_plugins(input.storage) {
        return RequestHookOutcome::Continue(PluginRequestPatch {
            body: input.body.clone(),
            model_for_log: input.model_for_log.map(str::to_string),
        });
    }
    let context = LuaPreRouteContext {
        request: build_request_context(
            input.path,
            input.method,
            input.body,
            input.model_for_log,
            input.is_stream,
        ),
        api_key: LuaApiKeyContext {
            id: input.key_id.to_string(),
            name: input.api_key_name.map(str::to_string),
        },
    };
    execute_request_hook_plugins(
        input.storage,
        input.trace_id,
        PluginHookPoint::PreRoute,
        &context,
        input.body,
        input.model_for_log,
    )
}

pub(crate) fn execute_post_route_plugins(input: PostRoutePluginInput<'_>) -> RequestHookOutcome {
    if !has_enabled_lua_plugins(input.storage) {
        return RequestHookOutcome::Continue(PluginRequestPatch {
            body: input.body.clone(),
            model_for_log: input.model_for_log.map(str::to_string),
        });
    }
    let context = LuaPostRouteContext {
        request: build_request_context(
            input.path,
            input.method,
            input.body,
            input.model_for_log,
            input.is_stream,
        ),
        api_key: LuaApiKeyContext {
            id: input.key_id.to_string(),
            name: input.api_key_name.map(str::to_string),
        },
        route: LuaRouteContext {
            strategy: input.route_strategy.to_string(),
            selected_account_id: input.account.id.clone(),
        },
        account: LuaAccountContext {
            id: input.account.id.clone(),
            label: input.account.label.clone(),
            status: input.account.status.clone(),
        },
    };
    execute_request_hook_plugins(
        input.storage,
        input.trace_id,
        PluginHookPoint::PostRoute,
        &context,
        input.body,
        input.model_for_log,
    )
}

pub(crate) fn execute_post_response_plugins(input: PostResponsePluginInput<'_>) {
    if !has_enabled_lua_plugins(input.storage) {
        return;
    }
    let context = LuaPostResponseContext {
        request: build_request_context(
            input.path,
            input.method,
            input.body,
            input.model_for_log,
            input.is_stream,
        ),
        api_key: LuaApiKeyContext {
            id: input.key_id.to_string(),
            name: input.api_key_name.map(str::to_string),
        },
        route: LuaRouteContext {
            strategy: input.route_strategy.to_string(),
            selected_account_id: input.account.id.clone(),
        },
        account: LuaAccountContext {
            id: input.account.id.clone(),
            label: input.account.label.clone(),
            status: input.account.status.clone(),
        },
        response: LuaResponseContext {
            status: input.status_code,
            headers: build_response_headers_map(input.response_headers),
        },
    };
    let plugins = match load_enabled_plugins(input.storage, PluginHookPoint::PostResponse) {
        Ok(plugins) => plugins,
        Err(err) => {
            log::warn!("load post_response plugins failed: {}", err);
            return;
        }
    };
    for plugin in plugins {
        match execute_lua_plugin(&plugin, &context) {
            Ok(result) => {
                let detail = result.message.clone().unwrap_or_default();
                crate::gateway::log_plugin_hook(
                    input.trace_id,
                    plugin.id.as_str(),
                    PluginHookPoint::PostResponse.as_str(),
                    normalize_action(result.action.as_deref()).unwrap_or("continue"),
                    (!detail.trim().is_empty()).then_some(detail.as_str()),
                );
                if let Some(annotations) = result.annotations {
                    log_plugin_annotations(
                        input.trace_id,
                        plugin.id.as_str(),
                        plugin.name.as_str(),
                        PluginHookPoint::PostResponse.as_str(),
                        &annotations,
                    );
                }
            }
            Err(err) => {
                crate::gateway::log_plugin_hook(
                    input.trace_id,
                    plugin.id.as_str(),
                    PluginHookPoint::PostResponse.as_str(),
                    "runtime_error",
                    Some(err.as_str()),
                );
                log::warn!(
                    "event=plugin_runtime_error trace_id={} plugin_id={} hook_point={} error={}",
                    input.trace_id,
                    plugin.id,
                    PluginHookPoint::PostResponse.as_str(),
                    err
                );
            }
        }
    }
}

fn execute_request_hook_plugins<T: Serialize>(
    storage: &Storage,
    trace_id: &str,
    hook_point: PluginHookPoint,
    context: &T,
    initial_body: &Bytes,
    initial_model_for_log: Option<&str>,
) -> RequestHookOutcome {
    let plugins = match load_enabled_plugins(storage, hook_point) {
        Ok(plugins) => plugins,
        Err(err) => {
            log::warn!("load {} plugins failed: {}", hook_point.as_str(), err);
            return RequestHookOutcome::Continue(PluginRequestPatch {
                body: initial_body.clone(),
                model_for_log: initial_model_for_log.map(str::to_string),
            });
        }
    };

    let mut body = initial_body.clone();
    let mut model_for_log = initial_model_for_log.map(str::to_string);
    for plugin in plugins {
        match execute_lua_plugin(&plugin, context) {
            Ok(result) => {
                let action = normalize_action(result.action.as_deref()).unwrap_or("continue");
                crate::gateway::log_plugin_hook(
                    trace_id,
                    plugin.id.as_str(),
                    hook_point.as_str(),
                    action,
                    result.message.as_deref(),
                );
                if let Some(annotations) = result.annotations.as_ref() {
                    log_plugin_annotations(
                        trace_id,
                        plugin.id.as_str(),
                        plugin.name.as_str(),
                        hook_point.as_str(),
                        annotations,
                    );
                }
                if action == "reject" {
                    let reject = build_plugin_reject_response(&plugin, result);
                    return RequestHookOutcome::Reject(reject);
                }

                match apply_request_patch(&body, model_for_log.as_deref(), &result) {
                    Ok(next_patch) => {
                        body = next_patch.body;
                        model_for_log = next_patch.model_for_log;
                    }
                    Err(err) => {
                        crate::gateway::log_plugin_hook(
                            trace_id,
                            plugin.id.as_str(),
                            hook_point.as_str(),
                            "patch_error",
                            Some(err.as_str()),
                        );
                        log::warn!(
                            "event=plugin_patch_error trace_id={} plugin_id={} hook_point={} error={}",
                            trace_id,
                            plugin.id,
                            hook_point.as_str(),
                            err
                        );
                    }
                }
            }
            Err(err) => {
                crate::gateway::log_plugin_hook(
                    trace_id,
                    plugin.id.as_str(),
                    hook_point.as_str(),
                    "runtime_error",
                    Some(err.as_str()),
                );
                log::warn!(
                    "event=plugin_runtime_error trace_id={} plugin_id={} hook_point={} error={}",
                    trace_id,
                    plugin.id,
                    hook_point.as_str(),
                    err
                );
            }
        }
    }

    RequestHookOutcome::Continue(PluginRequestPatch {
        body,
        model_for_log,
    })
}

fn has_enabled_lua_plugins(storage: &Storage) -> bool {
    match ENABLED_PLUGIN_CACHE.load(Ordering::Relaxed) {
        PLUGIN_CACHE_DISABLED => false,
        PLUGIN_CACHE_ENABLED => true,
        _ => {
            refresh_plugin_cache(storage);
            ENABLED_PLUGIN_CACHE.load(Ordering::Relaxed) == PLUGIN_CACHE_ENABLED
        }
    }
}

fn load_enabled_plugins(
    storage: &Storage,
    hook_point: PluginHookPoint,
) -> Result<Vec<LoadedPlugin>, String> {
    let hook_name = hook_point.as_str();
    let mut plugins = Vec::new();
    let mut has_any_enabled_lua_plugin = false;
    for record in storage
        .list_plugins()
        .map_err(|err| format!("list plugins failed: {err}"))?
    {
        if !record.runtime.eq_ignore_ascii_case(RUNTIME_LUA) {
            continue;
        }
        if record.enabled {
            has_any_enabled_lua_plugin = true;
        }
        if !record.enabled {
            continue;
        }
        let hook_points = parse_hook_points(&record)?;
        if hook_points.iter().any(|item| item == hook_name) {
            plugins.push(LoadedPlugin {
                id: record.id,
                name: record.name,
                script_content: record.script_content,
                timeout_ms: record.timeout_ms,
            });
        }
    }
    ENABLED_PLUGIN_CACHE.store(
        if has_any_enabled_lua_plugin {
            PLUGIN_CACHE_ENABLED
        } else {
            PLUGIN_CACHE_DISABLED
        },
        Ordering::Relaxed,
    );
    Ok(plugins)
}

fn parse_hook_points(record: &PluginRecord) -> Result<Vec<String>, String> {
    serde_json::from_str::<Vec<String>>(&record.hook_points_json)
        .map_err(|err| format!("parse plugin hook points failed for {}: {err}", record.id))
}

fn build_request_context(
    path: &str,
    method: &str,
    body: &Bytes,
    model_for_log: Option<&str>,
    is_stream: bool,
) -> LuaRequestContext {
    let body_json = serde_json::from_slice::<Value>(body.as_ref()).unwrap_or(Value::Null);
    let model = crate::gateway::parse_request_metadata(body.as_ref())
        .model
        .or_else(|| model_for_log.map(str::to_string));
    LuaRequestContext {
        path: path.to_string(),
        method: method.to_string(),
        model,
        stream: is_stream,
        body: body_json,
    }
}

fn build_response_headers_map(headers: &reqwest::header::HeaderMap) -> Map<String, Value> {
    let mut out = Map::new();
    for (name, value) in headers {
        if let Ok(value) = value.to_str() {
            out.insert(name.as_str().to_string(), Value::String(value.to_string()));
        }
    }
    out
}

fn execute_lua_plugin<T: Serialize>(
    plugin: &LoadedPlugin,
    context: &T,
) -> Result<LuaHandleResult, String> {
    let lua = build_lua_runtime(plugin.timeout_ms)?;
    let env = create_plugin_environment(&lua)?;
    lua.load(plugin.script_content.as_str())
        .set_name(plugin.id.as_str())
        .set_environment(env.clone())
        .exec()
        .map_err(|err| format!("load plugin script failed: {err}"))?;
    let handle: Function = env
        .get("handle")
        .map_err(|_| "plugin handle(ctx) function required".to_string())?;
    let context_value = lua
        .to_value(context)
        .map_err(|err| format!("serialize plugin context failed: {err}"))?;
    let result: LuaValue = handle
        .call(context_value)
        .map_err(|err| format!("run plugin failed: {err}"))?;
    if matches!(result, LuaValue::Nil) {
        return Ok(LuaHandleResult::default());
    }
    lua.from_value(result)
        .map_err(|err| format!("deserialize plugin result failed: {err}"))
}

fn build_lua_runtime(timeout_ms: i64) -> Result<Lua, String> {
    let lua = Lua::new();
    restrict_lua_globals(&lua)?;
    install_lua_timeout_hook(&lua, timeout_ms)?;
    Ok(lua)
}

fn create_plugin_environment(lua: &Lua) -> Result<mlua::Table, String> {
    let env = lua
        .create_table()
        .map_err(|err| format!("create plugin environment failed: {err}"))?;
    let meta = lua
        .create_table()
        .map_err(|err| format!("create plugin environment meta failed: {err}"))?;
    meta.set("__index", lua.globals())
        .map_err(|err| format!("set plugin environment meta failed: {err}"))?;
    env.set_metatable(Some(meta))
        .map_err(|err| format!("set plugin environment metatable failed: {err}"))?;
    Ok(env)
}

fn restrict_lua_globals(lua: &Lua) -> Result<(), String> {
    let globals = lua.globals();
    for key in ["dofile", "loadfile", "load", "require", "collectgarbage"] {
        globals
            .set(key, LuaValue::Nil)
            .map_err(|err| format!("disable Lua global {key} failed: {err}"))?;
    }
    for key in ["io", "os", "package", "debug"] {
        globals
            .set(key, LuaValue::Nil)
            .map_err(|err| format!("disable Lua library {key} failed: {err}"))?;
    }
    Ok(())
}

fn install_lua_timeout_hook(lua: &Lua, timeout_ms: i64) -> Result<(), String> {
    let started_at = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.max(1) as u64);
    lua.set_hook(
        HookTriggers {
            every_nth_instruction: Some(LUA_HOOK_INSTRUCTION_INTERVAL),
            ..HookTriggers::default()
        },
        move |_lua, _debug| {
            if started_at.elapsed() >= timeout {
                Err(mlua::Error::RuntimeError(
                    "plugin execution timed out".to_string(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    )
    .map_err(|err| format!("install plugin timeout hook failed: {err}"))
}

fn normalize_action(action: Option<&str>) -> Option<&str> {
    let normalized = action?.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => None,
        "continue" => Some("continue"),
        "reject" => Some("reject"),
        _ => None,
    }
}

fn apply_request_patch(
    current_body: &Bytes,
    current_model_for_log: Option<&str>,
    result: &LuaHandleResult,
) -> Result<PluginRequestPatch, String> {
    let patch = result.request.as_ref().cloned().unwrap_or_default();
    let patch_model = patch
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            result
                .model
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });

    let mut body = if let Some(value) = patch.body {
        Bytes::from(
            serde_json::to_vec(&value)
                .map_err(|err| format!("serialize plugin request body failed: {err}"))?,
        )
    } else {
        current_body.clone()
    };
    if let Some(model) = patch_model.as_deref() {
        body = rewrite_model_in_json_body(&body, model)?;
    }
    let model_for_log = crate::gateway::parse_request_metadata(body.as_ref())
        .model
        .or(patch_model)
        .or_else(|| current_model_for_log.map(str::to_string));
    Ok(PluginRequestPatch {
        body,
        model_for_log,
    })
}

fn rewrite_model_in_json_body(body: &Bytes, model: &str) -> Result<Bytes, String> {
    let mut value = serde_json::from_slice::<Value>(body.as_ref())
        .map_err(|err| format!("plugin request body must be valid json: {err}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "plugin request body must be a JSON object".to_string())?;
    object.insert("model".to_string(), Value::String(model.to_string()));
    serde_json::to_vec(&value)
        .map(Bytes::from)
        .map_err(|err| format!("serialize plugin request body failed: {err}"))
}

fn build_plugin_reject_response(
    plugin: &LoadedPlugin,
    result: LuaHandleResult,
) -> PluginRejectResponse {
    let message = result
        .message
        .unwrap_or_else(|| format!("request rejected by plugin {}", plugin.name));
    let status_code = result
        .status
        .filter(|status| (400..=599).contains(status))
        .unwrap_or(DEFAULT_PLUGIN_REJECT_STATUS);
    let body = result.body.unwrap_or_else(|| {
        json!({
            "error": {
                "message": message,
                "type": "plugin_reject",
                "code": "plugin_reject",
            }
        })
    });
    let message = extract_plugin_reject_message(&body)
        .unwrap_or_else(|| format!("request rejected by plugin {}", plugin.name));
    PluginRejectResponse {
        plugin_id: plugin.id.clone(),
        plugin_name: plugin.name.clone(),
        status_code,
        body,
        message,
    }
}

fn extract_plugin_reject_message(body: &Value) -> Option<String> {
    body.get("error")
        .and_then(Value::as_object)
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| body.get("message").and_then(Value::as_str))
        .or_else(|| body.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn log_plugin_annotations(
    trace_id: &str,
    plugin_id: &str,
    plugin_name: &str,
    hook_point: &str,
    annotations: &Map<String, Value>,
) {
    let serialized = serde_json::to_string(annotations).unwrap_or_else(|_| "{}".to_string());
    crate::gateway::log_plugin_hook(
        trace_id,
        plugin_id,
        hook_point,
        "annotations",
        Some(serialized.as_str()),
    );
    log::info!(
        "event=plugin_annotations trace_id={} plugin_id={} plugin_name={} hook_point={} annotations={}",
        trace_id,
        plugin_id,
        plugin_name,
        hook_point,
        serialized
    );
}

#[cfg(test)]
mod tests {
    use super::{
        build_plugin_reject_response, execute_lua_plugin, extract_plugin_reject_message,
        rewrite_model_in_json_body, LoadedPlugin, LuaPreRouteContext, LuaRequestContext,
        PluginHookPoint,
    };
    use bytes::Bytes;
    use serde_json::{json, Value};

    fn sample_plugin(script_content: &str) -> LoadedPlugin {
        LoadedPlugin {
            id: "plugin_test".to_string(),
            name: "测试插件".to_string(),
            script_content: script_content.to_string(),
            timeout_ms: 100,
        }
    }

    fn sample_pre_route_context() -> LuaPreRouteContext {
        LuaPreRouteContext {
            request: LuaRequestContext {
                path: "/v1/responses".to_string(),
                method: "POST".to_string(),
                model: Some("o3".to_string()),
                stream: false,
                body: json!({ "model": "o3", "input": "hello" }),
            },
            api_key: super::LuaApiKeyContext {
                id: "gk-test".to_string(),
                name: Some("默认 Key".to_string()),
            },
        }
    }

    #[test]
    fn validate_plugin_runtime_requires_handle_function() {
        let plugin = sample_plugin("local x = 1");
        let err = execute_lua_plugin(&plugin, &sample_pre_route_context())
            .expect_err("missing handle should fail");
        assert!(err.contains("handle"));
    }

    #[test]
    fn lua_plugin_can_rewrite_model_and_attach_annotations() {
        let plugin = sample_plugin(
            r#"
            function handle(ctx)
              return {
                action = "continue",
                request = { model = "gpt-4o" },
                annotations = { tag = "rewritten" }
              }
            end
            "#,
        );
        let result = execute_lua_plugin(&plugin, &sample_pre_route_context()).expect("run plugin");
        assert_eq!(
            result.request.and_then(|patch| patch.model),
            Some("gpt-4o".to_string())
        );
        assert_eq!(
            result
                .annotations
                .and_then(|annotations| annotations.get("tag").cloned()),
            Some(Value::String("rewritten".to_string()))
        );
    }

    #[test]
    fn lua_plugin_timeout_is_enforced() {
        let plugin = LoadedPlugin {
            id: "plugin_timeout".to_string(),
            name: "超时插件".to_string(),
            script_content: r#"
            function handle(ctx)
              while true do
              end
            end
            "#
            .to_string(),
            timeout_ms: 5,
        };
        let err = execute_lua_plugin(&plugin, &sample_pre_route_context())
            .expect_err("timeout should fail");
        assert!(err.contains("timed out"));
    }

    #[test]
    fn lua_sandbox_disables_os_and_io() {
        let plugin = sample_plugin(
            r#"
            function handle(ctx)
              if os ~= nil or io ~= nil or package ~= nil then
                return { action = "reject", status = 500, message = "unsafe" }
              end
              return { action = "continue" }
            end
            "#,
        );
        let result = execute_lua_plugin(&plugin, &sample_pre_route_context()).expect("run plugin");
        assert!(result.status.is_none());
    }

    #[test]
    fn rewrite_model_in_json_body_updates_model_field() {
        let body = Bytes::from_static(br#"{"model":"o3","input":"hello"}"#);
        let rewritten = rewrite_model_in_json_body(&body, "gpt-4o").expect("rewrite model");
        let value: serde_json::Value =
            serde_json::from_slice(rewritten.as_ref()).expect("parse rewritten body");
        assert_eq!(value.get("model").and_then(Value::as_str), Some("gpt-4o"));
    }

    #[test]
    fn build_plugin_reject_response_uses_custom_body_message() {
        let reject = build_plugin_reject_response(
            &sample_plugin("function handle(ctx) return { action = 'reject' } end"),
            super::LuaHandleResult {
                action: Some("reject".to_string()),
                status: Some(429),
                message: Some("quota low".to_string()),
                body: Some(json!({
                    "error": {
                        "message": "plugin says no"
                    }
                })),
                model: None,
                request: None,
                annotations: None,
            },
        );
        assert_eq!(reject.status_code, 429);
        assert_eq!(
            extract_plugin_reject_message(&reject.body).as_deref(),
            Some("plugin says no")
        );
    }

    #[test]
    fn hook_point_labels_match_todo_contract() {
        assert_eq!(PluginHookPoint::PreRoute.as_str(), "pre_route");
        assert_eq!(PluginHookPoint::PostRoute.as_str(), "post_route");
        assert_eq!(PluginHookPoint::PostResponse.as_str(), "post_response");
    }
}
