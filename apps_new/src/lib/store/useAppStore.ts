import { create } from "zustand";
import { AppSettings, ServiceStatus } from "../../types";

interface AppState {
  serviceStatus: ServiceStatus;
  appSettings: AppSettings;
  isSidebarOpen: boolean;
  
  setServiceStatus: (status: Partial<ServiceStatus>) => void;
  setAppSettings: (settings: Partial<AppSettings>) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  serviceStatus: {
    connected: false,
    version: "",
    uptime: 0,
    addr: "localhost:48760",
  },
  appSettings: {
    updateAutoCheck: true,
    closeToTrayOnClose: false,
    lowTransparency: false,
    lightweightModeOnCloseToTray: false,
    webAccessPasswordConfigured: false,
    serviceAddr: "localhost:48760",
  },
  isSidebarOpen: true,

  setServiceStatus: (status) => 
    set((state) => ({ serviceStatus: { ...state.serviceStatus, ...status } })),
  
  setAppSettings: (settings) =>
    set((state) => ({ appSettings: { ...state.appSettings, ...settings } })),
    
  toggleSidebar: () => set((state) => ({ isSidebarOpen: !state.isSidebarOpen })),
  
  setSidebarOpen: (open) => set({ isSidebarOpen: open }),
}));
