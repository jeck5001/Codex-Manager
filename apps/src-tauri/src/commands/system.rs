use crate::{
    app_shell::set_unsaved_settings_draft_sections, commands::shared::open_in_browser_blocking,
};

#[tauri::command]
pub async fn open_in_browser(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_in_browser_blocking(&url))
        .await
        .map_err(|err| format!("open_in_browser task failed: {err}"))?
}

fn open_in_browser_incognito_blocking(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let browsers = [
            ("chrome.exe", "--incognito"),
            ("msedge.exe", "--inprivate"),
            ("chrome", "--incognito"),
            ("msedge", "--inprivate"),
        ];
        for (browser, flag) in browsers {
            if std::process::Command::new(browser)
                .args([flag, url])
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }
        return open_in_browser_blocking(url);
    }

    #[cfg(target_os = "macos")]
    {
        let spawned = std::process::Command::new("open")
            .args(["-a", "Google Chrome", "--args", "--incognito", url])
            .spawn()
            .map_err(|err| err.to_string())?;
        let _ = spawned;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let browsers = [
            ("google-chrome", "--incognito"),
            ("chromium-browser", "--incognito"),
            ("chromium", "--incognito"),
            ("microsoft-edge", "--inprivate"),
        ];
        for (browser, flag) in browsers {
            if std::process::Command::new(browser)
                .args([flag, url])
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }
        open_in_browser_blocking(url)
    }
}

#[tauri::command]
pub async fn open_in_browser_incognito(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_in_browser_incognito_blocking(&url))
        .await
        .map_err(|err| format!("open_in_browser_incognito task failed: {err}"))?
}

#[tauri::command]
pub fn app_window_unsaved_draft_sections_set(sections: Vec<String>) -> Result<(), String> {
    set_unsaved_settings_draft_sections(sections);
    Ok(())
}
