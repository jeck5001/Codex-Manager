import { invoke, invokeFirst } from "./transport";
import { AppSettings } from "../../types";

export const appClient = {
  getSettings: () => invoke<AppSettings>("app_settings_get"),
  setSettings: (patch: Partial<AppSettings>) => invoke("app_settings_set", { patch }),
  
  // Tray
  getCloseToTray: () => invoke<boolean>("app_close_to_tray_on_close_get"),
  setCloseToTray: (enabled: boolean) => invoke("app_close_to_tray_on_close_set", { enabled }),
  
  // Utils
  openInBrowser: (url: string) => invoke("open_in_browser", { url }),
  
  // Update (Using invokeFirst)
  checkUpdate: () => invokeFirst<any>(["app_update_check", "update_check", "check_update"], {}),
  prepareUpdate: (payload = {}) => invokeFirst<any>(["app_update_prepare", "update_download", "download_update"], payload),
  launchInstaller: (payload = {}) => invokeFirst<any>(["app_update_launch_installer", "update_install", "install_update"], payload),
  applyUpdatePortable: (payload = {}) => invokeFirst<any>(["app_update_apply_portable", "update_restart", "restart_update"], payload),
  getStatus: () => invokeFirst<any>(["app_update_status", "update_status"], {}),
};
