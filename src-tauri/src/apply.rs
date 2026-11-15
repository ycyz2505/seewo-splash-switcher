use crate::config::Config;
use crate::paths::AppPaths;
use crate::util;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ApplyReport {
    pub shortcuts: u32,
    pub launcher: bool,
    pub autostart: bool,
    pub errors: Vec<String>,
    pub message: String,
    pub applied_at: String,
}

const APPLY_SHORTCUTS: &str = include_str!("../scripts/apply-shortcuts.ps1");
const RESTORE_SHORTCUTS: &str = include_str!("../scripts/restore-shortcuts.ps1");
const APPLY_AUTOSTART: &str = include_str!("../scripts/apply-autostart.ps1");
const REMOVE_AUTOSTART: &str = include_str!("../scripts/remove-autostart.ps1");

pub fn run_apply(paths: &AppPaths, restore: bool) -> ApplyReport {
    let mut report = ApplyReport {
        applied_at: util::now_iso(),
        ..Default::default()
    };
    let config = match Config::load(paths) {
        Ok(config) => config,
        Err(err) => {
            report.errors.push(format!("读取配置失败：{err:#}"));
            return report;
        }
    };
    if let Err(err) = ensure_scripts(paths) {
        report.errors.push(format!("写入脚本失败：{err:#}"));
        return report;
    }
    let app_exe = paths.app_exe();
    let backup = paths.backup_dir.clone();
    std::fs::create_dir_all(&backup).ok();

    if restore {
        let result = run_script(
            paths,
            "restore-shortcuts.ps1",
            &[
                "-BackupDir".to_string(),
                backup.to_string_lossy().to_string(),
            ],
        );
        match result {
            Ok(value) => {
                report.shortcuts = value
                    .get("restored")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                push_errors(&mut report.errors, &value);
            }
            Err(err) => report.errors.push(format!("恢复快捷方式失败：{err:#}")),
        }

        let result = run_script(
            paths,
            "remove-autostart.ps1",
            &[
                "-TaskName".to_string(),
                "SeewoHtmlSplash\\Boot".to_string(),
                "-BackupDir".to_string(),
                backup.to_string_lossy().to_string(),
            ],
        );
        match result {
            Ok(value) => {
                report.autostart = false;
                push_errors(&mut report.errors, &value);
            }
            Err(err) => report.errors.push(format!("移除计划任务失败：{err:#}")),
        }

        if let Err(err) = restore_launcher(paths, &mut report) {
            report.errors.push(format!("恢复 launcher 失败：{err:#}"));
        }
        report.message = "已恢复原状".to_string();
    } else {
        if config.takeover.shortcuts {
            let result = run_script(
                paths,
                "apply-shortcuts.ps1",
                &[
                    "-Root".to_string(),
                    paths.root.to_string_lossy().to_string(),
                    "-AppExe".to_string(),
                    app_exe.to_string_lossy().to_string(),
                    "-BackupDir".to_string(),
                    backup.to_string_lossy().to_string(),
                ],
            );
            match result {
                Ok(value) => {
                    report.shortcuts = value
                        .get("rewritten")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    push_errors(&mut report.errors, &value);
                }
                Err(err) => report.errors.push(format!("应用快捷方式失败：{err:#}")),
            }
        }

        if config.takeover.launcher_shim {
            match apply_launcher_shim(paths, &app_exe, &mut report) {
                Ok(()) => report.launcher = true,
                Err(err) => report.errors.push(format!("应用 launcher shim 失败：{err:#}")),
            }
        } else if let Err(err) = restore_launcher(paths, &mut report) {
            report.errors.push(format!("恢复 launcher 失败：{err:#}"));
        }

        if config.takeover.autostart {
            let result = run_script(
                paths,
                "apply-autostart.ps1",
                &[
                    "-AppExe".to_string(),
                    app_exe.to_string_lossy().to_string(),
                    "-TaskName".to_string(),
                    "SeewoHtmlSplash\\Boot".to_string(),
                    "-BackupDir".to_string(),
                    backup.to_string_lossy().to_string(),
                ],
            );
            match result {
                Ok(value) => {
                    report.autostart = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    push_errors(&mut report.errors, &value);
                }
                Err(err) => report.errors.push(format!("创建计划任务失败：{err:#}")),
            }
        } else {
            let _ = run_script(
                paths,
                "remove-autostart.ps1",
                &[
                    "-TaskName".to_string(),
                    "SeewoHtmlSplash\\Boot".to_string(),
                    "-BackupDir".to_string(),
                    backup.to_string_lossy().to_string(),
                ],
            );
            report.autostart = false;
        }

        report.message = "已应用当前配置".to_string();
    }

    let _ = write_report(paths, &report);
    report
}

fn ensure_scripts(paths: &AppPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.tools_dir)?;
    for (name, content) in [
        ("apply-shortcuts.ps1", APPLY_SHORTCUTS),
        ("restore-shortcuts.ps1", RESTORE_SHORTCUTS),
        ("apply-autostart.ps1", APPLY_AUTOSTART),
        ("remove-autostart.ps1", REMOVE_AUTOSTART),
    ] {
        std::fs::write(paths.tool(name), content)?;
    }
    Ok(())
}

fn run_script(paths: &AppPaths, name: &str, args: &[String]) -> Result<serde_json::Value> {
    let script = paths.tool(name);
    let output = util::run_powershell(&script, args)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "{} 退出码 {:?}\nstdout: {}\nstderr: {}",
            name,
            output.status.code(),
            stdout.trim(),
            stderr.trim()
        ));
    }
    let result_file = paths
        .backup_dir
        .join(name.replace(".ps1", "-result.json"));
    if result_file.exists() {
        let text = std::fs::read_to_string(&result_file)?;
        let text = text.trim_start_matches('\u{feff}');
        return serde_json::from_str(text).context("解析脚本结果");
    }
    Ok(serde_json::json!({}))
}

fn push_errors(errors: &mut Vec<String>, value: &serde_json::Value) {
    if let Some(items) = value.get("errors").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(text) = item.as_str() {
                errors.push(text.to_string());
            }
        }
    }
}

fn apply_launcher_shim(
    paths: &AppPaths,
    app_exe: &Path,
    report: &mut ApplyReport,
) -> Result<()> {
    let install = util::seewo_install();
    if install.launcher_path.is_empty() {
        return Err(anyhow::anyhow!("找不到希沃 launcher 路径"));
    }
    let launcher = PathBuf::from(&install.launcher_path);
    if !launcher.exists() {
        return Err(anyhow::anyhow!("launcher 不存在：{}", launcher.display()));
    }
    let orig = launcher.with_file_name("swenlauncher_orig.exe");
    if !orig.exists() {
        std::fs::copy(&launcher, &orig)
            .with_context(|| format!("备份 {}", launcher.display()))?;
    }
    let shim = paths.tool("seewo-launcher-shim.exe");
    if !shim.exists() {
        return Err(anyhow::anyhow!("缺少 {}", shim.display()));
    }
    std::fs::copy(&shim, &launcher)
        .with_context(|| format!("替换 {}", launcher.display()))?;
    let sidecar = launcher.with_file_name("seewo-shim.json");
    let json = serde_json::json!({
        "mainExe": app_exe.to_string_lossy(),
        "original": orig.to_string_lossy()
    });
    std::fs::write(sidecar, serde_json::to_vec_pretty(&json)?)?;
    report.launcher = true;
    util::log_line(paths, "apply.log", "launcher shim applied");
    Ok(())
}

fn restore_launcher(paths: &AppPaths, report: &mut ApplyReport) -> Result<()> {
    let install = util::seewo_install();
    if install.launcher_path.is_empty() {
        return Ok(());
    }
    let launcher = PathBuf::from(&install.launcher_path);
    let orig = launcher.with_file_name("swenlauncher_orig.exe");
    if orig.exists() {
        std::fs::copy(&orig, &launcher)
            .with_context(|| format!("恢复 {}", launcher.display()))?;
        util::log_line(paths, "apply.log", "launcher restored from backup");
    }
    let sidecar = launcher.with_file_name("seewo-shim.json");
    let _ = std::fs::remove_file(sidecar);
    report.launcher = false;
    Ok(())
}

pub fn write_report(paths: &AppPaths, report: &ApplyReport) -> Result<()> {
    let path = paths.data_dir.join("apply-report.json");
    std::fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

pub fn read_report(paths: &AppPaths) -> Option<ApplyReport> {
    let path = paths.data_dir.join("apply-report.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

pub fn run_elevated_helper(paths: &AppPaths, restore: bool) -> Result<()> {
    let helper = paths.app_dir.join("seewo-apply.exe");
    if !helper.exists() {
        return Err(anyhow::anyhow!("缺少 {}", helper.display()));
    }
    let mut args = format!("--root \"{}\"", paths.root.to_string_lossy());
    if restore {
        args.push_str(" --restore");
    }
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        fn wide(value: &str) -> Vec<u16> {
            OsStr::new(value)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }
        let verb = wide("runas");
        let file = wide(&helper.to_string_lossy());
        let params = wide(&args);
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                params.as_ptr(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err(anyhow::anyhow!(
                "请求管理员权限失败（ShellExecuteW={}）",
                result as isize
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let status = Command::new(helper).args(["--root", &paths.root.to_string_lossy()]).status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("helper 退出码 {:?}", status.code()));
        }
        Ok(())
    }
}
