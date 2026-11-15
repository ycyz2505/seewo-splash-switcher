use crate::config::Config;
use crate::paths::AppPaths;
use crate::util;
use crate::win32;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct SplashRuntime {
    pub started_at: Instant,
    pub entered: AtomicBool,
    pub seewo_launched: AtomicBool,
    pub main_hwnd: AtomicIsize,
    pub error: Mutex<Option<String>>,
    pub notify: tokio::sync::Notify,
}

impl SplashRuntime {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            entered: AtomicBool::new(false),
            seewo_launched: AtomicBool::new(false),
            main_hwnd: AtomicIsize::new(0),
            error: Mutex::new(None),
            notify: tokio::sync::Notify::new(),
        }
    }

    pub fn enter(&self) {
        self.entered.store(true, Ordering::SeqCst);
        self.notify.notify_one();
    }

    pub fn set_error(&self, message: impl Into<String>) {
        if let Ok(mut slot) = self.error.lock() {
            *slot = Some(message.into());
        }
    }

    pub fn status(&self) -> SplashStatus {
        let entered = self.entered.load(Ordering::SeqCst);
        let main_hwnd = self.main_hwnd.load(Ordering::SeqCst);
        let error = self.error.lock().ok().and_then(|e| e.clone());
        let state = if error.is_some() {
            "error"
        } else if entered {
            "entered"
        } else {
            "waiting"
        };
        SplashStatus {
            state: state.to_string(),
            seewo_running: win32::is_seewo_running(),
            main_ready: main_hwnd != 0,
            entered,
            elapsed_ms: self.started_at.elapsed().as_millis(),
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplashStatus {
    pub state: String,
    pub seewo_running: bool,
    pub main_ready: bool,
    pub entered: bool,
    pub elapsed_ms: u128,
    pub error: Option<String>,
}

pub fn launch_seewo(paths: &AppPaths, config: &Config, extra_args: &[String]) -> Result<()> {
    let install = util::seewo_install();
    let launcher = Path::new(&install.launcher_path);
    let launcher_orig = launcher.with_file_name("swenlauncher_orig.exe");
    let exe = Path::new(&install.exe_path);

    let target = if launcher_orig.exists() {
        launcher_orig
    } else if exe.exists() {
        exe.to_path_buf()
    } else if launcher.exists() {
        launcher.to_path_buf()
    } else {
        return Err(anyhow::anyhow!(
            "找不到希沃启动程序：{} / {}",
            install.launcher_path,
            install.exe_path
        ));
    };

    let mut command = Command::new(&target);
    if config.splash.forward_args {
        command.args(extra_args);
    }
    if let Some(parent) = target.parent() {
        command.current_dir(parent);
    }
    let child = command
        .spawn()
        .with_context(|| format!("启动 {}", target.display()))?;
    util::log_line(
        paths,
        "splash.log",
        &format!("launched {} pid={}", target.display(), child.id()),
    );
    Ok(())
}

pub fn run_splash_loop(
    paths: &AppPaths,
    config: &Config,
    runtime: &SplashRuntime,
    test_mode: bool,
    extra_args: &[String],
) {
    if !test_mode && config.splash.launch_seewo {
        if config.splash.skip_if_running && win32::is_seewo_running() {
            util::log_line(paths, "splash.log", "Seewo already running; skip launch");
        } else if let Err(err) = launch_seewo(paths, config, extra_args) {
            runtime.set_error(format!("启动希沃失败：{err:#}"));
            util::log_line(paths, "splash.log", &format!("launch failed: {err:#}"));
        } else {
            runtime.seewo_launched.store(true, Ordering::SeqCst);
        }
    }

    let hide = config.splash.hide_original_splash;
    let timeout = Duration::from_secs(if test_mode {
        10
    } else {
        config.splash.timeout_seconds.max(10)
    });

    let start = Instant::now();
    let poll_until = start + timeout;
    while Instant::now() < poll_until {
        if hide {
            let hidden = win32::hide_splash_windows();
            if hidden > 0 {
                util::log_line(paths, "splash.log", "hid original Splash Image window");
            }
        }
        if let Some(window) = win32::find_main_window() {
            if runtime.main_hwnd.load(Ordering::SeqCst) == 0 {
                util::log_line(
                    paths,
                    "splash.log",
                    &format!(
                        "main window found hwnd={} class={} title={}",
                        window.hwnd, window.class_name, window.title
                    ),
                );
            }
            runtime.main_hwnd.store(window.hwnd, Ordering::SeqCst);
            let entered = runtime.entered.load(Ordering::SeqCst);
            let launched = runtime.seewo_launched.load(Ordering::SeqCst);
            if !test_mode && launched && !entered {
                win32::hide_window(window.hwnd);
            }
        }
        if runtime.entered.load(Ordering::SeqCst) {
            let launched = runtime.seewo_launched.load(Ordering::SeqCst);
            let main_ready = runtime.main_hwnd.load(Ordering::SeqCst) != 0;
            if main_ready || !launched || test_mode {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if !runtime.entered.load(Ordering::SeqCst) {
        let remaining = timeout.saturating_sub(start.elapsed());
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                runtime.set_error(format!("创建等待运行时失败：{err}"));
                return;
            }
        };
        let _ = rt.block_on(async {
            tokio::time::timeout(remaining, runtime.notify.notified()).await
        });
    }

    if !test_mode && runtime.seewo_launched.load(Ordering::SeqCst) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if let Some(window) = win32::find_main_window() {
                runtime.main_hwnd.store(window.hwnd, Ordering::SeqCst);
                break;
            }
            std::thread::sleep(Duration::from_millis(80));
        }
    }

    let hwnd = runtime.main_hwnd.load(Ordering::SeqCst);
    if hwnd != 0 {
        win32::reveal_window(hwnd);
        util::log_line(paths, "splash.log", &format!("revealed main window {hwnd}"));
    } else {
        util::log_line(paths, "splash.log", "no main window to reveal");
    }
}
