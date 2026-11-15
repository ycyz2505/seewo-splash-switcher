fn main() {
    let repo = std::env::var("SEEWO_UPDATE_REPO")
        .unwrap_or_else(|_| "ycyz2505/seewo-splash-switcher".to_string());
    println!("cargo:rustc-env=SEEWO_UPDATE_REPO={repo}");
    println!("cargo:rerun-if-env-changed=SEEWO_UPDATE_REPO");
    tauri_build::build()
}
