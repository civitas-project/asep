//! Target size measurement and threshold-based cleanup for build directories.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TWO_GIB: u64 = 2 * 1024 * 1024 * 1024;
const FOUR_GIB: u64 = 4 * 1024 * 1024 * 1024;
const SIXTEEN_GIB: u64 = 16 * 1024 * 1024 * 1024;

fn serialize_path<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    path.to_string_lossy().serialize(serializer)
}

/// Result of a cleanup plan or execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CleanReport {
    #[serde(serialize_with = "serialize_path")]
    pub target: PathBuf,
    pub before_bytes: u64,
    pub threshold_bytes: u64,
    pub action: CleanAction,
    pub after_bytes: u64,
    pub removed_bytes: u64,
}

/// Cleanup action taken (or planned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CleanAction {
    None,
    CleanPackage,
    CleanAll,
}

/// Recursively measures the byte size of `path`.
///
/// Symlinks are skipped to avoid loops and per-entry errors are ignored.
pub fn measure_dir(path: &Path) -> u64 {
    let mut total = 0;
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        _ => return 0,
    };
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            _ => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        let child = entry.path();
        if file_type.is_dir() {
            total += measure_dir(&child);
        }
        if file_type.is_file() {
            total += entry.metadata().map_or(0, |metadata| metadata.len());
        }
    }
    total
}

/// `clamp(baseline * 1.5 + 2 GiB, 4 GiB, 16 GiB)` computed in integers
/// (floored for odd baselines; identical to the f64 formula for even ones).
pub fn threshold_bytes(baseline: u64) -> u64 {
    let value = baseline.saturating_mul(3) / 2 + TWO_GIB;
    value.clamp(FOUR_GIB, SIXTEEN_GIB)
}

/// Computes whether `target` exceeds `threshold` and the action to take, without deleting anything.
pub fn plan(target: &Path, threshold: u64) -> CleanReport {
    let before = measure_dir(target);
    #[allow(clippy::match_bool)]
    // Policy AS-CODE-ELSE forbids `else`; match on the bool explicitly instead.
    let (action, after) = match before <= threshold {
        true => (CleanAction::None, before),
        false => (CleanAction::CleanPackage, 0),
    };
    CleanReport {
        target: target.to_path_buf(),
        before_bytes: before,
        threshold_bytes: threshold,
        action,
        after_bytes: after,
        removed_bytes: before.saturating_sub(after),
    }
}

/// Runs a `cargo clean` command and turns a failed launch or non-successful
/// exit into an error. A helper keeps the status check a top-level guard
/// instead of an `if` nested inside the `before > threshold` / `after > threshold`
/// blocks (policy AS-CODE-NESTING / AS-CODE-ELSE).
fn run_cargo_clean(cmd: &mut Command, target: &Path, op: &str) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run cargo clean: {e}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "cargo clean {op}--target-dir {} failed (status: {status})",
        target.display()
    ))
}

/// Performs the cleanup decision for `target` under `root`.
///
/// When `dry_run` is true, the plan is returned without deleting anything.
/// Otherwise runs `cargo clean -p asep` first, then `cargo clean`
/// (CleanAll) if the target is still above threshold.
pub fn execute(
    root: &Path,
    target: &Path,
    threshold: u64,
    dry_run: bool,
) -> Result<CleanReport, String> {
    let before = measure_dir(target);
    if dry_run {
        return Ok(plan(target, threshold));
    }

    let mut action = CleanAction::None;
    let mut after = before;

    if before > threshold {
        let manifest_path = root.join("Cargo.toml");
        let mut cmd = Command::new("cargo");
        cmd.arg("clean")
            .arg("-p")
            .arg("asep")
            .arg("--target-dir")
            .arg(target)
            .arg("--manifest-path")
            .arg(&manifest_path);
        run_cargo_clean(&mut cmd, target, "-p asep ")?;
        action = CleanAction::CleanPackage;
        after = measure_dir(target);
    }

    if after > threshold {
        let mut cmd = Command::new("cargo");
        cmd.arg("clean").arg("--target-dir").arg(target);
        run_cargo_clean(&mut cmd, target, "")?;
        action = CleanAction::CleanAll;
        after = measure_dir(target);
    }

    Ok(CleanReport {
        target: target.to_path_buf(),
        before_bytes: before,
        threshold_bytes: threshold,
        action,
        after_bytes: after,
        removed_bytes: before.saturating_sub(after),
    })
}

/// `<root>/.spec-engine/target-baseline.json`
pub fn baseline_path(root: &Path) -> PathBuf {
    root.join(".spec-engine").join("target-baseline.json")
}

pub fn read_baseline(root: &Path) -> Option<u64> {
    let content = fs::read_to_string(baseline_path(root)).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_baseline(root: &Path, bytes: u64) -> Result<(), String> {
    let path = baseline_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create baseline directory: {e}"))?;
    }
    let json =
        serde_json::to_string(&bytes).map_err(|e| format!("failed to serialize baseline: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("failed to write baseline: {e}"))?;
    Ok(())
}
