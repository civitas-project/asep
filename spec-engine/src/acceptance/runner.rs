#![cfg(target_os = "linux")]

//! Trusted local acceptance runner for a single tested snapshot.
//!
//! Fail-closed: `accept_commit` provisions and installs nothing. Every trust
//! boundary (root identity, dispatch policy, pinned toolchain, trusted git
//! repo, root-only evidence dir, bubblewrap isolation) must already be
//! provisioned, otherwise it returns an error and writes no evidence.
//! The executed acceptance gates run inside an isolated sandbox; the trusted
//! structural gate inspects the extracted snapshot in-process and both are
//! evaluated with `crate::acceptance::evaluate_acceptance`.

use super::{
    evaluate_acceptance, AcceptanceEvidence, AcceptanceGate, AcceptanceVerdict, GateEvidence,
    SnapshotIdentity, MANDATORY_GATES, SCHEMA_VERSION,
};
use crate::ci::CiReport;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

const ACCEPTANCE_POLICY: &str = "/etc/asep/acceptance.json";
const GIT_BIN: &str = "/usr/bin/git";
const BWRAP_BIN: &str = "/usr/bin/bwrap";
const TAR_BIN: &str = "/usr/bin/tar";
const ASEP_DEST: &str = "/asep/bin/asep";
const CARGO_DEST: &str = "/toolchain/bin/cargo";
const TOOLCHAIN_DEST: &str = "/toolchain";
const SNAPSHOT_DEST: &str = "/snapshot.tar";
const WORKSPACE_DEST: &str = "/workspace";
const SCRATCH_DEST: &str = "/scratch";

/// Pin the toolchain identity exactly: the provisioned directory basename must
/// be `nightly-2026-11-14` and `rustc -vV` must report commit-date
/// `2026-11-13` (the commit date, not the distro date) with commit-hash
/// `5f4effd579500175af844241eb179e756190f1e7` (the full hash, not a prefix).
/// Fail closed if unsupported.
const TOOLCHAIN_DIR_NAME: &str = "nightly-2026-11-14";
const TOOLCHAIN_COMMIT_DATE: &str = "2026-11-13";
const TOOLCHAIN_COMMIT_HASH: &str = "5f4effd579500175af844241eb179e756190f1e7";

/// Strict dispatch policy. Only these fields are read; unknown fields are
/// rejected and workspace JSON is never accepted as evidence.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptancePolicy {
    approved_commit: String,
    snapshot_sha256: String,
    repo: String,
    toolchain: String,
    evidence_dir: String,
}

/// Output observed by the parent from an isolated subprocess.
struct IsolatedOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Unique private staging directory removed on drop (RAII). Never reused.
struct StagingDir(PathBuf);

impl StagingDir {
    fn new(evidence_dir: &Path, commit: &str) -> Result<Self, String> {
        let dir = evidence_dir.join(format!(".staging-{commit}-{}", std::process::id()));
        if dir.exists() {
            return Err(format!("staging dir already exists: {dir:?}"));
        }
        // Atomic private create: fail if present, force exact mode 0700
        // regardless of the caller umask.
        DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|e| format!("cannot create staging dir {dir:?}: {e}"))?;
        fs::set_permissions(&dir, Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set staging dir mode {dir:?}: {e}"))?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// @impl:FR-ACCEPT-01
/// Accept an exact commit snapshot offline.
pub fn accept_commit(commit: &str) -> Result<PathBuf, String> {
    validate_commit_sha(commit)?;
    ensure_root()?;
    // Policy is a trust boundary: root-owned, not group/world writable.
    protect_file(Path::new(ACCEPTANCE_POLICY), "acceptance policy")?;
    let (policy, policy_bytes) = load_policy()?;
    if policy.approved_commit != commit {
        return Err(format!(
            "commit {commit} is not the approved commit {}",
            policy.approved_commit
        ));
    }
    let policy_sha256 = sha256_hex(&policy_bytes);
    let repo = protect_dir(Path::new(&policy.repo), "repo")?;
    let toolchain = protect_dir(Path::new(&policy.toolchain), "toolchain")?;
    let evidence_dir = protect_dir(Path::new(&policy.evidence_dir), "evidence_dir")?;
    check_evidence_dir(&evidence_dir, "evidence_dir")?;
    let cargo = toolchain.join("bin").join("cargo");
    let rustc = toolchain.join("bin").join("rustc");
    if !cargo.is_file() {
        return Err(format!("trusted toolchain missing cargo: {cargo:?}"));
    }
    if !rustc.is_file() {
        return Err(format!("trusted toolchain missing rustc: {rustc:?}"));
    }
    // Protect the real toolchain binaries before executing them.
    protect_file(&cargo, "trusted cargo")?;
    protect_file(&rustc, "trusted rustc")?;
    verify_toolchain_identity(&toolchain)?;

    let exe = env::current_exe().map_err(|e| format!("cannot locate current exe: {e}"))?;
    protect_file(&exe, "trusted asep exe")?;
    let runner_id = hash_file(&exe)?;

    let staging = StagingDir::new(&evidence_dir, commit)?;
    let tar_path = staging.path().join("snapshot.tar");
    // Compare approved digest before executing any candidate code.
    let snapshot_digest = git_archive_to_tar(&repo, commit, &tar_path)?;
    if snapshot_digest != policy.snapshot_sha256 {
        return Err(format!(
            "snapshot digest mismatch for {commit}: expected {} got {}",
            policy.snapshot_sha256, snapshot_digest
        ));
    }

    let approved = SnapshotIdentity {
        commit_sha: commit.to_string(),
        snapshot_sha256: policy.snapshot_sha256.clone(),
        policy_sha256,
        runner_id,
        toolchain_id: toolchain.to_string_lossy().to_string(),
        schema_version: SCHEMA_VERSION,
    };

    let snapshot_out = staging.path().join("out");
    extract_snapshot_sandbox(&tar_path, &snapshot_out)?;

    let evidence = run_gates(&snapshot_out, &toolchain, &cargo, &exe, &approved)?;

    match evaluate_acceptance(&approved, &evidence) {
        AcceptanceVerdict::Accepted => {}
        AcceptanceVerdict::Rejected(reason) => {
            return Err(format!("acceptance rejected for {commit}: {reason:?}"));
        }
    }

    let evidence_path = evidence_dir.join(format!("acceptance-{commit}.json"));
    write_evidence(&evidence_path, &evidence)?;
    Ok(evidence_path)
}

fn validate_commit_sha(commit: &str) -> Result<(), String> {
    if commit.len() != 40 {
        return Err(format!(
            "commit must be a full 40-char lowercase SHA, got {} chars",
            commit.len()
        ));
    }
    if !commit.bytes().all(is_lowercase_hex) {
        return Err("commit must be full lowercase hex".to_string());
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), String> {
    if value.len() != 64 {
        return Err(format!(
            "snapshot digest must be 64 lowercase hex, got {} chars",
            value.len()
        ));
    }
    if !value.bytes().all(is_lowercase_hex) {
        return Err("snapshot digest must be lowercase hex".to_string());
    }
    Ok(())
}

fn is_lowercase_hex(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'a'..=b'f')
}

/// Require real and effective UID root via /proc/self/status.
fn ensure_root() -> Result<(), String> {
    let status = fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("cannot read /proc/self/status: {e}"))?;
    let mut real: Option<u32> = None;
    let mut effective: Option<u32> = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let fields: Vec<&str> = rest.split_whitespace().collect();
            let enough = fields.len() >= 2;
            #[allow(clippy::match_bool)]
            match enough {
                true => {
                    real = fields[0].parse::<u32>().ok();
                    effective = fields[1].parse::<u32>().ok();
                }
                false => {}
            }
            break;
        }
    }
    let real = real.ok_or_else(|| "Uid field missing in /proc/self/status".to_string())?;
    let effective =
        effective.ok_or_else(|| "effective Uid field missing in /proc/self/status".to_string())?;
    #[allow(clippy::match_bool)]
    match (real == 0, effective == 0) {
        (true, true) => Ok(()),
        _ => Err(format!(
            "runner must run as real and effective UID root; got real={real} effective={effective}"
        )),
    }
}

fn load_policy() -> Result<(AcceptancePolicy, Vec<u8>), String> {
    let bytes = fs::read(ACCEPTANCE_POLICY)
        .map_err(|e| format!("acceptance policy not provisioned at {ACCEPTANCE_POLICY}: {e}"))?;
    let policy: AcceptancePolicy = serde_json::from_slice(&bytes).map_err(|e| {
        format!("acceptance policy {ACCEPTANCE_POLICY} has invalid strict schema: {e}")
    })?;
    require_nonempty(&policy.approved_commit, "approved_commit")?;
    require_nonempty(&policy.snapshot_sha256, "snapshot_sha256")?;
    require_nonempty(&policy.repo, "repo")?;
    require_nonempty(&policy.toolchain, "toolchain")?;
    require_nonempty(&policy.evidence_dir, "evidence_dir")?;
    validate_commit_sha(&policy.approved_commit)?;
    validate_digest(&policy.snapshot_sha256)?;
    Ok((policy, bytes))
}

fn require_nonempty(value: &str, field: &'static str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("policy field '{field}' must be nonempty"));
    }
    Ok(())
}

/// Reject agent-writable roots: every component of the path must be a real
/// directory, root-owned, without group/world writes, and free of symlinks.
/// Trusted root administrators own their descendants by construction.
fn protect_dir(path: &Path, label: &str) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("{label} must be an absolute path: {path:?}"));
    }
    check_protected(path, label)?;
    let md = fs::metadata(path).map_err(|e| format!("{label} cannot stat {path:?}: {e}"))?;
    if !md.is_dir() {
        return Err(format!("{label} is not a directory: {path:?}"));
    }
    Ok(path.to_path_buf())
}

fn protect_file(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!("{label} must be an absolute path: {path:?}"));
    }
    check_protected(path, label)?;
    let md = fs::metadata(path).map_err(|e| format!("{label} cannot stat {path:?}: {e}"))?;
    if !md.is_file() {
        return Err(format!("{label} is not a regular file: {path:?}"));
    }
    Ok(())
}

fn check_protected(path: &Path, label: &str) -> Result<(), String> {
    let mut cur = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                cur.push(name);
                check_entry(&cur, label)?;
            }
            _ => {
                return Err(format!("{label} unsupported path component: {path:?}"));
            }
        }
    }
    Ok(())
}

fn check_entry(path: &Path, label: &str) -> Result<(), String> {
    let md =
        fs::symlink_metadata(path).map_err(|e| format!("{label} cannot stat {path:?}: {e}"))?;
    if md.file_type().is_symlink() {
        return Err(format!("{label} symlink rejected: {path:?}"));
    }
    if md.uid() != 0 {
        return Err(format!(
            "{label} not root-owned: {path:?} (uid {})",
            md.uid()
        ));
    }
    if md.mode() & 0o022 != 0 {
        return Err(format!(
            "{label} group/world writable: {path:?} (mode {:#o})",
            md.mode()
        ));
    }
    Ok(())
}

/// Evidence dir guard: root-owned and 0700 or stricter (no group/other
/// bits on the dir itself). Ancestor checks remain in `check_protected`.
fn check_evidence_dir(path: &Path, label: &str) -> Result<(), String> {
    let md = fs::metadata(path).map_err(|e| format!("{label} cannot stat {path:?}: {e}"))?;
    if md.uid() != 0 {
        return Err(format!(
            "{label} not root-owned: {path:?} (uid {})",
            md.uid()
        ));
    }
    if md.mode() & 0o077 != 0 {
        return Err(format!(
            "{label} must be 0700 or stricter: {path:?} (mode {:#o})",
            md.mode()
        ));
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {path:?}: {e}"))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

fn verify_toolchain_identity(toolchain: &Path) -> Result<(), String> {
    let dir_name = toolchain
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "toolchain has no directory name".to_string())?;
    if dir_name != TOOLCHAIN_DIR_NAME {
        return Err(format!(
            "unsupported toolchain directory name: {dir_name} (expected {TOOLCHAIN_DIR_NAME})"
        ));
    }
    let rustc = toolchain.join("bin").join("rustc");
    let out = Command::new(&rustc)
        .arg("-vV")
        .env_clear()
        .env("PATH", "/usr/bin")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run {rustc:?} -vV: {e}"))?;
    if !out.status.success() {
        return Err(format!("rustc -vV failed: {rustc:?}"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (commit_hash, commit_date) = parse_rustc_vv(&text)?;
    if commit_date != TOOLCHAIN_COMMIT_DATE {
        return Err(format!(
            "toolchain identity mismatch: provisioned {TOOLCHAIN_COMMIT_DATE} vs rustc {commit_date}"
        ));
    }
    if commit_hash != TOOLCHAIN_COMMIT_HASH {
        return Err(format!(
            "toolchain identity mismatch: commit-hash {commit_hash} != {TOOLCHAIN_COMMIT_HASH}"
        ));
    }
    Ok(())
}

/// Parse `rustc -vV` output: full `commit-hash:` SHA and `commit-date:`.
/// Fail closed when either line is missing.
fn parse_rustc_vv(text: &str) -> Result<(String, String), String> {
    let mut commit_hash: Option<String> = None;
    let mut commit_date: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("commit-date:") {
            commit_date = Some(rest.trim().to_string());
        }
        if let Some(rest) = line.strip_prefix("commit-hash:") {
            commit_hash = Some(rest.trim().to_string());
        }
    }
    let commit_date = commit_date
        .ok_or_else(|| "rustc -vV did not report commit-date; cannot pin toolchain".to_string())?;
    let commit_hash = commit_hash
        .ok_or_else(|| "rustc -vV did not report commit-hash; cannot pin toolchain".to_string())?;
    Ok((commit_hash, commit_date))
}

/// Run fixed `/usr/bin/git` into a tar stream, hashing it to the snapshot
/// digest and staging the tarball. Bare repo via explicit `--git-dir`; global
/// config redirected to /dev/null; no /tmp HOME git config; stderr drained;
/// exit reaped.
fn git_archive_to_tar(repo: &Path, commit: &str, out_path: &Path) -> Result<String, String> {
    let mut file =
        File::create(out_path).map_err(|e| format!("cannot create archive {out_path:?}: {e}"))?;
    let staging = out_path
        .parent()
        .ok_or_else(|| format!("archive has no parent: {out_path:?}"))?
        .to_path_buf();
    let mut child = Command::new(GIT_BIN)
        .env_clear()
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", &staging)
        .current_dir(repo)
        .arg("--git-dir")
        .arg(repo)
        .arg("archive")
        .arg(commit)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run {GIT_BIN} archive for {commit}: {e}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{GIT_BIN} archive for {commit} had no stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("{GIT_BIN} archive for {commit} had no stderr"))?;
    let stderr_handle = thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = stdout
            .read(&mut buf)
            .map_err(|e| format!("failed to read {GIT_BIN} archive for {commit}: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])
            .map_err(|e| format!("failed to write archive {out_path:?}: {e}"))?;
    }
    let stderr_str = stderr_handle.join().unwrap();
    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for {GIT_BIN} archive for {commit}: {e}"))?;
    if !status.success() {
        return Err(format!(
            "git archive {commit} failed with exit {}: {stderr_str}",
            status.code().unwrap_or(-1)
        ));
    }
    let result = hasher.finalize();
    Ok(result.iter().map(|b| format!("{b:02x}")).collect())
}

/// Extract the tarball inside bubblewrap: isolated user/network/pid namespace,
/// no caps, die-with-parent, /usr read-only with the distro loader alias,
/// /dev and /proc private, only the staging output dir writable. Then verify
/// the tree excludes link/special types.
fn extract_snapshot_sandbox(tar_path: &Path, out_dir: &Path) -> Result<(), String> {
    // Create the extract output before binding it into the sandbox: private,
    // exact mode 0700, fail if already present.
    DirBuilder::new()
        .mode(0o700)
        .create(out_dir)
        .map_err(|e| format!("cannot create extract output {out_dir:?}: {e}"))?;
    fs::set_permissions(out_dir, Permissions::from_mode(0o700))
        .map_err(|e| format!("cannot set extract output mode {out_dir:?}: {e}"))?;

    let mut args = bwrap_base_args();
    args.push("--ro-bind".to_string());
    args.push(tar_path.to_string_lossy().to_string());
    args.push(SNAPSHOT_DEST.to_string());
    args.push("--bind".to_string());
    args.push(out_dir.to_string_lossy().to_string());
    args.push(WORKSPACE_DEST.to_string());
    args.push("--setenv".to_string());
    args.push("PATH".to_string());
    args.push("/usr/bin".to_string());
    args.push("--setenv".to_string());
    args.push("HOME".to_string());
    args.push("/tmp".to_string());
    args.push("--chdir".to_string());
    args.push(WORKSPACE_DEST.to_string());
    args.push("--".to_string());
    args.push(TAR_BIN.to_string());
    args.push("-xf".to_string());
    args.push(SNAPSHOT_DEST.to_string());
    args.push("-C".to_string());
    args.push(WORKSPACE_DEST.to_string());
    run_bwrap_void(&args)?;

    // Reject symlinks and special file types after extraction: iterative
    // traversal, symlinks are never followed.
    verify_tree(out_dir)?;
    Ok(())
}

/// Reject any extracted entry that is not a regular file or directory.
/// Iterative stack traversal: no recursion, symlinks are never followed.
/// Hard links are rejected: regular files must have exactly one link.
fn verify_tree(root: &Path) -> Result<(), String> {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            fs::read_dir(&dir).map_err(|e| format!("cannot read tree dir {dir:?}: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot read tree entry in {dir:?}: {e}"))?;
            let path = entry.path();
            let ft = entry
                .file_type()
                .map_err(|e| format!("cannot stat {path:?}: {e}"))?;
            // Guard clauses: no standalone else chains.
            if ft.is_symlink() {
                return Err(format!("snapshot tree contains symlink: {path:?}"));
            }
            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            if !ft.is_file() {
                return Err(format!(
                    "snapshot tree contains non-regular entry: {path:?}"
                ));
            }
            // Reject hard links: a regular file must have nlink == 1.
            // symlink_metadata never follows symlinks (already rejected above).
            let md =
                fs::symlink_metadata(&path).map_err(|e| format!("cannot stat {path:?}: {e}"))?;
            if md.nlink() != 1 {
                return Err(format!(
                    "snapshot tree contains hard-linked file: {path:?} (nlink {})",
                    md.nlink()
                ));
            }
        }
    }
    Ok(())
}

/// Build the fixed minimal bwrap mount set. No evidence or home mounts.
fn bwrap_base_args() -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    args.push("--unshare-user".to_string());
    args.push("--unshare-ipc".to_string());
    args.push("--unshare-pid".to_string());
    args.push("--unshare-net".to_string());
    args.push("--disable-userns".to_string());
    args.push("--die-with-parent".to_string());
    // New session: detach from the controlling TTY so the sandbox cannot
    // receive job-control signals from the parent terminal.
    args.push("--new-session".to_string());
    args.push("--cap-drop".to_string());
    args.push("ALL".to_string());
    args.push("--dev".to_string());
    args.push("/dev".to_string());
    args.push("--proc".to_string());
    args.push("/proc".to_string());
    args.push("--tmpfs".to_string());
    args.push("/tmp".to_string());
    args.push("--ro-bind".to_string());
    args.push("/usr".to_string());
    args.push("/usr".to_string());
    // Distro dynamic loader aliases, read-only, where applicable: the loader
    // lives at /lib64/ld-linux (and /lib/ld-linux on some distros) which
    // resolves into /usr/lib on this host.
    for (loader_dir, alias) in [("/usr/lib", "/lib64"), ("/usr/lib", "/lib")] {
        if Path::new(alias).exists() {
            args.push("--ro-bind".to_string());
            args.push(loader_dir.to_string());
            args.push(alias.to_string());
        }
    }
    args
}

fn run_bwrap_void(args: &[String]) -> Result<(), String> {
    let mut cmd = Command::new(BWRAP_BIN);
    cmd.env_clear()
        .env("PATH", "/usr/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for a in args {
        cmd.arg(a);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("failed to launch bwrap: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("bwrap failed: {stderr}"));
    }
    Ok(())
}

fn run_gates(
    snapshot: &Path,
    toolchain: &Path,
    cargo: &Path,
    asep: &Path,
    approved: &SnapshotIdentity,
) -> Result<AcceptanceEvidence, String> {
    let mut gates: Vec<GateEvidence> = Vec::with_capacity(MANDATORY_GATES.len());
    for gate in MANDATORY_GATES {
        let evidence = match gate {
            AcceptanceGate::Formal => run_asep_gate(
                asep,
                snapshot,
                toolchain,
                gate,
                &["ci", "--gate", "0", "--json"],
            )?,
            AcceptanceGate::Structural => {
                // Trusted in-process structural inspection: no behavioral JSON,
                // no candidate execution. Record the internal gate label as the
                // command; any failure rejects before acceptance is evaluated.
                crate::ci::check_acceptance_structure(snapshot)
                    .map_err(|e| format!("structural gate failed: {e}"))?;
                GateEvidence {
                    gate,
                    command: vec![format!("{gate:?}")],
                    exit: Some(0),
                    skipped: false,
                }
            }
            AcceptanceGate::TypeCheck => run_rust_gate(
                cargo,
                snapshot,
                toolchain,
                gate,
                &[
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--offline",
                    "--locked",
                ],
            )?,
            AcceptanceGate::Clippy => run_rust_gate(
                cargo,
                snapshot,
                toolchain,
                gate,
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--offline",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?,
            AcceptanceGate::Tests => run_rust_gate(
                cargo,
                snapshot,
                toolchain,
                gate,
                &[
                    "test",
                    "--workspace",
                    "--all-targets",
                    "--offline",
                    "--locked",
                ],
            )?,
            AcceptanceGate::Format => run_rust_gate(
                cargo,
                snapshot,
                toolchain,
                gate,
                &["fmt", "--all", "--check"],
            )?,
            AcceptanceGate::BenchPreflight => run_rust_gate(
                cargo,
                snapshot,
                toolchain,
                gate,
                &["bench", "--workspace", "--no-run", "--offline", "--locked"],
            )?,
        };
        gates.push(evidence);
    }
    Ok(AcceptanceEvidence {
        identity: approved.clone(),
        gates,
    })
}

fn gate_id(gate: AcceptanceGate) -> u8 {
    match gate {
        AcceptanceGate::Formal => 0,
        AcceptanceGate::Structural => 1,
        AcceptanceGate::TypeCheck => 2,
        AcceptanceGate::Clippy => 3,
        AcceptanceGate::Tests => 4,
        AcceptanceGate::Format => 5,
        AcceptanceGate::BenchPreflight => 6,
    }
}

/// Run a trusted `asep ci --gate N --json` gate (Formal). Reject when the
/// observed exit status is nonzero OR the JSON report does not pass (both
/// required). Fail closed on any error.
fn run_asep_gate(
    asep: &Path,
    snapshot: &Path,
    toolchain: &Path,
    gate: AcceptanceGate,
    args: &[&str],
) -> Result<GateEvidence, String> {
    let gate_id = gate_id(gate);
    // Record the executable actually invoked inside the sandbox.
    let command: Vec<String> = std::iter::once(ASEP_DEST.to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    let out = run_in_sandbox(toolchain, snapshot, asep, ASEP_DEST, ASEP_DEST, args)?;
    let status_ok = out.status == 0;
    let report_ok = serde_json::from_str::<CiReport>(&out.stdout)
        .map(|r| r.passes_required(&[gate_id]))
        .unwrap_or(false);
    #[allow(clippy::match_bool)]
    match (status_ok, report_ok) {
        (true, true) => {}
        _ => {
            return Err(format!(
                "gate {gate_id} failed: exit={} report_ok={report_ok} stderr={}",
                out.status, out.stderr
            ));
        }
    }
    Ok(GateEvidence {
        gate,
        command,
        exit: Some(out.status),
        skipped: false,
    })
}

/// Run a cargo gate (check/clippy/test/fmt/bench). Fail on non-zero exit or
/// on an unsupported/missing gate helper.
fn run_rust_gate(
    cargo: &Path,
    snapshot: &Path,
    toolchain: &Path,
    gate: AcceptanceGate,
    args: &[&str],
) -> Result<GateEvidence, String> {
    let command: Vec<String> = std::iter::once(CARGO_DEST.to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    let out = run_in_sandbox(toolchain, snapshot, cargo, CARGO_DEST, CARGO_DEST, args)?;
    if out.status != 0 {
        return Err(format!(
            "gate {:?} failed: exit={} stderr={}",
            gate, out.status, out.stderr
        ));
    }
    Ok(GateEvidence {
        gate,
        command,
        exit: Some(out.status),
        skipped: false,
    })
}

/// Run one command inside the isolated sandbox: trusted toolchain pinned at
/// /toolchain, trusted executable bound at `bind_dest` and invoked as
/// `invoke` (the sandbox path, never the host exe path), bwrap env-clear,
/// unshare-all + die-with-parent, read-only snapshot, private writable
/// scratch, explicit PATH, absolute trusted executables, no uncontrolled
/// shell, null stdin.
fn run_in_sandbox(
    toolchain: &Path,
    snapshot: &Path,
    exe: &Path,
    bind_dest: &str,
    invoke: &str,
    args: &[&str],
) -> Result<IsolatedOutput, String> {
    let mut args_vec = bwrap_base_args();
    args_vec.push("--ro-bind".to_string());
    args_vec.push(toolchain.to_string_lossy().to_string());
    args_vec.push(TOOLCHAIN_DEST.to_string());
    args_vec.push("--ro-bind".to_string());
    args_vec.push(exe.to_string_lossy().to_string());
    args_vec.push(bind_dest.to_string());
    args_vec.push("--ro-bind".to_string());
    args_vec.push(snapshot.to_string_lossy().to_string());
    args_vec.push(WORKSPACE_DEST.to_string());
    args_vec.push("--tmpfs".to_string());
    args_vec.push(SCRATCH_DEST.to_string());
    args_vec.push("--setenv".to_string());
    args_vec.push("PATH".to_string());
    args_vec.push(format!("{TOOLCHAIN_DEST}/bin:/usr/bin"));
    args_vec.push("--setenv".to_string());
    args_vec.push("HOME".to_string());
    args_vec.push("/tmp".to_string());
    args_vec.push("--setenv".to_string());
    args_vec.push("CARGO_HOME".to_string());
    args_vec.push(format!("{SCRATCH_DEST}/cargo-home"));
    args_vec.push("--setenv".to_string());
    args_vec.push("CARGO_TARGET_DIR".to_string());
    args_vec.push(format!("{SCRATCH_DEST}/target"));
    args_vec.push("--setenv".to_string());
    args_vec.push("RUSTUP_HOME".to_string());
    args_vec.push(format!("{SCRATCH_DEST}/rustup-home"));
    args_vec.push("--setenv".to_string());
    args_vec.push("CARGO_NET_OFFLINE".to_string());
    args_vec.push("1".to_string());
    args_vec.push("--chdir".to_string());
    args_vec.push(WORKSPACE_DEST.to_string());
    args_vec.push("--".to_string());
    args_vec.push(invoke.to_string());
    for a in args {
        args_vec.push(a.to_string());
    }
    run_bwrap_capture(&args_vec)
}

fn run_bwrap_capture(args: &[String]) -> Result<IsolatedOutput, String> {
    let mut cmd = Command::new(BWRAP_BIN);
    cmd.env_clear()
        .env("PATH", "/usr/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for a in args {
        cmd.arg(a);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("failed to launch bwrap: {e}"))?;
    let status = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    Ok(IsolatedOutput {
        status,
        stdout,
        stderr,
    })
}

fn write_evidence(path: &Path, evidence: &AcceptanceEvidence) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("cannot create evidence file {path:?} (create_new failed): {e}"))?;
    let json = serde_json::to_string_pretty(evidence)
        .map_err(|e| format!("cannot serialize evidence: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("cannot write evidence {path:?}: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("cannot sync evidence {path:?}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_non_full_commit_sha() {
        assert!(validate_commit_sha("").is_err());
        assert!(validate_commit_sha("a1b2").is_err());
        assert!(validate_commit_sha("A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2").is_err());
        assert!(validate_commit_sha("g1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2").is_err());
        assert!(validate_commit_sha("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2").is_ok());
    }

    #[test]
    fn reject_bad_digest() {
        assert!(validate_digest("a1b2").is_err());
        assert!(validate_digest("A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2").is_err());
        assert!(validate_digest(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
        )
        .is_ok());
    }

    const RUSTC_VV: &str = "rustc 1.102.0-nightly (5f4effd57 2026-11-13)\nbinary: rustc\ncommit-hash: 5f4effd579500175af844241eb179e756190f1e7\ncommit-date: 2026-11-13\nhost: x86_64-unknown-linux-gnu\n";

    #[test]
    fn parse_rustc_vv_matches_real_pin() {
        let (hash, date) = parse_rustc_vv(RUSTC_VV).expect("real rustc -vV output parses");
        assert_eq!(hash, "5f4effd579500175af844241eb179e756190f1e7");
        assert_eq!(date, "2026-11-13");
    }

    #[test]
    fn parse_rustc_vv_extracts_wrong_commit_hash() {
        let bad = RUSTC_VV.replace(
            "5f4effd579500175af844241eb179e756190f1e7",
            "0000000000000000000000000000000000000000",
        );
        let (hash, date) = parse_rustc_vv(&bad).expect("wrong-hash output still parses");
        assert_eq!(hash, "0000000000000000000000000000000000000000");
        assert_eq!(date, "2026-11-13");
    }

    /// Stub `rustc` whose `-vV` prints a controlled commit-hash/commit-date.
    fn stub_rustc(parent: &Path, hash: &str, date: &str) -> Result<PathBuf, String> {
        let dir = parent.join(TOOLCHAIN_DIR_NAME);
        fs::create_dir_all(dir.join("bin"))
            .map_err(|e| format!("cannot create toolchain stub dir: {e}"))?;
        let rustc = dir.join("bin").join("rustc");
        File::create(&rustc).map_err(|e| format!("cannot create rustc stub: {e}"))?;
        fs::set_permissions(&rustc, Permissions::from_mode(0o755))
            .map_err(|e| format!("cannot set rustc stub mode: {e}"))?;
        let script = format!(
            "#!/bin/sh\necho 'rustc 1.0 (abc {date})'\necho 'binary: rustc'\necho 'commit-hash: {hash}'\necho 'commit-date: {date}'\n"
        );
        std::fs::write(&rustc, script).map_err(|e| format!("cannot write rustc stub: {e}"))?;
        Ok(dir)
    }

    #[test]
    fn verify_toolchain_identity_accepts_exact_hash_and_date() {
        let parent = tempfile::tempdir().expect("tempdir creates");
        let toolchain = stub_rustc(
            parent.path(),
            "5f4effd579500175af844241eb179e756190f1e7",
            "2026-11-13",
        )
        .expect("stub creates");
        assert!(verify_toolchain_identity(&toolchain).is_ok());
    }

    #[test]
    fn verify_toolchain_identity_rejects_wrong_hash() {
        let parent = tempfile::tempdir().expect("tempdir creates");
        let toolchain = stub_rustc(
            parent.path(),
            "0000000000000000000000000000000000000000",
            "2026-11-13",
        )
        .expect("stub creates");
        assert!(
            verify_toolchain_identity(&toolchain).is_err(),
            "wrong commit-hash must be rejected"
        );
    }

    #[test]
    fn verify_toolchain_identity_rejects_wrong_date() {
        let parent = tempfile::tempdir().expect("tempdir creates");
        let toolchain = stub_rustc(
            parent.path(),
            "5f4effd579500175af844241eb179e756190f1e7",
            "2026-11-14",
        )
        .expect("stub creates");
        assert!(
            verify_toolchain_identity(&toolchain).is_err(),
            "wrong commit-date must be rejected"
        );
    }

    #[test]
    fn parse_rustc_vv_rejects_missing_lines() {
        assert!(
            parse_rustc_vv("rustc 1.0 (abc 2026-11-13)\nbinary: rustc\n").is_err(),
            "missing commit-hash and commit-date must be rejected"
        );
        assert!(
            parse_rustc_vv("rustc 1.0 (abc 2026-11-13)\nbinary: rustc\ncommit-date: 2026-11-13\n")
                .is_err(),
            "missing commit-hash must be rejected"
        );
    }

    #[test]
    fn staging_dir_is_private_mode_0700() {
        let parent = tempfile::tempdir().expect("dev tempfile parent");
        let commit = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let staging = StagingDir::new(parent.path(), commit).expect("staging dir creates");
        let md = fs::metadata(staging.path()).expect("staging dir metadata");
        assert_eq!(
            md.mode() & 0o077,
            0o000,
            "staging dir must be 0700 or stricter"
        );
        let child = staging.path().to_path_buf();
        drop(staging);
        assert!(!child.exists(), "staging child removed on drop");
        assert!(parent.path().exists(), "dev tempfile parent retained");
    }

    #[test]
    fn verify_tree_accepts_regular_tree_rejects_links_and_specials() {
        let root = std::env::temp_dir().join(format!("verify-tree-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).expect("temp root");
        fs::create_dir(root.join("sub")).expect("subdir");
        File::create(root.join("a.txt")).expect("file");
        File::create(root.join("sub").join("b.txt")).expect("nested file");
        assert!(verify_tree(&root).is_ok());

        std::os::unix::fs::symlink(root.join("a.txt"), root.join("link.txt")).expect("symlink");
        assert!(verify_tree(&root).is_err(), "symlink must be rejected");

        fs::hard_link(root.join("a.txt"), root.join("hard.txt")).expect("hard link");
        assert!(verify_tree(&root).is_err(), "hard link must be rejected");

        let _ = fs::remove_dir_all(&root);
    }
}
