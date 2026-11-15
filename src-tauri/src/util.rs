use crate::paths::AppPaths;
use anyhow::{Context, Result};
use log::{Level, LevelFilter, Log, Metadata, Record};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

struct FileLogger {
    file: Mutex<File>,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(
                file,
                "{} [{}] {}",
                now_iso(),
                record.level(),
                record.args()
            );
            let _ = file.flush();
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

pub fn init_logging(paths: &AppPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.logs_dir)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.log_path("app.log"))?;
    let logger = FileLogger {
        file: Mutex::new(file),
    };
    let _ = log::set_boxed_logger(Box::new(logger));
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

pub fn log_line(paths: &AppPaths, name: &str, message: &str) {
    let path = paths.log_path(name);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} {}", now_iso(), message);
    }
}

pub fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(())
}

pub fn read_registry_string(subkey: &str, value: &str) -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(subkey).ok()?;
    key.get_value::<String, _>(value).ok()
}

pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
        unsafe { IsUserAnAdmin() != 0 }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[derive(Clone, Debug, Default)]
pub struct SeewoInstall {
    pub install_root: String,
    pub exe_path: String,
    pub version_path: String,
    pub version: String,
    pub launcher_path: String,
}

pub fn seewo_install() -> SeewoInstall {
    const KEY: &str = r"SOFTWARE\WOW6432Node\Seewo\EasiNote5";
    let exe_path = read_registry_string(KEY, "ActualExePath").unwrap_or_default();
    let version_path = read_registry_string(KEY, "VersionPath").unwrap_or_default();
    let version = read_registry_string(KEY, "version").unwrap_or_default();
    let launcher_path = read_registry_string(KEY, "ExePath").unwrap_or_default();

    let mut install_root = if !version_path.is_empty() {
        PathBuf::from(&version_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };
    if install_root.is_empty() {
        for candidate in [
            r"C:\Program Files (x86)\Seewo\EasiNote5",
            r"C:\Program Files\Seewo\EasiNote5",
        ] {
            if Path::new(candidate).exists() {
                install_root = candidate.to_string();
                break;
            }
        }
    }

    let launcher = if !launcher_path.is_empty() {
        launcher_path
    } else if !install_root.is_empty() {
        Path::new(&install_root)
            .join(r"swenlauncher\swenlauncher.exe")
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };

    let exe = if !exe_path.is_empty() {
        exe_path
    } else if !version_path.is_empty() {
        Path::new(&version_path)
            .join(r"Main\EasiNote.exe")
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };

    SeewoInstall {
        install_root,
        exe_path: exe,
        version_path,
        version,
        launcher_path: launcher,
    }
}

pub fn run_powershell(script: &Path, args: &[String]) -> Result<Output> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output()
        .with_context(|| format!("run powershell script {}", script.display()))
}

pub fn webview2_version() -> String {
    const KEY: &str =
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    read_registry_string(KEY, "pv").unwrap_or_else(|| "未检测到".to_string())
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)
        .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    Ok(())
}
