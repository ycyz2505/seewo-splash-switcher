use crate::paths::AppPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    pub active_theme: String,
    pub enabled: bool,
    pub splash: SplashConfig,
    pub takeover: TakeoverConfig,
    pub update: UpdateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            active_theme: "default".to_string(),
            enabled: true,
            splash: SplashConfig::default(),
            takeover: TakeoverConfig::default(),
            update: UpdateConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SplashConfig {
    pub launch_seewo: bool,
    pub hide_original_splash: bool,
    pub timeout_seconds: u64,
    pub skip_if_running: bool,
    pub monitor: String,
    pub audio: bool,
    pub forward_args: bool,
}

impl Default for SplashConfig {
    fn default() -> Self {
        Self {
            launch_seewo: true,
            hide_original_splash: true,
            timeout_seconds: 90,
            skip_if_running: true,
            monitor: "primary".to_string(),
            audio: false,
            forward_args: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TakeoverConfig {
    pub shortcuts: bool,
    pub launcher_shim: bool,
    pub autostart: bool,
}

impl Default for TakeoverConfig {
    fn default() -> Self {
        Self {
            shortcuts: true,
            launcher_shim: false,
            autostart: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub auto_install: bool,
    pub channel: String,
    pub check_interval_hours: u64,
    pub custom_mirrors: Vec<String>,
    pub proxy_mode: String,
    pub proxy_url: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_install: true,
            channel: "stable".to_string(),
            check_interval_hours: 24,
            custom_mirrors: Vec::new(),
            proxy_mode: "system".to_string(),
            proxy_url: String::new(),
        }
    }
}

impl Config {
    pub fn load(paths: &AppPaths) -> Result<Self> {
        let path = &paths.config_path;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save(paths)?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let mut cfg: Config = serde_json::from_str(&text).context("parse config.json")?;
        if cfg.schema_version == 0 {
            cfg.schema_version = 1;
        }
        Ok(cfg)
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        atomic_write(&paths.config_path, text.as_bytes())
    }
}

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("json")
    ));
    std::fs::write(&tmp, data)?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}
