#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();

    let sidecar = dir.join("seewo-shim.json");
    let mut main_exe: Option<PathBuf> = None;
    let mut original: Option<PathBuf> = None;
    if let Ok(text) = std::fs::read_to_string(&sidecar) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            main_exe = value
                .get("mainExe")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            original = value
                .get("original")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
        }
    }

    if let Some(main_exe) = main_exe {
        if main_exe.exists() && main_exe != exe {
            let mut command = Command::new(main_exe);
            command.arg("--splash").args(&args);
            let _ = command.spawn();
            return;
        }
    }

    if let Some(original) = original {
        if original.exists() {
            let _ = Command::new(original).args(&args).spawn();
            return;
        }
    }

    let fallback = dir.join("swenlauncher_orig.exe");
    if fallback.exists() {
        let _ = Command::new(fallback).args(&args).spawn();
    }
}
