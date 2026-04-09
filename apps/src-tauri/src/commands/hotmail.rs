use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::Manager;

const HOTMAIL_LOCAL_HANDOFF_SCRIPT_ENV: &str = "CODEXMANAGER_HOTMAIL_LOCAL_HANDOFF_SCRIPT";
const HOTMAIL_LOCAL_HANDOFF_PYTHON_ENV: &str = "CODEXMANAGER_HOTMAIL_LOCAL_PYTHON";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotmailLocalHandoffPayload {
    pub handoff_id: String,
    pub url: String,
    pub title: String,
    pub user_agent: String,
    pub proxy_url: String,
    pub state: String,
    pub cookies: Vec<serde_json::Value>,
    pub origins: Vec<serde_json::Value>,
}

fn validate_local_handoff_payload(payload: &HotmailLocalHandoffPayload) -> Result<(), String> {
    if payload.handoff_id.trim().is_empty() {
        return Err("handoffId is required".to_string());
    }
    if payload.url.trim().is_empty() {
        return Err("url is required".to_string());
    }
    Ok(())
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn local_handoff_root_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut root = app
        .path()
        .app_data_dir()
        .map_err(|_| "未找到应用数据目录".to_string())?;
    root.push("hotmail-local-handoff");
    fs::create_dir_all(&root).map_err(|err| format!("创建本地接管目录失败: {err}"))?;
    Ok(root)
}

fn write_payload_file(
    app: &tauri::AppHandle,
    payload: &HotmailLocalHandoffPayload,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let mut root = local_handoff_root_dir(app)?;
    let suffix = unique_suffix();
    root.push(format!("{}-{suffix}", payload.handoff_id.trim()));
    fs::create_dir_all(&root).map_err(|err| format!("创建本地接管工作目录失败: {err}"))?;

    let payload_path = root.join("payload.json");
    let profile_dir = root.join("profile");
    fs::create_dir_all(&profile_dir).map_err(|err| format!("创建浏览器 profile 目录失败: {err}"))?;
    let log_path = root.join("launcher.log");

    let bytes = serde_json::to_vec_pretty(payload)
        .map_err(|err| format!("序列化本地接管 payload 失败: {err}"))?;
    fs::write(&payload_path, bytes).map_err(|err| format!("写入本地接管 payload 失败: {err}"))?;

    Ok((payload_path, profile_dir, log_path))
}

fn resolve_python_bin() -> Result<String, String> {
    let override_bin = std::env::var(HOTMAIL_LOCAL_HANDOFF_PYTHON_ENV).unwrap_or_default();
    if !override_bin.trim().is_empty() {
        return Ok(override_bin);
    }

    #[cfg(target_os = "windows")]
    let candidates = ["python", "python3"];
    #[cfg(not(target_os = "windows"))]
    let candidates = ["python3", "python"];

    for candidate in candidates {
        if Command::new(candidate)
            .args(["-c", "import sys; print(sys.executable)"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return Ok(candidate.to_string());
        }
    }

    Err("未找到可用的 Python 解释器，请安装 python3 或设置 CODEXMANAGER_HOTMAIL_LOCAL_PYTHON".to_string())
}

fn resolve_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let override_path = std::env::var(HOTMAIL_LOCAL_HANDOFF_SCRIPT_ENV).unwrap_or_default();
    if !override_path.trim().is_empty() {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Ok(path);
        }
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/codex-register/src/services/hotmail/local_handoff_cli.py");
    if dev_path.is_file() {
        return Ok(dev_path);
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|_| "未找到桌面应用资源目录".to_string())?;
    let candidates = [
        resource_dir.join("local_handoff_cli.py"),
        resource_dir
            .join("vendor")
            .join("codex-register")
            .join("src")
            .join("services")
            .join("hotmail")
            .join("local_handoff_cli.py"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err("未找到 Hotmail 本地接管脚本".to_string())
}

fn ensure_playwright_ready(python_bin: &str) -> Result<(), String> {
    let status = Command::new(python_bin)
        .args(["-c", "from playwright.sync_api import sync_playwright"])
        .status()
        .map_err(|err| format!("检查 Playwright 依赖失败: {err}"))?;
    if status.success() {
        return Ok(());
    }
    Err(
        "当前本机 Python 缺少 Playwright，请先安装 `pip install playwright && playwright install chromium`"
            .to_string(),
    )
}

fn open_hotmail_local_handoff_blocking(
    app: tauri::AppHandle,
    payload: HotmailLocalHandoffPayload,
) -> Result<String, String> {
    validate_local_handoff_payload(&payload)?;

    let python_bin = resolve_python_bin()?;
    ensure_playwright_ready(&python_bin)?;
    let script_path = resolve_script_path(&app)?;
    let (payload_path, profile_dir, log_path) = write_payload_file(&app, &payload)?;

    let stdout = fs::File::create(&log_path).map_err(|err| format!("创建本地接管日志失败: {err}"))?;
    let stderr = stdout
        .try_clone()
        .map_err(|err| format!("复制本地接管日志句柄失败: {err}"))?;

    Command::new(&python_bin)
        .arg(script_path.as_os_str())
        .args(["--payload", payload_path.to_string_lossy().as_ref()])
        .args(["--profile-dir", profile_dir.to_string_lossy().as_ref()])
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|err| format!("启动 Hotmail 本地接管浏览器失败: {err}"))?;

    Ok(log_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_hotmail_local_handoff(
    app: tauri::AppHandle,
    payload: HotmailLocalHandoffPayload,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || open_hotmail_local_handoff_blocking(app, payload))
        .await
        .map_err(|err| format!("open_hotmail_local_handoff task failed: {err}"))?
}

#[cfg(test)]
mod tests {
    use super::{validate_local_handoff_payload, HotmailLocalHandoffPayload};

    #[test]
    fn local_handoff_payload_rejects_blank_url() {
        let payload = HotmailLocalHandoffPayload {
            handoff_id: "handoff-1".into(),
            url: "   ".into(),
            title: String::new(),
            user_agent: String::new(),
            proxy_url: String::new(),
            state: "unsupported_challenge".into(),
            cookies: vec![],
            origins: vec![],
        };

        let err = validate_local_handoff_payload(&payload).expect_err("blank url should fail");
        assert!(err.contains("url"));
    }

    #[test]
    fn local_handoff_payload_accepts_minimal_valid_input() {
        let payload = HotmailLocalHandoffPayload {
            handoff_id: "handoff-1".into(),
            url: "https://signup.live.com/signup".into(),
            title: String::new(),
            user_agent: String::new(),
            proxy_url: String::new(),
            state: "unsupported_challenge".into(),
            cookies: vec![],
            origins: vec![],
        };

        assert!(validate_local_handoff_payload(&payload).is_ok());
    }
}
