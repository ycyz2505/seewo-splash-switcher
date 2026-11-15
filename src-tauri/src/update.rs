use crate::config::{Config, UpdateConfig};
use crate::paths::AppPaths;
use crate::util;
use crate::win32;
use anyhow::{anyhow, Context, Result};
use futures_util::future::join_all;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const PUBKEY_FILE: &str = include_str!("../keys/minisign.pub");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: String,
    pub pub_date: String,
    pub size: u64,
    pub url: String,
    #[serde(skip)]
    pub mirrors: Vec<String>,
    #[serde(skip)]
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateManifest {
    version: String,
    #[serde(default)]
    pub_date: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    min_app_version: String,
    asset: UpdateAsset,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAsset {
    #[allow(dead_code)]
    name: String,
    url: String,
    #[serde(default)]
    mirrors: Vec<String>,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    sha256: String,
}

fn public_key_base64() -> Option<String> {
    let text = PUBKEY_FILE.trim();
    if text.contains("PLACEHOLDER") || text.is_empty() {
        return None;
    }
    text.lines()
        .rev()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment"))
        .map(|line| line.to_string())
}

fn signature_required() -> bool {
    public_key_base64().is_some()
}

fn build_client(config: &UpdateConfig, timeout: Duration) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(format!(
            "seewo-splash-switcher/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .no_proxy();
    match config.proxy_mode.as_str() {
        "direct" => {}
        "manual" => {
            let url = config.proxy_url.trim();
            if url.is_empty() {
                return Err(anyhow!("手动代理地址为空"));
            }
            builder = builder.proxy(
                reqwest::Proxy::all(url)
                    .with_context(|| format!("解析手动代理 {url}"))?,
            );
        }
        _ => {
            if let Some(url) = crate::util::system_proxy_url() {
                builder = builder.proxy(
                    reqwest::Proxy::all(&url)
                        .with_context(|| format!("解析系统代理 {url}"))?,
                );
            }
        }
    }
    builder.build().context("创建 HTTP 客户端")
}

fn verify_minisign(data: &[u8], signature_text: &str) -> Result<()> {
    let Some(pubkey) = public_key_base64() else {
        return Ok(());
    };
    let public_key = PublicKey::from_base64(&pubkey)
        .map_err(|e| anyhow!("解析 minisign 公钥失败：{e}"))?;
    let signature = Signature::decode(signature_text)
        .map_err(|e| anyhow!("解析 minisign 签名失败：{e}"))?;
    public_key
        .verify(data, &signature, false)
        .map_err(|e| anyhow!("minisign 验签失败：{e}"))?;
    Ok(())
}

pub fn mirror_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("direct", "{url}"),
        ("ghproxy", "https://ghproxy.com/{url}"),
        ("ghfast", "https://ghfast.top/{url}"),
        ("gh-proxy", "https://gh-proxy.com/{url}"),
        ("gitmirror", "https://hub.gitmirror.com/{url}"),
    ]
}

fn expand(template: &str, url: &str) -> String {
    if template.contains("{url}") {
        template.replace("{url}", url)
    } else {
        template.to_string()
    }
}

fn manifest_urls(config: &Config) -> Vec<String> {
    let repo = env!("SEEWO_UPDATE_REPO");
    let direct = format!("https://github.com/{repo}/releases/latest/download/latest.json");
    let mut urls = vec![direct.clone()];
    for (_, template) in mirror_templates() {
        urls.push(expand(template, &direct));
    }
    for custom in &config.update.custom_mirrors {
        if !custom.trim().is_empty() {
            urls.push(expand(custom.trim(), &direct));
        }
    }
    urls.sort();
    urls.dedup();
    urls
}

fn asset_urls(info: &UpdateInfo, config: &Config) -> Vec<String> {
    let mut urls = vec![info.url.clone()];
    urls.extend(info.mirrors.iter().cloned());
    for (_, template) in mirror_templates() {
        urls.push(expand(template, &info.url));
    }
    for custom in &config.update.custom_mirrors {
        let custom = custom.trim();
        if custom.is_empty() {
            continue;
        }
        if custom.contains("{url}") {
            urls.push(expand(custom, &info.url));
        }
    }
    urls.sort();
    urls.dedup();
    urls
}

pub fn check(paths: &AppPaths, config: &Config) -> Result<Option<UpdateInfo>> {
    if !config.update.enabled {
        return Ok(None);
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    let urls = manifest_urls(config);
    let client = build_client(&config.update, Duration::from_secs(8))?;

    let rt = tokio::runtime::Runtime::new()?;
    let results = rt.block_on(async {
        let futures = urls.into_iter().map(|url| {
            let client = client.clone();
            async move {
                let response = client.get(&url).send().await?.error_for_status()?;
                let bytes = response.bytes().await?;
                let signature = if signature_required() {
                    let sig_url = format!("{url}.sig");
                    match client.get(&sig_url).send().await {
                        Ok(resp) if resp.status().is_success() => Some(resp.text().await?),
                        _ => None,
                    }
                } else {
                    None
                };
                Ok::<_, reqwest::Error>((url, bytes.to_vec(), signature))
            }
        });
        join_all(futures).await
    });

    let mut best: Option<(semver::Version, UpdateInfo)> = None;
    let mut fetched_any = false;
    for result in results {
        let Ok((url, bytes, signature)) = result else {
            continue;
        };
        if signature_required() {
            let Some(signature) = signature else {
                util::log_line(
                    paths,
                    "app.log",
                    &format!("manifest without signature rejected: {url}"),
                );
                continue;
            };
            if let Err(err) = verify_minisign(&bytes, &signature) {
                util::log_line(
                    paths,
                    "app.log",
                    &format!("manifest signature rejected: {url}: {err:#}"),
                );
                continue;
            }
        }
        let Ok(manifest) = serde_json::from_slice::<UpdateManifest>(&bytes) else {
            continue;
        };
        fetched_any = true;
        let Ok(version) = semver::Version::parse(&manifest.version) else {
            continue;
        };
        let Ok(current_version) = semver::Version::parse(&current) else {
            continue;
        };
        if version <= current_version {
            continue;
        }
        if !manifest.min_app_version.is_empty() {
            if let Ok(min_version) = semver::Version::parse(&manifest.min_app_version) {
                if current_version < min_version {
                    util::log_line(
                        paths,
                        "app.log",
                        "update requires a newer base version; manual update needed",
                    );
                    continue;
                }
            }
        }
        let info = UpdateInfo {
            version: manifest.version,
            current_version: current.clone(),
            notes: manifest.notes,
            pub_date: manifest.pub_date,
            size: manifest.asset.size,
            url: manifest.asset.url,
            mirrors: manifest.asset.mirrors,
            sha256: manifest.asset.sha256,
        };
        if best
            .as_ref()
            .map(|(best_version, _)| version > *best_version)
            .unwrap_or(true)
        {
            best = Some((version, info));
        }
    }
    if !fetched_any {
        return Err(anyhow!("无法从任何源获取更新清单"));
    }
    Ok(best.map(|(_, info)| info))
}

pub fn download_and_stage(paths: &AppPaths, config: &Config, info: &UpdateInfo) -> Result<PathBuf> {
    let download_dir = paths.updates_dir.join("download");
    std::fs::create_dir_all(&download_dir)?;
    let zip_path = download_dir.join(format!("SeewoHtmlSplash-v{}.zip", info.version));
    let client = build_client(&config.update, Duration::from_secs(300))?;
    let rt = tokio::runtime::Runtime::new()?;

    let urls = asset_urls(info, config);
    let mut last_error: Option<anyhow::Error> = None;
    let mut downloaded = false;
    for url in urls {
        util::log_line(paths, "app.log", &format!("downloading update from {url}"));
        match rt.block_on(download_file(&client, &url, &zip_path)) {
            Ok(()) => {
                let size_ok = info.size == 0
                    || std::fs::metadata(&zip_path)
                        .map(|m| m.len() == info.size)
                        .unwrap_or(false);
                if !size_ok {
                    last_error = Some(anyhow!("文件大小不匹配"));
                    continue;
                }
                if !info.sha256.is_empty() {
                    let actual = util::sha256_file(&zip_path)?;
                    if !actual.eq_ignore_ascii_case(&info.sha256) {
                        last_error = Some(anyhow!("SHA256 不匹配"));
                        continue;
                    }
                }
                if signature_required() {
                    match rt.block_on(fetch_text(&client, &format!("{url}.sig"))) {
                        Ok(signature) => {
                            let data = std::fs::read(&zip_path)?;
                            if let Err(err) = verify_minisign(&data, &signature) {
                                last_error = Some(anyhow!("更新包签名校验失败：{err:#}"));
                                continue;
                            }
                        }
                        Err(err) => {
                            last_error = Some(anyhow!("缺少更新包签名：{err:#}"));
                            continue;
                        }
                    }
                }
                downloaded = true;
                break;
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }
    if !downloaded {
        return Err(last_error.unwrap_or_else(|| anyhow!("没有可用的下载源")));
    }

    let staging = paths.updates_dir.join("staging").join(&info.version);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    extract_zip(&zip_path, &staging)?;
    let app_exe = staging.join("app").join("seewo-splash-switcher.exe");
    if !app_exe.exists() {
        return Err(anyhow!("更新包缺少 app\\seewo-splash-switcher.exe"));
    }
    Ok(staging)
}

async fn download_file(client: &reqwest::Client, url: &str, path: &Path) -> Result<()> {
    let mut response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    let mut file = tokio::fs::File::create(path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?.error_for_status()?;
    Ok(response.text().await?)
}

fn extract_zip(zip_path: &Path, destination: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                continue;
            }
        }
        total = total.saturating_add(entry.size());
        if total > 512 * 1024 * 1024 {
            return Err(anyhow!("更新包解压后超过 512MB，已中止"));
        }
        let out = destination.join(path);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)?;
            std::fs::write(&out, buf)?;
        }
    }
    Ok(())
}

fn normalize_staging(staging: &Path) -> Result<PathBuf> {
    let exe = Path::new("app").join("seewo-splash-switcher.exe");
    if staging.join(&exe).exists() {
        return Ok(staging.to_path_buf());
    }
    let dirs: Vec<PathBuf> = std::fs::read_dir(staging)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    if dirs.len() == 1 && dirs[0].join(&exe).exists() {
        return Ok(dirs[0].clone());
    }
    Err(anyhow!("更新包缺少 app\\seewo-splash-switcher.exe"))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn install_from_zip(paths: &AppPaths, zip_path: &Path, restart_args: &str) -> Result<()> {
    if !zip_path.exists() {
        return Err(anyhow!("更新包不存在：{}", zip_path.display()));
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let staging = paths
        .updates_dir
        .join("staging")
        .join(format!("local-{stamp}"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    extract_zip(zip_path, &staging)?;
    let staging_root = normalize_staging(&staging)?;

    let new_theme = staging_root.join("data").join("themes").join("default");
    if new_theme.join("manifest.json").exists() {
        let target = paths.themes_dir.join("default");
        if target.exists() {
            let _ = std::fs::remove_dir_all(&target);
        }
        copy_dir_recursive(&new_theme, &target)?;
        util::log_line(paths, "app.log", "updated default theme from local zip");
    }

    let updater = paths.tool("seewo-updater.exe");
    if !updater.exists() {
        return Err(anyhow!("缺少 {}", updater.display()));
    }
    let app_exe = paths.app_exe();
    let pid = std::process::id();
    std::process::Command::new(&updater)
        .arg("--root")
        .arg(&paths.root)
        .arg("--staging")
        .arg(&staging_root)
        .arg("--pid")
        .arg(pid.to_string())
        .arg("--restart")
        .arg(&app_exe)
        .arg("--restart-args")
        .arg(restart_args)
        .spawn()
        .with_context(|| format!("启动 {}", updater.display()))?;
    util::log_line(
        paths,
        "app.log",
        &format!("local zip install spawned: {}", zip_path.display()),
    );
    Ok(())
}

pub fn install(paths: &AppPaths, staging: &Path, restart_args: &str) -> Result<()> {
    let updater = paths.tool("seewo-updater.exe");
    if !updater.exists() {
        return Err(anyhow!("缺少 {}", updater.display()));
    }
    let app_exe = paths.app_exe();
    let pid = std::process::id();
    std::process::Command::new(&updater)
        .arg("--root")
        .arg(&paths.root)
        .arg("--staging")
        .arg(staging)
        .arg("--pid")
        .arg(pid.to_string())
        .arg("--restart")
        .arg(&app_exe)
        .arg("--restart-args")
        .arg(restart_args)
        .spawn()
        .with_context(|| format!("启动 {}", updater.display()))?;
    util::log_line(paths, "app.log", "updater spawned; exiting");
    std::process::exit(0);
}

pub fn run_boot_update(paths: &AppPaths, config: &Config) {
    if !config.update.enabled {
        return;
    }
    if win32::is_seewo_running() {
        util::log_line(paths, "app.log", "Seewo is running; skip boot update");
        return;
    }
    match check(paths, config) {
        Ok(Some(info)) => {
            util::log_line(paths, "app.log", &format!("update available: {}", info.version));
            if config.update.auto_install {
                match download_and_stage(paths, config, &info) {
                    Ok(staging) => {
                        if let Err(err) = install(paths, &staging, "--boot --silent") {
                            util::log_line(paths, "app.log", &format!("install failed: {err:#}"));
                        }
                    }
                    Err(err) => util::log_line(paths, "app.log", &format!("download failed: {err:#}")),
                }
            }
        }
        Ok(None) => util::log_line(paths, "app.log", "no update available"),
        Err(err) => util::log_line(paths, "app.log", &format!("update check failed: {err:#}")),
    }
}
