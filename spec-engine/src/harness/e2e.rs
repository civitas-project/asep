//! E2E harness: scaffold run directories and write `manifest.json`.
//! Port of `harness/e2e/scripts/new-run.sh`.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default run root; overridden by the `RUN_ROOT` environment variable.
pub const DEFAULT_RUN_ROOT: &str = "/tmp/opencode/asep-e2e";

const ASEP_BINARY_REL: &str = "target/debug/asep";
const SECS_PER_DAY: u64 = 86_400;

/// Repo root: parent of this crate's manifest dir at compile time.
fn repo_root() -> PathBuf {
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

/// Deterministic u32 seed: first 4 bytes of sha256(run_id), little-endian.
fn seed_from_run_id(run_id: &str) -> u32 {
    let hash = Sha256::digest(run_id.as_bytes());
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
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
