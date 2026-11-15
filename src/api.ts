import { invoke } from "@tauri-apps/api/core";

export interface SplashConfig {
  launchSeewo: boolean;
  hideOriginalSplash: boolean;
  timeoutSeconds: number;
  skipIfRunning: boolean;
  monitor: string;
  audio: boolean;
  forwardArgs: boolean;
}

export interface TakeoverConfig {
  shortcuts: boolean;
  launcherShim: boolean;
  autostart: boolean;
}

export interface UpdateConfig {
  enabled: boolean;
  autoInstall: boolean;
  channel: string;
  checkIntervalHours: number;
  customMirrors: string[];
  proxyMode: string;
  proxyUrl: string;
}

export interface Config {
  schemaVersion: number;
  activeTheme: string;
  enabled: boolean;
  splash: SplashConfig;
  takeover: TakeoverConfig;
  update: UpdateConfig;
}

export interface ThemeInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  previewUrl: string;
  active: boolean;
  path: string;
}

export interface Status {
  appVersion: string;
  seewoPath: string;
  seewoVersion: string;
  isAdmin: boolean;
  serverPort: number;
  activeTheme: string;
  shortcutsOk: boolean;
  launcherShim: boolean;
  autostartOk: boolean;
  lastApply: string;
  webview2Version: string;
}

export interface ApplyReport {
  shortcuts: number;
  launcher: boolean;
  autostart: boolean;
  errors: string[];
  message: string;
  appliedAt: string;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string;
  pubDate: string;
  size: number;
  url: string;
}

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  listThemes: () => invoke<ThemeInfo[]>("list_themes"),
  setActiveTheme: (id: string) => invoke<void>("set_active_theme", { id }),
  importTheme: () => invoke<string | null>("import_theme"),
  deleteTheme: (id: string) => invoke<void>("delete_theme", { id }),
  getStatus: () => invoke<Status>("get_status"),
  runApply: (restore = false) => invoke<ApplyReport>("run_apply", { restore }),
  checkUpdates: () => invoke<UpdateInfo | null>("check_updates"),
  installUpdate: () => invoke<void>("install_update"),
  installFromZip: () => invoke<boolean>("install_from_zip"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openDataDir: () => invoke<void>("open_data_dir"),
  openLogsDir: () => invoke<void>("open_logs_dir"),
  readLog: (name: string) => invoke<string>("read_log", { name }),
  testSplash: () => invoke<void>("test_splash"),
};

export function formatBytes(n: number): string {
  if (!n) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
