//! Self-update: download a release asset only when its integrity can be
//! verified, and replace the current binary without ever deleting the working
//! copy first.

use colored::Colorize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

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
    /// Trusted digest published with the release, when available.
    #[serde(default, alias = "sha256")]
    digest: Option<String>,
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

/// Select exactly one asset for this target. A loose substring match can pick
/// the wrong artifact (for example a debug build), so ambiguous releases are
/// rejected rather than guessed.
fn select_asset<'a>(assets: &'a [Asset], target: &str) -> Result<&'a Asset, String> {
    let matches: Vec<&Asset> = assets
        .iter()
        .filter(|asset| {
            let stem = asset
                .name
                .trim_end_matches(".exe")
                .trim_end_matches(".tar.gz")
                .trim_end_matches(".zip")
                .trim_end_matches(".gz");
            stem == target || stem.ends_with(&format!("-{target}")) || stem.contains(&format!("{target}."))
        })
        .collect();
    match matches.as_slice() {
        [asset] => Ok(asset),
        [] => Err(format!("no release asset found for target '{target}'")),
        _ => Err(format!(
            "ambiguous release assets for target '{target}': {}",
            matches
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Verifies the downloaded bytes against the digest published with the asset.
/// Without published integrity metadata automatic replacement is disabled;
/// this milestone does not invent a signing infrastructure.
fn verify_artifact(bytes: &[u8], digest: Option<&str>) -> Result<(), String> {
    let expected = digest.ok_or_else(|| {
        "artifact integrity unavailable; automatic self-update disabled".to_string()
    })?;
    let expected = expected.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("published artifact digest is not a SHA-256 hex value".to_string());
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "artifact digest mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

/// Writes a verified binary to a uniquely created temporary file beside the
/// target and renames it into place. The original is never removed first, so
/// every failure path leaves the working binary intact.
fn replace_binary(exe_path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = exe_path
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_string())?;
    let stem = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "executable name is not valid UTF-8".to_string())?;

    let mut tmp_path = None;
    for attempt in 0..10_000u32 {
        let candidate = parent.join(format!(
            ".{stem}.asep-new-{}-{attempt}",
            std::process::id()
        ));
        match fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => {
                tmp_path = Some((candidate, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("cannot create update temp file: {e}")),
        }
    }
    let (tmp_path, mut file) = tmp_path.ok_or_else(|| "cannot reserve update temp file".to_string())?;

    let write_result = (|| -> Result<(), String> {
        file.write_all(bytes).map_err(|e| format!("update write failed: {e}"))?;
        file.flush().map_err(|e| format!("update flush failed: {e}"))?;
        drop(file);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&tmp_path)
                .map_err(|e| format!("update metadata failed: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tmp_path, perms)
                .map_err(|e| format!("update permissions failed: {e}"))?;
        }
        fs::rename(&tmp_path, exe_path)
            .map_err(|e| format!("replacement failed: {e} (original binary preserved)"))
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_result
}

/// Fetches the latest release, verifies the intended artifact, and only then
/// replaces the current executable. Returns Ok with the new version tag.
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
    let response = reqwest::blocking::get(&url).map_err(|e| format!("network error: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("failed to fetch release: HTTP {}", response.status()));
    }
    let release: Release = response.json().map_err(|e| format!("parse error: {}", e))?;
    let tag = release.tag_name.clone();
    let asset = select_asset(&release.assets, target)?;

    println!("{} Downloading {} ({})...", "✔".green(), asset.name, tag);

    let download = reqwest::blocking::get(&asset.browser_download_url)
        .map_err(|e| format!("download error: {}", e))?;
    if !download.status().is_success() {
        return Err(format!("asset download failed: HTTP {}", download.status()));
    }
    let bytes = download.bytes().map_err(|e| format!("read error: {}", e))?;
    verify_artifact(&bytes, asset.digest.as_deref())?;

    let exe_path = env::current_exe().map_err(|e| format!("cannot locate current exe: {}", e))?;
    replace_binary(&exe_path, &bytes)?;
    Ok(tag)
}

/// Returns current binary version.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn asset(name: &str, digest: Option<&str>) -> Asset {
        Asset {
            name: name.to_string(),
            browser_download_url: "http://invalid.invalid/asset".to_string(),
            digest: digest.map(str::to_string),
        }
    }

    #[test]
    // @test:R04
    fn asset_selection_is_exact_and_rejects_ambiguous_or_wrong_target() {
        let assets = vec![
            asset("asep-x86_64-unknown-linux-gnu", None),
            asset("asep-x86_64-unknown-linux-gnu-debug", None),
        ];
        let selected = select_asset(&assets, "x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(selected.name, "asep-x86_64-unknown-linux-gnu");
        assert!(select_asset(&assets, "aarch64-unknown-linux-gnu").is_err());

        let ambiguous = vec![
            asset("asep-x86_64-unknown-linux-gnu", None),
            asset("tool-x86_64-unknown-linux-gnu", None),
        ];
        assert!(select_asset(&ambiguous, "x86_64-unknown-linux-gnu").is_err());
    }

    #[test]
    // @test:R04
    fn integrity_is_required_and_mismatch_is_rejected() {
        let bytes = b"verified artifact";
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_artifact(bytes, None).is_err());
        assert!(verify_artifact(bytes, Some("not-a-digest")).is_err());
        assert!(verify_artifact(bytes, Some(&digest)).is_ok());
        assert!(verify_artifact(b"tampered", Some(&digest)).is_err());
    }

    #[test]
    // @test:R04
    fn failed_replacement_preserves_existing_binary() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("asep");
        fs::write(&exe, b"working binary").unwrap();
        // A directory cannot be replaced by a file rename; the original stays.
        let target = dir.path().join("occupied");
        fs::create_dir(&target).unwrap();
        assert!(replace_binary(&target, b"new binary").is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"working binary");
        assert!(target.is_dir());
    }

    #[test]
    // @test:R04
    fn successful_replacement_uses_verified_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("asep");
        fs::write(&exe, b"old").unwrap();
        replace_binary(&exe, b"new").expect("replacement succeeds");
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert!(
            fs::read_dir(dir.path())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains("asep-new")),
            "temporary update file must be cleaned up"
        );
    }
}
