//! Self-update: download latest release binary from Codeberg.

use colored::Colorize;
use std::env;
use std::fs;
use std::io::Write;

const REPO_OWNER: &str = "civitas-project";
const REPO_NAME: &str = "asep";
const API_BASE: &str = "https://codeberg.org/api/v1";

#[derive(serde::Deserialize, Debug)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize, Debug)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Returns the target triple for the current platform.
pub fn current_target() -> Option<&'static str> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

/// Fetches the latest release and downloads the matching binary, replacing
/// the current executable. Returns Ok with the new version tag.
pub fn self_update() -> Result<String, String> {
    let target = current_target().ok_or_else(|| {
        format!(
            "unsupported platform: {} {}",
            env::consts::OS,
            env::consts::ARCH
        )
    })?;

    println!("{}", "Checking latest release...".cyan());

    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        API_BASE, REPO_OWNER, REPO_NAME
    );
    let resp = reqwest::blocking::get(&url).map_err(|e| format!("network error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("failed to fetch release: HTTP {}", resp.status()));
    }
    let release: Release = resp.json().map_err(|e| format!("parse error: {}", e))?;
    let tag = release.tag_name.clone();

    // Find asset matching target
    let asset_name_contains = target;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains(asset_name_contains))
        .ok_or_else(|| {
            format!(
                "no release asset found for target '{}' in release {}",
                target, tag
            )
        })?;

    println!("{} Downloading {} ({})...", "✔".green(), asset.name, tag);

    let bytes = reqwest::blocking::get(&asset.browser_download_url)
        .map_err(|e| format!("download error: {}", e))?
        .bytes()
        .map_err(|e| format!("read error: {}", e))?;

    // Determine current executable path
    let exe_path = env::current_exe().map_err(|e| format!("cannot locate current exe: {}", e))?;

    // Write to a temp file next to the exe, then rename (atomic-ish).
    let tmp_path = exe_path.with_extension("new");
    let mut f = fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    f.flush().map_err(|e| e.to_string())?;
    drop(f);

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tmp_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tmp_path, perms).map_err(|e| e.to_string())?;
    }

    // Replace
    // On Windows, rename over running exe may fail; attempt anyway.
    if let Err(e) = fs::rename(&tmp_path, &exe_path) {
        // fallback: remove then rename
        let _ = fs::remove_file(&exe_path);
        fs::rename(&tmp_path, &exe_path).map_err(|e2| format!("replace failed: {} / {}", e, e2))?;
    }

    Ok(tag)
}

/// Returns current binary version.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
