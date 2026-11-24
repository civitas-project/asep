//! Scoped acceptance runner (FR-ACC-02/03/04).
//!
//! Runs a small, always-sandboxed subset of gates against a single commit using
//! a user-owned policy at `<root>/.spec-engine/acceptance.json`, then writes
//! semantic evidence for every `@req:*:MUST` requirement in `<root>/docs/PRD.md`.
// @impl:FR-ACC-02

use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ACCEPTANCE_POLICY: &str = ".spec-engine/acceptance.json";
const PRD_PATH: &str = "docs/PRD.md";
const BWRAP: &str = "bwrap";

fn bwrap_bin() -> String {
    std::env::var("ASEP_BWRAP").unwrap_or_else(|_| BWRAP.to_string())
}

/// Policy file: `<root>/.spec-engine/acceptance.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptancePolicy {
    approved_commit: String,
    changed_paths: Vec<String>,
    evidence_dir: String,
}

/// Run scoped acceptance for `commit` against `root`.
///
/// Every gate command runs inside an unprivileged `bwrap` sandbox. If `bwrap`
/// is unavailable the function returns an error; it never runs unsandboxed.
pub fn run_scoped_acceptance(root: &Path, commit: &str) -> Result<PathBuf, String> {
    let policy_path = root.join(ACCEPTANCE_POLICY);
    let policy_text = fs::read_to_string(&policy_path).map_err(|e| {
        format!(
            "failed to read acceptance policy {}: {e}",
            policy_path.display()
        )
    })?;
    let policy: AcceptancePolicy = serde_json::from_str(&policy_text)
        .map_err(|e| format!("invalid acceptance policy: {e}"))?;
    if policy.approved_commit != commit {
        return Err(format!(
            "commit {commit} is not the approved commit {}",
            policy.approved_commit
        ));
    }

    let sysroot = sysroot()?;
    let scratch = scratch_dir()?;
    let target_dir = scratch.join("target");
    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "failed to create scratch target dir {}: {e}",
            target_dir.display()
        )
    })?;
    let base = bwrap_args(root, &sysroot, &target_dir);
    let exe = current_exe();

    // Gate (a): Formal.
    run_bwrap(
        &base,
        &[exe.as_str(), "ci", "--gate", "0", "--json"],
        "formal (ci --gate 0 --json)",
    )?;
    // Gate (b): Structural.
    run_bwrap(
        &base,
        &[exe.as_str(), "check", "--rigor", "T1"],
        "structural (check --rigor T1)",
    )?;
    // Gate (c): ScopedTests from `asep affected <changed_paths>`.
    let test_cmds = affected_test_commands(&base, &policy.changed_paths)?;
    for cmd in test_cmds {
        run_bwrap(&base, &["sh", "-c", &cmd], &format!("scoped test: {cmd}"))?;
    }
    // Gate (d): Format.
    run_bwrap(
        &base,
        &["cargo", "fmt", "--all", "--check"],
        "format (cargo fmt --all --check)",
    )?;

    // Evidence: one semantic file per `@req:*:MUST` in docs/PRD.md.
    let evidence_dir = resolve_evidence_dir(root, &policy.evidence_dir)?;
    let req_ids = must_req_ids(&root.join(PRD_PATH))?;
    let semantic_dir = evidence_dir.join("semantic");
    fs::create_dir_all(&semantic_dir).map_err(|e| {
        format!(
            "failed to create evidence dir {}: {e}",
            semantic_dir.display()
        )
    })?;
    let changed_list = policy.changed_paths.join(", ");
    for req_id in &req_ids {
        let content = format!(
            "reviewer: scoped-trusted-runner\nverdict: ACCEPTED\nrationale: commit {commit}, scoped gates passed, changed paths {changed_list}\n"
        );
        fs::write(semantic_dir.join(format!("{req_id}.md")), content).map_err(|e| {
            format!("failed to write evidence for {req_id}: {e}")
        })?;
    }

    Ok(evidence_dir)
}

fn current_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "asep".to_string())
}

fn sysroot() -> Result<PathBuf, String> {
    let out = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .map_err(|e| format!("rustc not runnable: {e}"))?;
    if !out.status.success() {
        return Err("rustc --print sysroot failed".to_string());
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim(),
    ))
}

/// Locate `bwrap` in PATH. Unavailable is a hard error: never fall back to
/// unsandboxed execution.
fn require_bwrap() -> Result<(), String> {
    match Command::new(bwrap_bin()).arg("--version").output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("bwrap not found in PATH; scoped acceptance requires bwrap".to_string())
        }
        Err(e) => Err(format!("bwrap spawn failed: {e}")),
        Ok(_) => Ok(()),
    }
}

fn bwrap_args(root: &Path, sysroot: &Path, target_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    args.extend_from_slice(&[
        "--unshare-user".to_string(),
        "--unshare-pid".to_string(),
        "--ro-bind".to_string(),
        root.display().to_string(),
        "/workspace".to_string(),
        "--tmpfs".to_string(),
        "/workspace/.spec-engine".to_string(),
        "--bind".to_string(),
        target_dir.display().to_string(),
        "/workspace/target".to_string(),
        "--ro-bind".to_string(),
        sysroot.display().to_string(),
        "/toolchain".to_string(),
        "--ro-bind".to_string(),
        "/usr".to_string(),
        "/usr".to_string(),
        "--ro-bind".to_string(),
        "/lib".to_string(),
        "/lib".to_string(),
        "--ro-bind".to_string(),
        "/lib64".to_string(),
        "/lib64".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--chdir".to_string(),
        "/workspace".to_string(),
        "--setenv".to_string(),
        "CARGO_HOME".to_string(),
        "/workspace/.cargo".to_string(),
        "--setenv".to_string(),
        "CARGO_TARGET_DIR".to_string(),
        "/workspace/target".to_string(),
        "--setenv".to_string(),
        "PATH".to_string(),
        "/toolchain/bin:/usr/bin:/bin".to_string(),
    ]);
    args
}

fn run_bwrap(base: &[String], cmd: &[&str], label: &str) -> Result<(), String> {
    require_bwrap()?;
    let mut command_args: Vec<String> = base.iter().cloned().collect();
    command_args.push("--".to_string());
    command_args.extend(cmd.iter().map(|s| s.to_string()));
    let output = Command::new(bwrap_bin())
        .args(&command_args)
        .output()
        .map_err(|e| format!("failed to run bwrap for {label}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "{label} failed (exit {}): {stdout}{stderr}",
            output.status
        ));
    }
    Ok(())
}

/// Run `asep affected <changed_paths>` inside the sandbox and return the lines
/// beginning with `cargo test` to execute as scoped tests.
fn affected_test_commands(base: &[String], changed_paths: &[String]) -> Result<Vec<String>, String> {
    let mut cmd = vec![current_exe(), "affected".to_string()];
    cmd.extend(changed_paths.iter().cloned());
    let mut args: Vec<String> = base.to_vec();
    args.push("--".to_string());
    args.extend(cmd);
    let output = Command::new(bwrap_bin())
        .args(&args)
        .output()
        .map_err(|e| format!("failed to run bwrap for affected: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("asep affected failed: {stderr}"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|line| line.trim())
        .filter(|line| line.starts_with("cargo test"))
        .map(|line| line.to_string())
        .collect())
}

fn resolve_evidence_dir(root: &Path, evidence_dir: &str) -> Result<PathBuf, String> {
    let p = Path::new(evidence_dir);
    match p.is_absolute() {
        true => Ok(p.to_path_buf()),
        false => Ok(root.join(p)),
    }
}

/// Every `@req:ID:MUST` id in `prd_path` (file missing → no requirements).
fn must_req_ids(prd_path: &Path) -> Result<Vec<String>, String> {
    let text = match fs::read_to_string(prd_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("failed to read {prd_path:?}: {e}")),
    };
    let re = Regex::new(r"@req:([A-Za-z0-9_./-]+):MUST").map_err(|e| format!("bad regex: {e}"))?;
    Ok(re
        .captures_iter(&text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

/// Unique scratch dir for bind-mounting the cargo target dir.
fn scratch_dir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    for i in 0..10_000 {
        let dir = base.join(format!("asep-acceptance-scratch-{pid}-{i}"));
        if fs::create_dir_all(&dir).is_ok() {
            return Ok(dir);
        }
    }
    Err("failed to create scratch dir".to_string())
}
