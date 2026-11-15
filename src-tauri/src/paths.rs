use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub app_dir: PathBuf,
    pub data_dir: PathBuf,
    pub themes_dir: PathBuf,
    pub config_path: PathBuf,
    pub logs_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub updates_dir: PathBuf,
    pub webview_dir: PathBuf,
}

impl AppPaths {
    pub fn from_root(root: PathBuf) -> Self {
        let data_dir = root.join("data");
        Self {
            app_dir: root.join("app"),
            themes_dir: data_dir.join("themes"),
            config_path: data_dir.join("config.json"),
            logs_dir: root.join("logs"),
            tools_dir: root.join("tools"),
            backup_dir: root.join("backup"),
            updates_dir: root.join("updates"),
            webview_dir: data_dir.join("webview2"),
            data_dir,
            root,
        }
    }

    pub fn discover() -> Result<Self> {
        let exe = std::env::current_exe().context("current_exe")?;
        let exe_dir = exe.parent().context("exe parent")?.to_path_buf();

        let root = if let Some(root) = std::env::var_os("SEEWO_SPLASH_ROOT") {
            PathBuf::from(root)
        } else if exe_dir
            .file_name()
            .map(|name| name.to_string_lossy().eq_ignore_ascii_case("app"))
            .unwrap_or(false)
        {
            exe_dir.parent().context("app parent")?.to_path_buf()
        } else {
            exe_dir.clone()
        };

        Ok(Self::from_root(root))
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [
            &self.root,
            &self.app_dir,
            &self.data_dir,
            &self.themes_dir,
            &self.logs_dir,
            &self.tools_dir,
            &self.backup_dir,
            &self.updates_dir,
            &self.webview_dir,
        ] {
            std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        Ok(())
    }

    pub fn app_exe(&self) -> PathBuf {
        self.app_dir.join("seewo-splash-switcher.exe")
    }

    pub fn tool(&self, name: &str) -> PathBuf {
        self.tools_dir.join(name)
    }

    pub fn log_path(&self, name: &str) -> PathBuf {
        self.logs_dir.join(name)
    }

    pub fn theme_dir(&self, id: &str) -> PathBuf {
        self.themes_dir.join(id)
    }

    pub fn default_data_dir(&self) -> PathBuf {
        self.root.join("default-data")
    }

    pub fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
