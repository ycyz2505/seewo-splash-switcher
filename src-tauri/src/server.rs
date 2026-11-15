use crate::config::Config;
use crate::paths::AppPaths;
use crate::splash::{SplashRuntime, SplashStatus};
use crate::themes;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct ServerState {
    pub paths: AppPaths,
    pub config: Arc<RwLock<Config>>,
    pub splash: Arc<SplashRuntime>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    #[serde(flatten)]
    splash: SplashStatus,
    theme: String,
    options: serde_json::Value,
}

pub fn start_server(
    paths: AppPaths,
    config_store: Arc<RwLock<Config>>,
    splash: Arc<SplashRuntime>,
) -> anyhow::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    let state = ServerState {
        paths: paths.clone(),
        config: config_store,
        splash,
    };

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return,
        };
        rt.block_on(async move {
            let themes_dir = state.paths.themes_dir.clone();
            let app = Router::new()
                .route("/health", get(health))
                .route("/api/status", get(status))
                .route("/api/enter", post(enter))
                .route("/api/config", get(api_config))
                .route("/_sdk.js", get(sdk))
                .nest_service("/themes", ServeDir::new(themes_dir))
                .layer(CorsLayer::permissive())
                .with_state(state);
            if let Ok(listener) = tokio::net::TcpListener::from_std(listener) {
                let _ = axum::serve(listener, app).await;
            }
        });
    });

    Ok(port)
}

async fn health() -> &'static str {
    "ok"
}

async fn status(State(state): State<ServerState>) -> Json<StatusResponse> {
    let (theme, options) = {
        let cfg = state.config.read().unwrap();
        let theme = cfg.active_theme.clone();
        let options = themes::get_theme(&state.paths, &theme)
            .map(|m| m.options)
            .unwrap_or_else(|_| json!({}));
        (theme, options)
    };
    Json(StatusResponse {
        splash: state.splash.status(),
        theme,
        options,
    })
}

async fn enter(State(state): State<ServerState>) -> Json<serde_json::Value> {
    state.splash.enter();
    Json(json!({ "ok": true }))
}

async fn api_config(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let cfg = state.config.read().unwrap();
    let theme = cfg.active_theme.clone();
    let options = themes::get_theme(&state.paths, &theme)
        .map(|m| m.options)
        .unwrap_or_else(|_| json!({}));
    Json(json!({
        "config": cfg.clone(),
        "theme": theme,
        "options": options
    }))
}

async fn sdk() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        include_str!("sdk.js"),
    )
}
