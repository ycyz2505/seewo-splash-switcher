use crate::paths::AppPaths;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub entry: String,
    pub preview: String,
    pub options: serde_json::Value,
}

impl Default for ThemeManifest {
    fn default() -> Self {
        Self {
            schema_version: 1,
            id: "default".to_string(),
            name: "默认".to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            entry: "index.html".to_string(),
            preview: String::new(),
            options: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub preview_url: String,
    pub active: bool,
    pub path: String,
}

pub fn sanitize_id(input: &str) -> String {
    let id: String = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "theme".to_string()
    } else {
        id
    }
}

pub fn ensure_default_theme(paths: &AppPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.themes_dir)?;
    let has_theme = std::fs::read_dir(&paths.themes_dir)
        .map(|mut it| {
            it.any(|entry| {
                entry
                    .ok()
                    .map(|e| e.path().is_dir() && e.path().join("manifest.json").exists())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if has_theme {
        return Ok(());
    }
    let src = paths.default_data_dir().join("themes");
    if src.exists() {
        copy_dir_recursive(&src, &paths.themes_dir)?;
    }
    Ok(())
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

pub fn read_manifest(dir: &Path) -> Result<ThemeManifest> {
    let path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let mut manifest: ThemeManifest =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if manifest.id.trim().is_empty() {
        manifest.id = dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "theme".to_string());
    }
    if manifest.entry.trim().is_empty() {
        manifest.entry = "index.html".to_string();
    }
    Ok(manifest)
}

pub fn list_themes(paths: &AppPaths, active_id: &str, port: u16) -> Vec<ThemeInfo> {
    let mut result = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(&paths.themes_dir) else {
        return result;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("manifest.json").exists() {
            continue;
        }
        let Ok(manifest) = read_manifest(&path) else {
            continue;
        };
        let id = manifest.id.clone();
        let preview = if manifest.preview.trim().is_empty() {
            manifest.entry.clone()
        } else {
            manifest.preview.clone()
        };
        let preview_url = format!(
            "http://127.0.0.1:{}/themes/{}/{}",
            port,
            urlencoding::encode(&id),
            preview
                .split('/')
                .map(|s| urlencoding::encode(s).to_string())
                .collect::<Vec<_>>()
                .join("/")
        );
        result.push(ThemeInfo {
            id: id.clone(),
            name: manifest.name,
            description: manifest.description,
            version: manifest.version,
            preview_url,
            active: id == active_id,
            path: path.to_string_lossy().to_string(),
        });
    }
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

pub fn get_theme(paths: &AppPaths, id: &str) -> Result<ThemeManifest> {
    let id = sanitize_id(id);
    let dir = paths.theme_dir(&id);
    if !dir.exists() {
        return Err(anyhow!("主题不存在：{}", id));
    }
    let mut manifest = read_manifest(&dir)?;
    if manifest.id != id {
        manifest.id = id;
    }
    Ok(manifest)
}

pub fn set_active_theme(paths: &AppPaths, config: &mut crate::config::Config, id: &str) -> Result<()> {
    let id = sanitize_id(id);
    if !paths.theme_dir(&id).join("manifest.json").exists() {
        return Err(anyhow!("主题不存在：{}", id));
    }
    config.active_theme = id;
    config.save(paths)
}

pub fn delete_theme(paths: &AppPaths, active_id: &str, id: &str) -> Result<()> {
    let id = sanitize_id(id);
    if id == sanitize_id(active_id) {
        return Err(anyhow!("不能删除当前正在使用的主题"));
    }
    let dir = paths.theme_dir(&id);
    if !dir.exists() {
        return Err(anyhow!("主题不存在：{}", id));
    }
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

pub fn import_theme_zip(paths: &AppPaths, zip_path: &Path) -> Result<String> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).context("打开主题 zip")?;

    let mut manifest_candidates: Vec<PathBuf> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if let Some(path) = entry.enclosed_name() {
            if path
                .file_name()
                .map(|n| n.eq_ignore_ascii_case("manifest.json"))
                .unwrap_or(false)
            {
                manifest_candidates.push(path);
            }
        }
    }
    manifest_candidates.sort_by_key(|p| p.components().count());
    let manifest_path = manifest_candidates
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("主题包里没有 manifest.json"))?;
    let prefix = manifest_path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    let suffix: u32 = rand::random();
    let temp_dir = paths.themes_dir.join(format!(".import-{suffix:08x}"));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }
    std::fs::create_dir_all(&temp_dir)?;

    let result = (|| -> Result<String> {
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let Some(path) = entry.enclosed_name() else {
                continue;
            };
            if !path.starts_with(&prefix) {
                continue;
            }
            let rel = path.strip_prefix(&prefix).unwrap_or(path.as_path());
            if rel.as_os_str().is_empty() {
                continue;
            }
            if let Some(mode) = entry.unix_mode() {
                if mode & 0o170000 == 0o120000 {
                    continue;
                }
            }
            let out = temp_dir.join(rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&out)?;
            } else {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&out, buf)?;
            }
        }

        let manifest = read_manifest(&temp_dir)?;
        let id = sanitize_id(&manifest.id);
        if !temp_dir.join(&manifest.entry).exists() {
            return Err(anyhow!("主题入口文件不存在：{}", manifest.entry));
        }
        let target = paths.theme_dir(&id);
        if target.exists() {
            return Err(anyhow!("主题已存在：{}", id));
        }
        std::fs::rename(&temp_dir, &target)?;
        Ok(id)
    })();

    if result.is_err() && temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    result
}
