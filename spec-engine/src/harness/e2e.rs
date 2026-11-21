//! E2E harness: scaffold run directories and write `manifest.json`.
//! Port of `harness/e2e/scripts/new-run.sh`.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Default run root; overridden by the `RUN_ROOT` environment variable.
pub const DEFAULT_RUN_ROOT: &str = "/tmp/opencode/asep-e2e";

const ASEP_BINARY_REL: &str = "target/debug/asep";
const SECS_PER_DAY: u64 = 86_400;

/// Repo root: parent of this crate's manifest dir at compile time.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent directory")
        .to_path_buf()
}

#[derive(serde::Serialize)]
struct Manifest {
    schema_version: u8,
    run_id: String,
    seed: u32,
    rustc_version: String,
    asep_commit: String,
    asep_binary: PathBuf,
    created_at: String,
}

/// Scaffold `<root>/<run-id>/` and write `manifest.json` with the run
/// metadata. Returns the run directory path.
pub fn new_run(root: &Path, run_id: &str) -> Result<PathBuf, String> {
    if run_id.is_empty() {
        return Err("run_id must not be empty".to_string());
    }

    let run_dir = root.join(run_id);
    fs::create_dir_all(&run_dir)
        .map_err(|e| format!("failed to create run directory '{}': {e}", run_dir.display()))?;

    let manifest = Manifest {
        schema_version: 1,
        run_id: run_id.to_string(),
        seed: seed_from_run_id(run_id),
        rustc_version: rustc_version()?,
        asep_commit: asep_commit()?,
        asep_binary: repo_root().join(ASEP_BINARY_REL),
        created_at: now_rfc3339_utc()?,
    };

    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("failed to serialize manifest: {e}"))?;
    let manifest_path = run_dir.join("manifest.json");
    fs::write(&manifest_path, format!("{manifest_json}\n"))
        .map_err(|e| format!("failed to write manifest '{0}': {e}", manifest_path.display()))?;

    Ok(run_dir)
}

/// Deterministic u32 seed: first 4 bytes of sha256(run_id), big-endian.
/// Matches the legacy shell script which parses the first 8 hex chars as a
/// big-endian integer.
pub fn seed_from_run_id(run_id: &str) -> u32 {
    let hash = Sha256::digest(run_id.as_bytes());
    u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
}

fn rustc_version() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("-V")
        .output()
        .map_err(|e| format!("failed to execute 'rustc -V': {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn asep_commit() -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .map_err(|e| format!("failed to execute 'git rev-parse --short HEAD': {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Current UTC time as an RFC 3339 string (`YYYY-MM-DDTHH:MM:SSZ`), std-only.
fn now_rfc3339_utc() -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system time is before UNIX_EPOCH: {e}"))?;
    let days = now.as_secs() / SECS_PER_DAY;
    let secs_of_day = now.as_secs() % SECS_PER_DAY;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Converts days since 1970-01-01 to a Gregorian calendar date (Howard
/// Hinnant's `civil_from_days` algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Copy `src` recursively into `dst` using only the standard library.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("failed to create directory '{}': {e}", dst.display()))?;
    for entry in fs::read_dir(src)
        .map_err(|e| format!("failed to read directory '{}': {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let file_type = entry.file_type().map_err(|e| format!("failed to get file type: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "failed to copy '{}' to '{}': {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Proof crate manifest: sole dependency is vendored `loom = "=0.7.3"`.
const LOOM_PROOF_CARGO_TOML: &str = r#"[package]
name = "loom-proof"
version = "0.0.0"
edition = "2021"

[dependencies]
loom = { version = "=0.7.3", path = "../vendor/loom" }

[features]
loom-test = []
"#;

/// Proof crate source: exercises `loom::model` with two threads and an
/// `AtomicUsize` under feature `loom-test`.
const LOOM_PROOF_LIB_RS: &str = r#"//! Proof that vendored loom builds and runs with feature `loom-test` offline.

#[cfg(feature = "loom-test")]
use loom::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "loom-test")]
use std::sync::Arc;

#[cfg(feature = "loom-test")]
#[test]
fn vendored_loom_explores_schedules() {
    loom::model(|| {
        let flag = Arc::new(AtomicUsize::new(0));
        let t = loom::thread::spawn({
            let flag = Arc::clone(&flag);
            move || {
                flag.store(1, Ordering::Release);
            }
        });
        let _ = t.join();
        assert_eq!(flag.load(Ordering::Acquire), 1);
    });
}
"#;

/// Vendor `loom = "=0.7.3"` into `<run_dir>/vendor/` and write the cargo
/// source replacement at `<run_dir>/.cargo/config.toml`.
///
/// Refuses to overwrite if `<run_dir>/vendor/` or
/// `<run_dir>/.cargo/config.toml` already exists.
pub fn prepare_vendor_dir(run_dir: &Path, repo_root: &Path) -> Result<(), String> {
    let vendor_dir = run_dir.join("vendor");
    let config_dir = run_dir.join(".cargo");
    let config_toml = config_dir.join("config.toml");

    if vendor_dir.exists() || config_toml.exists() {
        return Err(format!(
            "refusing to overwrite existing '{}' or '{}'",
            vendor_dir.display(),
            config_toml.display()
        ));
    }

    let source = repo_root.join("harness").join("e2e").join("vendor-loom");
    copy_dir_all(&source, &vendor_dir)?;

    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("failed to create directory '{}': {e}", config_dir.display()))?;

    // Use an absolute directory replacement so cargo can locate the vendored
    // tree on any platform (no symlink required).
    fs::write(
        &config_toml,
        format!(
            "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n[source.vendored-sources]\ndirectory = \"{}\"\n",
            vendor_dir.display()
        ),
    )
    .map_err(|e| format!("failed to write '{}': {e}", config_toml.display()))?;

    Ok(())
}

/// End-to-end proof that vendored `loom = "=0.7.3"` builds and passes its
/// test suite fully offline. Removes the temporary proof crate on success;
/// on failure returns `Err` carrying cargo's stderr.
pub fn vendor_loom(run_dir: &Path, repo_root: &Path) -> Result<(), String> {
    prepare_vendor_dir(run_dir, repo_root)?;

    let proof_dir = run_dir.join(".loom-proof");
    let src_dir = proof_dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| format!("failed to create directory '{}': {e}", src_dir.display()))?;

    fs::write(proof_dir.join("Cargo.toml"), LOOM_PROOF_CARGO_TOML)
        .map_err(|e| format!("failed to write proof Cargo.toml: {e}"))?;
    fs::write(src_dir.join("lib.rs"), LOOM_PROOF_LIB_RS)
        .map_err(|e| format!("failed to write proof lib.rs: {e}"))?;

    let mut child = Command::new("cargo")
        .args(["test", "--features", "loom-test", "--offline"])
        .env("LOOM_MAX_BRANCHES", "4096")
        .current_dir(&proof_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = fs::remove_dir_all(&proof_dir);
            format!("failed to spawn 'cargo test': {e}")
        })?;

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut pipe| {
                        let mut buffer = String::new();
                        pipe.read_to_string(&mut buffer).ok();
                        buffer
                    })
                    .unwrap_or_default();
                let _ = fs::remove_dir_all(&proof_dir);
                if status.success() {
                    return Ok(());
                }
                return Err(format!(
                    "'cargo test --features loom-test --offline' failed (exit {}): {stderr}",
                    status
                        .code()
                        .map_or_else(|| "-".to_string(), |code| code.to_string())
                ));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_dir_all(&proof_dir);
                    return Err(
                        "'cargo test --features loom-test --offline' timed out after 60 seconds"
                            .to_string(),
                    );
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&proof_dir);
                return Err(format!("failed to wait for 'cargo test': {e}"));
            }
        }
    }
}
