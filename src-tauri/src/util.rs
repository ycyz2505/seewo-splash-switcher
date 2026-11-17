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

fn reg_string(hive: winreg::HKEY, subkey: &str, value: &str) -> Option<String> {
    let key = winreg::RegKey::predef(hive).open_subkey(subkey).ok()?;
    key.get_value::<String, _>(value).ok()
}

fn find_version_dir(install_root: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(install_root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("Main").join("EasiNote.exe").exists())
        .collect();
    dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    dirs.into_iter().next()
}

impl SeewoInstall {
    pub fn summary(&self) -> String {
        let exists = |p: &str| !p.is_empty() && Path::new(p).exists();
        format!(
            "install_root={} (exists={}) version_path={} version={} exe_path={} (exists={}) launcher_path={} (exists={})",
            self.install_root,
            exists(&self.install_root),
            self.version_path,
            self.version,
            self.exe_path,
            exists(&self.exe_path),
            self.launcher_path,
            exists(&self.launcher_path)
        )
    }
}

pub fn seewo_install() -> SeewoInstall {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let keys: [(&str, winreg::HKEY); 6] = [
        (r"SOFTWARE\WOW6432Node\Seewo\EasiNote5", HKEY_LOCAL_MACHINE),
        (r"SOFTWARE\Seewo\EasiNote5", HKEY_LOCAL_MACHINE),
        (r"SOFTWARE\WOW6432Node\CVTE\EasiNote5", HKEY_LOCAL_MACHINE),
        (r"SOFTWARE\CVTE\EasiNote5", HKEY_LOCAL_MACHINE),
        (r"SOFTWARE\WOW6432Node\Seewo\EasiNote5", HKEY_CURRENT_USER),
        (r"SOFTWARE\Seewo\EasiNote5", HKEY_CURRENT_USER),
    ];
    let mut actual_exe = String::new();
    let mut version_path = String::new();
    let mut version = String::new();
    let mut launcher_path = String::new();
    let mut install_path = String::new();
    for (key, hive) in keys {
        if actual_exe.is_empty() {
            actual_exe = reg_string(hive, key, "ActualExePath").unwrap_or_default();
        }
        if version_path.is_empty() {
            version_path = reg_string(hive, key, "VersionPath").unwrap_or_default();
        }
        if version.is_empty() {
            version = reg_string(hive, key, "version").unwrap_or_default();
        }
        if launcher_path.is_empty() {
            launcher_path = reg_string(hive, key, "ExePath").unwrap_or_default();
        }
        if install_path.is_empty() {
            install_path = reg_string(hive, key, "path").unwrap_or_default();
        }
    }

    let mut install_root = if !install_path.is_empty() {
        install_path
    } else if !version_path.is_empty() {
        Path::new(&version_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else if !actual_exe.is_empty() {
        Path::new(&actual_exe)
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if install_root.is_empty() || !Path::new(&install_root).exists() {
        for candidate in [
            r"C:\Program Files (x86)\Seewo\EasiNote5",
            r"C:\Program Files\Seewo\EasiNote5",
            r"C:\Seewo\EasiNote5",
            r"D:\Program Files (x86)\Seewo\EasiNote5",
            r"D:\Program Files\Seewo\EasiNote5",
            r"D:\Seewo\EasiNote5",
            r"E:\Program Files (x86)\Seewo\EasiNote5",
            r"E:\Program Files\Seewo\EasiNote5",
            r"E:\Seewo\EasiNote5",
        ] {
            if Path::new(candidate).exists() {
                install_root = candidate.to_string();
                break;
            }
        }
    }

    if install_root.is_empty() && !actual_exe.is_empty() {
        if let Some(root) = Path::new(&actual_exe)
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            if root.exists() {
                install_root = root.to_string_lossy().to_string();
            }
        }
    }
    if install_root.is_empty() && !launcher_path.is_empty() {
        if let Some(root) = Path::new(&launcher_path).parent().and_then(|p| p.parent()) {
            if root.exists() {
                install_root = root.to_string_lossy().to_string();
            }
        }
    }

    if (version_path.is_empty() || !Path::new(&version_path).exists()) && !install_root.is_empty() {
        if let Some(dir) = find_version_dir(Path::new(&install_root)) {
            version_path = dir.to_string_lossy().to_string();
        }
    }

    let mut exe_path = if !actual_exe.is_empty() && Path::new(&actual_exe).exists() {
        actual_exe
    } else if !version_path.is_empty()
        && Path::new(&version_path).join("Main").join("EasiNote.exe").exists()
    {
        Path::new(&version_path)
            .join("Main")
            .join("EasiNote.exe")
            .to_string_lossy()
            .to_string()
    } else {
        String::new()
    };
    if exe_path.is_empty() && !install_root.is_empty() {
        if let Some(dir) = find_version_dir(Path::new(&install_root)) {
            let candidate = dir.join("Main").join("EasiNote.exe");
            if candidate.exists() {
                version_path = dir.to_string_lossy().to_string();
                exe_path = candidate.to_string_lossy().to_string();
            }
        }
    }
    if exe_path.is_empty() && !install_root.is_empty() {
        let candidate = Path::new(&install_root).join("Main").join("EasiNote.exe");
        if candidate.exists() {
            exe_path = candidate.to_string_lossy().to_string();
        }
    }

    let mut launcher = if !launcher_path.is_empty() && Path::new(&launcher_path).exists() {
        launcher_path
    } else {
        String::new()
    };
    if launcher.is_empty() && !install_root.is_empty() {
        let candidate = Path::new(&install_root)
            .join("swenlauncher")
            .join("swenlauncher.exe");
        if candidate.exists() {
            launcher = candidate.to_string_lossy().to_string();
        }
    }

    SeewoInstall {
        install_root,
        exe_path,
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

pub fn system_proxy_url() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_READ,
        )
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    let server = server.trim();
    if server.is_empty() {
        return None;
    }
    let candidate = if server.contains('=') {
        server.split(';').find_map(|part| {
            let (scheme, value) = part.split_once('=')?;
            if scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http") {
                Some(value.trim())
            } else {
                None
            }
        })?
    } else {
        server
    };
    if candidate.contains("://") {
        Some(candidate.to_string())
    } else {
        Some(format!("http://{candidate}"))
    }
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)
        .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    Ok(())
}
