use crate::apply::{self, ApplyReport};
use crate::config::Config;
use crate::paths::AppPaths;
use crate::splash::SplashRuntime;
use crate::themes::{self, ThemeInfo};
use crate::update::{self, UpdateInfo};
use crate::util;
use crate::win32;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tauri::State;

pub struct AppState {
    pub paths: AppPaths,
    pub config: Arc<RwLock<Config>>,
    pub port: u16,
    pub splash: Arc<SplashRuntime>,
    pub update: Mutex<Option<UpdateInfo>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub app_version: String,
    pub seewo_path: String,
    pub seewo_version: String,
    pub is_admin: bool,
    pub server_port: u16,
    pub active_theme: String,
    pub shortcuts_ok: bool,
    pub launcher_shim: bool,
    pub autostart_ok: bool,
    pub last_apply: String,
    pub webview2_version: String,
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.read().unwrap().clone()
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    if !state.paths.theme_dir(&config.active_theme).join("manifest.json").exists() {
        return Err(format!("主题不存在：{}", config.active_theme));
    }
    config.save(&state.paths).map_err(|e| format!("{e:#}"))?;
    *state.config.write().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub fn list_themes(state: State<'_, AppState>) -> Vec<ThemeInfo> {
    let active = state.config.read().unwrap().active_theme.clone();
    themes::list_themes(&state.paths, &active, state.port)
}

#[tauri::command]
pub fn set_active_theme(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut config = state.config.write().unwrap();
    themes::set_active_theme(&state.paths, &mut *config, &id).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn import_theme(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("主题包 (ZIP)", &["zip"])
        .add_filter("单个 HTML", &["html", "htm"])
        .pick_file()
    else {
        return Ok(None);
    };
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let result = if ext == "html" || ext == "htm" {
        themes::import_theme_html(&state.paths, &path)
    } else {
        themes::import_theme_zip(&state.paths, &path)
    };
    result.map(Some).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn delete_theme(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let active = state.config.read().unwrap().active_theme.clone();
    themes::delete_theme(&state.paths, &active, &id).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Status {
    let install = util::seewo_install();
    let report = apply::read_report(&state.paths);
    let config = state.config.read().unwrap();
    Status {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        seewo_path: install.exe_path,
        seewo_version: install.version,
        is_admin: util::is_admin(),
        server_port: state.port,
        active_theme: config.active_theme.clone(),
        shortcuts_ok: report.as_ref().map(|r| r.shortcuts > 0).unwrap_or(false),
        launcher_shim: report.as_ref().map(|r| r.launcher).unwrap_or(false),
        autostart_ok: report.as_ref().map(|r| r.autostart).unwrap_or(false),
        last_apply: report.map(|r| r.applied_at).unwrap_or_default(),
        webview2_version: util::webview2_version(),
    }
}

#[tauri::command]
pub fn run_apply(state: State<'_, AppState>, restore: bool) -> Result<ApplyReport, String> {
    if util::is_admin() {
        return Ok(apply::run_apply(&state.paths, restore));
    }
    apply::run_elevated_helper(&state.paths, restore).map_err(|e| format!("{e:#}"))?;
    Ok(ApplyReport {
        message: "已请求管理员权限，完成后请刷新状态".to_string(),
        ..Default::default()
    })
}

#[tauri::command]
pub fn check_updates(state: State<'_, AppState>) -> Result<Option<UpdateInfo>, String> {
    let config = state.config.read().unwrap().clone();
    let info = update::check(&state.paths, &config).map_err(|e| format!("{e:#}"))?;
    *state.update.lock().unwrap() = info.clone();
    Ok(info)
}

#[tauri::command]
pub fn install_update(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.read().unwrap().clone();
    let info = {
        let stored = state.update.lock().unwrap().clone();
        match stored {
            Some(info) => info,
            None => update::check(&state.paths, &config)
                .map_err(|e| format!("{e:#}"))?
                .ok_or_else(|| "当前已是最新版本".to_string())?,
        }
    };
    let staging = update::download_and_stage(&state.paths, &config, &info)
        .map_err(|e| format!("{e:#}"))?;
    update::install(&state.paths, &staging, "").map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn install_from_zip(state: State<'_, AppState>) -> Result<bool, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("更新包", &["zip"])
        .pick_file()
    else {
        return Ok(false);
    };
    update::install_from_zip(&state.paths, &path, "").map_err(|e| format!("{e:#}"))?;
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(800));
        std::process::exit(0);
    });
    Ok(true)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open::that(path).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn open_data_dir(state: State<'_, AppState>) -> Result<(), String> {
    open::that(&state.paths.data_dir).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn open_logs_dir(state: State<'_, AppState>) -> Result<(), String> {
    open::that(&state.paths.logs_dir).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn read_log(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let allowed = [
        "app.log",
        "apply.log",
        "splash.log",
        "updater.log",
        "diagnose.log",
    ];
    if !allowed.contains(&name.as_str()) {
        return Err("不允许读取该日志".to_string());
    }
    let path: PathBuf = state.paths.log_path(&name);
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(path).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn test_splash(state: State<'_, AppState>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("{e:#}"))?;
    std::process::Command::new(exe)
        .arg("--splash")
        .arg("--test")
        .spawn()
        .map_err(|e| format!("{e:#}"))?;
    let _ = win32::is_seewo_running();
    let _ = state.port;
    Ok(())
}
