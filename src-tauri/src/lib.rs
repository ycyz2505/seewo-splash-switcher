pub mod apply;
pub mod commands;
pub mod config;
pub mod paths;
pub mod server;
pub mod splash;
pub mod themes;
pub mod update;
pub mod util;
pub mod win32;

use crate::commands::AppState;
use crate::config::Config;
use crate::paths::AppPaths;
use crate::splash::SplashRuntime;
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let paths = match AppPaths::discover() {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("无法定位应用目录：{err:#}");
            std::process::exit(1);
        }
    };
    let _ = paths.ensure();
    let _ = util::init_logging(&paths);
    let _ = themes::ensure_default_theme(&paths);

    match mode {
        "--apply" => {
            let restore = args.iter().any(|arg| arg == "--restore");
            let report = apply::run_apply(&paths, restore);
            let _ = apply::write_report(&paths, &report);
            std::process::exit(if report.errors.is_empty() { 0 } else { 1 });
        }
        "--boot" => {
            let config = Config::load(&paths).unwrap_or_default();
            let report = apply::run_apply(&paths, false);
            util::log_line(
                &paths,
                "apply.log",
                &format!(
                    "boot apply: shortcuts={} launcher={} autostart={} errors={:?}",
                    report.shortcuts, report.launcher, report.autostart, report.errors
                ),
            );
            update::run_boot_update(&paths, &config);
            std::process::exit(0);
        }
        "--health-check" => {
            let ok = Config::load(&paths).is_ok() && paths.themes_dir.exists();
            std::process::exit(if ok { 0 } else { 1 });
        }
        "--splash" => {
            run_splash(paths, args[2..].to_vec());
        }
        _ => run_gui(paths),
    }
}

fn configure_webview(paths: &AppPaths) {
    std::env::set_var(
        "WEBVIEW2_USER_DATA_FOLDER",
        paths.webview_dir.to_string_lossy().to_string(),
    );
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--autoplay-policy=no-user-gesture-required",
    );
}

fn run_gui(paths: AppPaths) {
    configure_webview(&paths);
    let config = Config::load(&paths).unwrap_or_default();
    let config = Arc::new(RwLock::new(config));
    let splash = Arc::new(SplashRuntime::new());
    let port = match server::start_server(paths.clone(), config.clone(), splash.clone()) {
        Ok(port) => port,
        Err(err) => {
            eprintln!("启动本地服务失败：{err:#}");
            std::process::exit(1);
        }
    };
    let state = AppState {
        paths,
        config,
        port,
        splash,
        update: Mutex::new(None),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::list_themes,
            commands::set_active_theme,
            commands::import_theme,
            commands::delete_theme,
            commands::get_status,
            commands::run_apply,
            commands::check_updates,
            commands::install_update,
            commands::open_path,
            commands::open_data_dir,
            commands::open_logs_dir,
            commands::read_log,
            commands::test_splash,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}

fn run_splash(paths: AppPaths, args: Vec<String>) {
    configure_webview(&paths);
    let config = Config::load(&paths).unwrap_or_default();
    let test_mode = args.iter().any(|arg| arg == "--test");
    let forward_args: Vec<String> = args
        .into_iter()
        .filter(|arg| arg != "--test")
        .collect();

    if !config.enabled {
        if config.splash.launch_seewo {
            if let Err(err) = splash::launch_seewo(&paths, &config, &forward_args) {
                util::log_line(&paths, "splash.log", &format!("launch failed: {err:#}"));
            }
        }
        return;
    }

    let config_arc = Arc::new(RwLock::new(config.clone()));
    let runtime = Arc::new(SplashRuntime::new());
    let port = match server::start_server(paths.clone(), config_arc.clone(), runtime.clone()) {
        Ok(port) => port,
        Err(err) => {
            util::log_line(&paths, "splash.log", &format!("server failed: {err:#}"));
            if config.splash.launch_seewo {
                let _ = splash::launch_seewo(&paths, &config, &forward_args);
            }
            return;
        }
    };

    let theme_id = config.active_theme.clone();
    let manifest = themes::get_theme(&paths, &theme_id)
        .or_else(|_| themes::get_theme(&paths, "default"));
    let Ok(manifest) = manifest else {
        util::log_line(&paths, "splash.log", "no theme found; launching Seewo directly");
        if config.splash.launch_seewo {
            let _ = splash::launch_seewo(&paths, &config, &forward_args);
        }
        return;
    };
    let entry = manifest
        .entry
        .split('/')
        .map(|segment| urlencoding::encode(segment).to_string())
        .collect::<Vec<_>>()
        .join("/");
    let url_text = format!(
        "http://127.0.0.1:{}/themes/{}/{}",
        port,
        urlencoding::encode(&theme_id),
        entry
    );
    let url = match url::Url::parse(&url_text) {
        Ok(url) => url,
        Err(err) => {
            util::log_line(&paths, "splash.log", &format!("bad theme url: {err}"));
            return;
        }
    };

    let paths_for_thread = paths.clone();
    let config_for_thread = config.clone();
    let runtime_for_thread = runtime.clone();
    let args_for_thread = forward_args.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let _window = tauri::webview::WebviewWindowBuilder::new(
                app,
                "splash",
                tauri::WebviewUrl::External(url),
            )
            .title("Seewo Splash")
            .fullscreen(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(true)
            .visible(true)
            .build()?;

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                splash::run_splash_loop(
                    &paths_for_thread,
                    &config_for_thread,
                    &runtime_for_thread,
                    test_mode,
                    &args_for_thread,
                );
                let _ = handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行启动页失败");
}
