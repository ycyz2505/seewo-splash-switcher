#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use seewo_splash_switcher_lib::{paths::AppPaths, util};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut root: Option<PathBuf> = None;
    let mut staging: Option<PathBuf> = None;
    let mut pid: u32 = 0;
    let mut restart: Option<PathBuf> = None;
    let mut restart_args = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                i += 1;
                root = args.get(i).map(PathBuf::from);
            }
            "--staging" => {
                i += 1;
                staging = args.get(i).map(PathBuf::from);
            }
            "--pid" => {
                i += 1;
                pid = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            "--restart" => {
                i += 1;
                restart = args.get(i).map(PathBuf::from);
            }
            "--restart-args" => {
                i += 1;
                restart_args = args.get(i).cloned().unwrap_or_default();
            }
            _ => {}
        }
        i += 1;
    }

    let Some(root) = root else {
        eprintln!("missing --root");
        std::process::exit(2);
    };
    let Some(staging) = staging else {
        eprintln!("missing --staging");
        std::process::exit(2);
    };
    let paths = AppPaths::from_root(root.clone());
    let _ = paths.ensure();
    let _ = util::init_logging(&paths);
    util::log_line(
        &paths,
        "updater.log",
        &format!("updater start staging={}", staging.display()),
    );

    if pid != 0 {
        wait_for_pid(pid, 60_000);
    }

    let app_dir = root.join("app");
    let new_app = staging.join("app");
    if !new_app.join("seewo-splash-switcher.exe").exists() {
        util::log_line(&paths, "updater.log", "staging app missing");
        std::process::exit(3);
    }

    let old_dir = root.join(format!(
        "app.old.{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    if app_dir.exists() {
        if let Err(err) = std::fs::rename(&app_dir, &old_dir) {
            util::log_line(&paths, "updater.log", &format!("rename old app failed: {err}"));
            std::process::exit(4);
        }
    }
    if let Err(err) = std::fs::rename(&new_app, &app_dir) {
        util::log_line(&paths, "updater.log", &format!("move new app failed: {err}"));
        let _ = std::fs::rename(&old_dir, &app_dir);
        std::process::exit(5);
    }

    let app_exe = app_dir.join("seewo-splash-switcher.exe");
    if health_check(&app_exe) {
        util::log_line(&paths, "updater.log", "health check ok; restarting");
        start_app(&app_exe, &restart_args);
        std::process::exit(0);
    }

    util::log_line(&paths, "updater.log", "health check failed; rolling back");
    let _ = std::fs::remove_dir_all(&app_dir);
    let _ = std::fs::rename(&old_dir, &app_dir);
    if let Some(restart) = restart {
        start_app(&restart, &restart_args);
    }
    std::process::exit(6);
}

fn health_check(exe: &Path) -> bool {
    let mut child = match Command::new(exe).arg("--health-check").spawn() {
        Ok(child) => child,
        Err(_) => return false,
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                return false;
            }
        }
    }
}

fn start_app(exe: &Path, args: &str) {
    let mut command = Command::new(exe);
    for arg in args.split_whitespace() {
        command.arg(arg);
    }
    let _ = command.spawn();
}

#[cfg(windows)]
fn wait_for_pid(pid: u32, timeout_ms: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !handle.is_null() {
            WaitForSingleObject(handle, timeout_ms);
            CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
fn wait_for_pid(_pid: u32, _timeout_ms: u32) {}
