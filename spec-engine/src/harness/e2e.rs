//! E2E harness: scaffold run directories and write `manifest.json`.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
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

/// Required E2E run artifacts (P0-P7).
const REQUIRED_ARTIFACTS: &[&str] = &[
    "manifest.json",
    "01-prd.md",
    "01-judge.json",
    "02-tdd.md",
    "02-judge.json",
    "03-tests.diff",
    "03-red.log",
    "03-hash.txt",
    "04-impl.diff",
    "05-findings.md",
    "05-loom.log",
    "06-optimization.md",
    "07-judge.json",
    "07-check.log",
    "07-ci.log",
];

/// Result of a single re-run command executed during verification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommandResult {
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub exit_code: i32,
    pub passed: bool,
    pub output: String,
}

/// Authoritative verdict for an E2E run directory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifyReport {
    pub pass: bool,
    pub artifacts: BTreeMap<String, bool>,
    pub hash_lock_ok: bool,
    pub test_rerun: CommandResult,
    pub check_rerun: CommandResult,
    pub reasons: Vec<String>,
}

/// Artifact existence check + test hash lock, without spawning `cargo`/`asep`.
///
/// Safe to call in unit tests: it only inspects the run directory.
pub fn verify_preflight(run_dir: &Path) -> (BTreeMap<String, bool>, bool, Vec<String>) {
    let mut artifacts = BTreeMap::new();
    let mut missing = Vec::new();
    for name in REQUIRED_ARTIFACTS {
        let present = run_dir.join(name).is_file();
        artifacts.insert(name.to_string(), present);
        if !present {
            missing.push(name.to_string());
        }
    }

    let mut reasons = Vec::new();
    if !missing.is_empty() {
        reasons.push(format!("missing artifacts: {}", missing.join(", ")));
    }

    let (hash_lock_ok, hash_reason) = test_hash_lock(run_dir);
    if !hash_lock_ok {
        reasons.push(format!("hash lock: {hash_reason}"));
    }

    (artifacts, hash_lock_ok, reasons)
}

/// Test hash lock: sha256 of every `*test*.rs` file under `run_dir`.
///
/// Returns `(ok, reason)`. A `controller-ruling.json` artifact overrides the
/// lock. With no test files the lock is unsatisfiable (`false`).
fn test_hash_lock(run_dir: &Path) -> (bool, String) {
    if run_dir.join("controller-ruling.json").is_file() {
        return (true, "controller ruling overrides hash lock".to_string());
    }

    let mut test_files = Vec::new();
    if collect_test_files(run_dir, &mut test_files).is_err() {
        return (false, "run directory unreadable".to_string());
    }

    if test_files.is_empty() {
        return (false, "no dummy test files found to lock".to_string());
    }

    let stored = fs::read_to_string(run_dir.join("03-hash.txt"))
        .ok()
        .and_then(|content| content.split_whitespace().next().map(|s| s.to_string()));
    let Some(stored) = stored.as_deref() else {
        return (false, "03-hash.txt missing".to_string());
    };

    let mut lines: Vec<String> = test_files
        .iter()
        .map(|path| {
            let hex = sha256_hex_file(path);
            let rel = path
                .strip_prefix(run_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            format!("{hex}  {rel}")
        })
        .collect();
    lines.sort();
    let mut listing = lines.join("\n");
    listing.push('\n');
    let aggregate = sha256_hex(listing.as_bytes());

    let mut single_ok = false;
    for path in &test_files {
        if sha256_hex_file(path) == stored {
            single_ok = true;
            break;
        }
    }

    if stored == aggregate {
        (true, "hash lock matches 03-hash.txt".to_string())
    } else if single_ok {
        (true, "hash lock matches a single test file in 03-hash.txt".to_string())
    } else {
        (
            false,
            "hash lock failed: 03-hash.txt matches neither aggregate nor any single test file"
                .to_string(),
        )
    }
}

/// Recursively collect regular files matching `*test*.rs` under `dir`.
fn collect_test_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, out)?;
        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            if name.contains("test") && name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// Hex-encoded sha256 of `path`'s contents.
fn sha256_hex_file(path: &Path) -> String {
    sha256_hex(&fs::read(path).unwrap_or_default())
}

/// Hex-encoded sha256 of `data`.
fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write;
    let digest = Sha256::digest(data);
    digest
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// Verify an E2E run directory: required artifacts, test hash lock, re-run
/// `cargo test` in `<run_dir>/dummy` and re-run `asep check --rigor T4`.
/// Writes `<run_dir>/verify.json` and returns the report.
///
/// Failure to spawn a command returns `Err(String)`.
pub fn verify_artifacts(run_dir: &Path, repo_root: &Path) -> Result<VerifyReport, String> {
    let (artifacts, hash_lock_ok, mut reasons) = verify_preflight(run_dir);

    let asep_binary = resolve_asep_binary(run_dir, repo_root);
    let dummy = run_dir.join("dummy");

    let test_rerun = run_command(&["cargo".to_string(), "test".to_string()], &dummy);
    let check_rerun = run_command(
        &[
            asep_binary.to_string_lossy().to_string(),
            "check".to_string(),
            dummy.to_string_lossy().to_string(),
            "--rigor".to_string(),
            "T4".to_string(),
        ],
        &dummy,
    );

    let all_artifacts = artifacts.values().all(|present| *present);
    let passed = all_artifacts && hash_lock_ok && test_rerun.passed && check_rerun.passed;

    if !test_rerun.passed {
        reasons.push(format!(
            "cargo test re-run failed (exit {})",
            test_rerun.exit_code
        ));
    }
    if !check_rerun.passed {
        reasons.push(format!(
            "asep check re-run failed (exit {})",
            check_rerun.exit_code
        ));
    }

    let report = VerifyReport {
        pass: passed,
        artifacts,
        hash_lock_ok,
        test_rerun,
        check_rerun,
        reasons,
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("failed to serialize verify report: {e}"))?;
    fs::write(run_dir.join("verify.json"), format!("{json}\n"))
        .map_err(|e| format!("failed to write verify.json: {e}"))?;

    Ok(report)
}

/// `asep_binary` from `manifest.json` when it names an executable file,
/// otherwise `<repo_root>/target/debug/asep`.
fn resolve_asep_binary(run_dir: &Path, repo_root: &Path) -> PathBuf {
    let default = repo_root.join("target/debug/asep");
    let Ok(content) = fs::read_to_string(run_dir.join("manifest.json")) else {
        return default;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) else {
        return default;
    };
    let Some(binary) = manifest.get("asep_binary").and_then(serde_json::Value::as_str) else {
        return default;
    };
    if binary.is_empty() {
        return default;
    }
    let path = PathBuf::from(binary);
    if is_executable_file(&path) {
        return path;
    }
    default
}

fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions())
        .map(|permissions| permissions.mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Spawn `command` with `cwd`, capture combined output.
///
/// If the command cannot be spawned (e.g. `cwd` does not exist), returns a
/// `CommandResult` with `passed: false` and `exit_code: -1` instead of an
/// error, so the verifier can still write `verify.json`.
fn run_command(command: &[String], cwd: &Path) -> CommandResult {
    let display = command.join(" ");
    match Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => match child.wait_with_output() {
            Ok(output) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let passed = output.status.success();
                let mut out = String::from_utf8_lossy(&output.stdout).to_string();
                if !output.stderr.is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                CommandResult {
                    command: command.to_vec(),
                    cwd: cwd.to_path_buf(),
                    exit_code,
                    passed,
                    output: out,
                }
            }
            Err(e) => CommandResult {
                command: command.to_vec(),
                cwd: cwd.to_path_buf(),
                exit_code: -1,
                passed: false,
                output: format!("failed to wait for '{display}': {e}"),
            },
        },
        Err(e) => CommandResult {
            command: command.to_vec(),
            cwd: cwd.to_path_buf(),
            exit_code: -1,
            passed: false,
            output: format!("failed to spawn '{display}': {e}"),
        },
    }
}
