#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use seewo_splash_switcher_lib::{apply, paths::AppPaths, util};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut root: Option<String> = None;
    let mut restore = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                i += 1;
                root = args.get(i).cloned();
            }
            "--restore" => restore = true,
            _ => {}
        }
        i += 1;
    }

    let paths = if let Some(root) = root {
        AppPaths::from_root(root.into())
    } else {
        AppPaths::discover().unwrap_or_else(|_| {
            AppPaths::from_root(std::env::current_dir().unwrap_or_default())
        })
    };
    let _ = paths.ensure();
    let _ = util::init_logging(&paths);
    let report = apply::run_apply(&paths, restore);
    let _ = apply::write_report(&paths, &report);
    std::process::exit(if report.errors.is_empty() { 0 } else { 1 });
}
