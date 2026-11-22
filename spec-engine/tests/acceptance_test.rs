//! Acceptance CLI contract tests for `asep accept`.
//!
//! Guard-clause policy: clap-level rejections (short ref, upper-case SHA,
//! unknown skip/trust override flags, missing commit argument) must fail
//! closed before the trusted runner is invoked. A well-formed full SHA
//! without trusted setup must exit nonzero and never emit ACCEPTED evidence.
//! The trusted-setup success path is intentionally not covered here: positive
//! runner proof is a separate isolated unit, and acceptance must never be
//! faked from arbitrary JSON.

use asep::acceptance::{AcceptanceEvidence, MANDATORY_GATES, SCHEMA_VERSION};
use assert_cmd::Command;
use predicates::prelude::*;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::fs::Permissions;
use std::io::Read;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::thread;

fn asep_bin() -> Command {
    Command::cargo_bin("asep").expect("asep binary")
}

/// A valid full lowercase Git object ID (40 lowercase hex characters).
const FULL_SHA: &str = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

/// True when the effective UID read from /proc/self/status is 0 (root).
fn is_running_as_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|uid| uid.trim().parse::<u32>().ok())
                })
        })
        .map_or(false, |effective_uid| effective_uid == 0)
}

#[test]
fn test_accept_rejects_short_ref() {
    let mut cmd = asep_bin();
    cmd.arg("accept").arg("a1b2c3d4");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid commit id"));
}

#[test]
fn test_accept_rejects_uppercase_sha() {
    let mut cmd = asep_bin();
    cmd.arg("accept")
        .arg("A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid commit id"));
}

#[test]
fn test_accept_rejects_skip_trust_override_flags() {
    for flag in ["--skip", "--trust"] {
        let mut cmd = asep_bin();
        cmd.arg("accept").arg(FULL_SHA).arg(flag);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_accept_requires_commit_argument() {
    let mut cmd = asep_bin();
    cmd.arg("accept");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// @test:FR-ACCEPT-01
#[test]
fn test_accept_valid_full_sha_without_trusted_setup_is_not_accepted() {
    // Only exercise trusted-runner rejection under non-root. Never change
    // /etc or probe trusted configuration when running as root.
    if is_running_as_root() {
        eprintln!(
            "skip: running as root (effective UID 0); refusing to probe trusted configuration"
        );
        return;
    }

    let tmp = tempfile::tempdir().expect("temp dir for untrusted cwd");
    let mut cmd = asep_bin();
    cmd.current_dir(tmp.path());
    cmd.arg("accept").arg(FULL_SHA);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("NOT ACCEPTED"))
        .stdout(predicate::str::contains("ACCEPTED").not());
}

// Exact command (opt-in, ignored; run from the workspace root):
//   CARGO_TARGET_DIR=/home/da578/Documents/Projects/Rust/asep-pilot.yNBxac4c/target \
//   cargo test -p asep --offline --locked --test acceptance_test test_accept_offline_protected_fixture -- --ignored --exact --nocapture
//
// Trusted-setup success path inside an ephemeral bwrap user namespace. The
// outer namespace uses the lead-confirmed flags
//   --unshare-user --uid 0 --gid 0 --cap-add CAP_SETFCAP --unshare-pid
//   --unshare-net --ro-bind /usr /usr --symlink usr/lib /lib
//   --symlink usr/lib /lib64 --proc /proc --dev /dev --tmpfs /tmp
// so the outer CAP_SETFCAP capability applies only inside the test namespace:
// no sudo, no host /etc. Inside, the provisioned acceptance runner spawns the
// production inner bwrap (same flags as production) for extraction and gates.
//
// The fixture is a real, tiny, dependency-free Git/Rust snapshot: one commit,
// valid trace tags (@req/@impl/@test with @rel:satisfies/@rel:verifies), a real
// passing test, rustfmt- and clippy-clean. The toolchain dir basename is
// exactly `nightly-2026-11-14`; the installed rustup nightly reports
// commit-date 2026-11-13 and commit-hash
// 5f4effd579500175af844241eb179e756190f1e7, matching the runner pin.
// ASEP is the compiled binary at env!("CARGO_BIN_EXE_asep"). /etc is a private
// fake (tmpfs + bind-mounted policy) and evidence is isolated under the staging
// dir. Targets: smoke exact success (seven successful gates, evidence identity
// confirmed), then a digest-mismatch policy update that fails before any gate
// and writes no evidence, then a rewritten fixture committed as a new SHA
// whose policy matches the new snapshot but whose Tests gate fails, which must
// be NOT ACCEPTED with a Tests-gate failure diagnostic and no evidence file.
#[cfg(target_os = "linux")]
#[ignore]
#[test]
fn test_accept_offline_protected_fixture() -> Result<(), String> {
    if !Path::new("/usr/bin/bwrap").exists() {
        return Err("/usr/bin/bwrap is required for the opt-in offline fixture".to_string());
    }
    let home = env::var("HOME").unwrap_or_default();
    let rustup_home = env::var("RUSTUP_HOME").unwrap_or_else(|_| format!("{home}/.rustup"));
    let toolchain_src = PathBuf::from(&rustup_home)
        .join("toolchains")
        .join(format!("{TOOLCHAIN_DIR_NAME}-{TOOLCHAIN_TRIPLE}"));
    if !toolchain_src.join("bin").join("rustc").exists() {
        return Err(format!(
            "pinned rustup toolchain {TOOLCHAIN_DIR_NAME} is not installed"
        ));
    }

    let staging = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let repo = staging.path().join("repo");
    let evidence_dir = staging.path().join("evidence");
    fs::create_dir_all(repo.join("src")).map_err(|e| format!("mkdir src: {e}"))?;
    fs::create_dir_all(repo.join("tests")).map_err(|e| format!("mkdir tests: {e}"))?;
    fs::create_dir_all(&evidence_dir).map_err(|e| format!("mkdir evidence: {e}"))?;
    fs::set_permissions(&evidence_dir, Permissions::from_mode(0o700))
        .map_err(|e| format!("evidence mode: {e}"))?;
    fs::create_dir_all(staging.path().join("etc")).map_err(|e| format!("mkdir etc: {e}"))?;

    // --- Tiny dependency-free Git/Rust fixture -----------------------------
    fs::write(repo.join("Cargo.toml"), FIXTURE_CARGO_TOML)
        .map_err(|e| format!("write Cargo.toml: {e}"))?;
    fs::write(repo.join("Cargo.lock"), FIXTURE_CARGO_LOCK)
        .map_err(|e| format!("write Cargo.lock: {e}"))?;
    fs::write(repo.join("src").join("lib.rs"), FIXTURE_LIB_RS)
        .map_err(|e| format!("write lib.rs: {e}"))?;
    fs::write(repo.join("tests").join("hello_test.rs"), FIXTURE_TEST_RS)
        .map_err(|e| format!("write hello_test.rs: {e}"))?;
    fs::write(repo.join("spec.md"), FIXTURE_SPEC_MD).map_err(|e| format!("write spec.md: {e}"))?;

    git(&["init"], &repo)?;
    git(
        &[
            "-c",
            "user.name=asep-offline-test",
            "-c",
            "user.email=offline@test.invalid",
            "add",
            "-A",
        ],
        &repo,
    )?;
    git(
        &[
            "-c",
            "user.name=asep-offline-test",
            "-c",
            "user.email=offline@test.invalid",
            "commit",
            "-m",
            "offline acceptance fixture",
        ],
        &repo,
    )?;
    let sha = git(&["rev-parse", "HEAD"], &repo)?;
    if sha.len() != 40 {
        return Err(format!("unexpected git SHA: {sha}"));
    }
    let digest = git_archive_digest(&repo, &sha)?;

    // --- Pinned toolchain: ro-bound in full at /toolchains/nightly-2026-11-14 ---
    // No host-side copy of individual binaries; the runner invokes cargo/rustc
    // inside the sandbox via the read-only toolchain bind below.

    // --- Private fake /etc acceptance policy --------------------------------
    let policy = serde_json::json!({
        "approved_commit": sha,
        "snapshot_sha256": digest,
        "repo": format!("{PROV_INSIDE}/repo/.git"),
        "toolchain": format!("/toolchains/{TOOLCHAIN_DIR_NAME}"),
        "evidence_dir": format!("{PROV_INSIDE}/evidence"),
    })
    .to_string();
    let policy_path = staging.path().join("policy.json");
    fs::write(&policy_path, &policy).map_err(|e| format!("write policy: {e}"))?;
    let policy_bytes = fs::read(&policy_path).map_err(|e| format!("read policy: {e}"))?;

    let evidence_path = evidence_dir.join(format!("acceptance-{sha}.json"));
    let empty_dir = staging.path().join("etc");

    // --- RUN 1: smoke exact success ----------------------------------------
    let out = bwrap_accept(
        staging.path(),
        &toolchain_src,
        &evidence_dir,
        env!("CARGO_BIN_EXE_asep"),
        &sha,
        &policy_path,
        &empty_dir,
    )?;
    if !out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "smoke accept failed: exit={} stdout={stdout} stderr={stderr}",
            out.status.code().unwrap_or(-1)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ACCEPTED:"),
        "accept must print ACCEPTED, got: {stdout}"
    );
    assert!(
        stdout.contains(&format!("acceptance-{sha}.json")),
        "accept must print evidence path, got: {stdout}"
    );

    // --- Confirm evidence identity and seven successful gates --------------
    let evidence: AcceptanceEvidence = serde_json::from_slice(
        &fs::read(&evidence_path).map_err(|e| format!("read evidence: {e}"))?,
    )
    .map_err(|e| format!("parse evidence: {e}"))?;
    assert_eq!(evidence.identity.commit_sha, sha);
    assert_eq!(evidence.identity.snapshot_sha256, digest);
    assert_eq!(evidence.identity.policy_sha256, sha256_hex(&policy_bytes));
    assert!(
        !evidence.identity.runner_id.is_empty(),
        "runner_id must be recorded"
    );
    assert_eq!(
        evidence.identity.toolchain_id,
        format!("/toolchains/{TOOLCHAIN_DIR_NAME}")
    );
    assert_eq!(evidence.identity.schema_version, SCHEMA_VERSION);
    assert_eq!(
        evidence.gates.len(),
        MANDATORY_GATES.len(),
        "expected seven mandatory gates, got {}",
        evidence.gates.len()
    );
    for (gate_evidence, expected_gate) in evidence.gates.iter().zip(MANDATORY_GATES.iter()) {
        assert_eq!(gate_evidence.gate, *expected_gate, "gate order mismatch");
        assert_eq!(
            gate_evidence.exit,
            Some(0),
            "gate {:?} must exit 0, got {:?}, stderr captured above",
            gate_evidence.gate,
            gate_evidence.exit
        );
        assert!(
            !gate_evidence.skipped,
            "gate {:?} must not be skipped",
            gate_evidence.gate
        );
        assert!(
            !gate_evidence.command.is_empty(),
            "gate {:?} must record a command",
            gate_evidence.gate
        );
    }

    // --- RUN 2: digest mismatch (policy update) -> fail closed, no evidence
    let bad_policy = policy.replace(&digest, &"2".repeat(64));
    fs::write(staging.path().join("policy-bad.json"), &bad_policy)
        .map_err(|e| format!("write bad policy: {e}"))?;
    let _ = fs::remove_file(&evidence_path);
    let out2 = bwrap_accept(
        staging.path(),
        &toolchain_src,
        &evidence_dir,
        env!("CARGO_BIN_EXE_asep"),
        &sha,
        &staging.path().join("policy-bad.json"),
        &empty_dir,
    )?;
    assert!(
        !out2.status.success(),
        "digest-mismatch accept must fail: {:?}",
        out2.status
    );
    let stderr2 = String::from_utf8_lossy(&out2.stderr);
    assert!(
        stderr2.contains("digest mismatch"),
        "expected digest-mismatch diagnostic, got: {stderr2}"
    );
    assert!(
        !evidence_path.exists(),
        "no evidence may be written on digest mismatch"
    );
    eprintln!("digest-mismatch diagnostics (usable evidence):\n{stderr2}");

    // --- RUN 3: rewritten failing fixture (new commit) -> Tests gate failed, no evidence
    fs::write(
        repo.join("tests").join("hello_test.rs"),
        FIXTURE_TEST_RS_FAIL,
    )
    .map_err(|e| format!("write failing hello_test.rs: {e}"))?;
    git(
        &[
            "-c",
            "user.name=asep-offline-test",
            "-c",
            "user.email=offline@test.invalid",
            "add",
            "-A",
        ],
        &repo,
    )?;
    git(
        &[
            "-c",
            "user.name=asep-offline-test",
            "-c",
            "user.email=offline@test.invalid",
            "commit",
            "-m",
            "offline acceptance fixture: failing test",
        ],
        &repo,
    )?;
    let sha_fail = git(&["rev-parse", "HEAD"], &repo)?;
    if sha_fail.len() != 40 {
        return Err(format!("unexpected git SHA: {sha_fail}"));
    }
    let digest_fail = git_archive_digest(&repo, &sha_fail)?;

    let fail_policy = serde_json::json!({
        "approved_commit": sha_fail,
        "snapshot_sha256": digest_fail,
        "repo": format!("{PROV_INSIDE}/repo/.git"),
        "toolchain": format!("/toolchains/{TOOLCHAIN_DIR_NAME}"),
        "evidence_dir": format!("{PROV_INSIDE}/evidence"),
    })
    .to_string();
    let fail_policy_path = staging.path().join("policy-fail.json");
    fs::write(&fail_policy_path, &fail_policy).map_err(|e| format!("write fail policy: {e}"))?;

    let evidence_fail_path = evidence_dir.join(format!("acceptance-{sha_fail}.json"));
    let _ = fs::remove_file(&evidence_fail_path);
    let out3 = bwrap_accept(
        staging.path(),
        &toolchain_src,
        &evidence_dir,
        env!("CARGO_BIN_EXE_asep"),
        &sha_fail,
        &fail_policy_path,
        &empty_dir,
    )?;
    assert!(
        !out3.status.success(),
        "failing-fixture accept must fail: {:?}",
        out3.status
    );
    let stderr3 = String::from_utf8_lossy(&out3.stderr);
    assert!(
        stderr3.contains("NOT ACCEPTED"),
        "expected NOT ACCEPTED diagnostic, got: {stderr3}"
    );
    assert!(
        stderr3.contains("gate Tests failed"),
        "expected Tests gate failure diagnostic, got: {stderr3}"
    );
    assert!(
        !evidence_fail_path.exists(),
        "no evidence may be written for failing fixture commit"
    );
    eprintln!("failing-fixture diagnostics (usable evidence):\n{stderr3}");

    Ok(())
}

/// Run `git` on the host (fixture provisioning only; the trusted runner runs
/// git inside its own sandbox).
fn git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = StdCommand::new(GIT_BIN)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|e| format!("git spawn {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Mirror the runner's `git_archive_to_tar` byte stream so the policy digest
/// matches the digest computed inside the sandbox.
fn git_archive_digest(repo: &Path, commit: &str) -> Result<String, String> {
    let mut child = StdCommand::new(GIT_BIN)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(repo)
        .arg("archive")
        .arg(commit)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git archive spawn: {e}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "git archive had no stdout".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "git archive had no stderr".to_string())?;
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
            .map_err(|e| format!("read git archive: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let _stderr = stderr_handle.join().unwrap();
    let status = child.wait().map_err(|e| format!("wait git archive: {e}"))?;
    if !status.success() {
        return Err(format!(
            "git archive {commit} failed: exit {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(sha256_finalize(hasher))
}

fn sha256_finalize(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Spin up the ephemeral outer namespace (lead-confirmed flags; capability
// scoped inside only) and run `asep accept <commit>` with the compiled ASEP
/// binary bound into the namespace. The policy bind provides the private fake
// /etc; the staging bind provides repo (read-only). The pinned toolchain is
/// ro-bound in full at /toolchains/nightly-2026-11-14, and the evidence dir
/// is bind-mounted writable at /tmp/prov/evidence, separate from /prov RO.
fn bwrap_accept(
    staging: &Path,
    toolchain_src: &Path,
    evidence_dir: &Path,
    asep_exe: &str,
    commit: &str,
    policy: &Path,
    empty_dir: &Path,
) -> Result<std::process::Output, String> {
    let mut args: Vec<String> = Vec::new();
    let push = |args: &mut Vec<String>, a: &str| args.push(a.to_string());
    push(&mut args, "--unshare-user");
    push(&mut args, "--uid");
    push(&mut args, "0");
    push(&mut args, "--gid");
    push(&mut args, "0");
    push(&mut args, "--cap-add");
    push(&mut args, "CAP_SETFCAP");
    push(&mut args, "--unshare-pid");
    push(&mut args, "--unshare-net");
    push(&mut args, "--ro-bind");
    push(&mut args, "/usr");
    push(&mut args, "/usr");
    push(&mut args, "--symlink");
    push(&mut args, "usr/lib");
    push(&mut args, "/lib");
    push(&mut args, "--symlink");
    push(&mut args, "usr/lib");
    push(&mut args, "/lib64");
    push(&mut args, "--proc");
    push(&mut args, "/proc");
    push(&mut args, "--dev");
    push(&mut args, "/dev");
    push(&mut args, "--tmpfs");
    push(&mut args, "/tmp");
    push(&mut args, "--bind");
    push(&mut args, &empty_dir.to_string_lossy());
    push(&mut args, "/etc/asep");
    push(&mut args, "--bind");
    push(&mut args, &policy.to_string_lossy());
    push(&mut args, POLICY_PATH_INSIDE);
    push(&mut args, "--ro-bind");
    push(&mut args, &staging.to_string_lossy());
    push(&mut args, PROV_INSIDE);
    // Full pinned toolchain, read-only, at /toolchains/nightly-2026-11-14.
    push(&mut args, "--ro-bind");
    push(&mut args, &toolchain_src.to_string_lossy());
    push(&mut args, &format!("/toolchains/{TOOLCHAIN_DIR_NAME}"));
    // Writable evidence dir, separate from the read-only /prov bind.
    push(&mut args, "--bind");
    push(&mut args, &evidence_dir.to_string_lossy());
    push(&mut args, &format!("{PROV_INSIDE}/evidence"));
    push(&mut args, "--ro-bind");
    push(&mut args, asep_exe);
    push(&mut args, ASEP_BIN_INSIDE);
    push(&mut args, "--");
    push(&mut args, ASEP_BIN_INSIDE);
    push(&mut args, "accept");
    push(&mut args, commit);
    StdCommand::new(BWRAP_BIN)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("bwrap spawn: {e}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const FIXTURE_CARGO_TOML: &str = r#"[package]
name = "hello-fixture"
version = "0.1.0"
edition = "2021"

[workspace]
"#;

const FIXTURE_CARGO_LOCK: &str = r#"# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "hello-fixture"
version = "0.1.0"
"#;

const FIXTURE_LIB_RS: &str = r#"//! Integer helpers for the acceptance fixture.
// @impl:FR-HELLO-01
// @rel:satisfies:FR-HELLO-01->FR-HELLO-01

/// Compute the sum of two integers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

const FIXTURE_TEST_RS: &str = r#"// @test:FR-HELLO-01
// @rel:verifies:FR-HELLO-01->FR-HELLO-01 "covers the requirement"

#[test]
fn add_ok() {
    assert_eq!(hello_fixture::add(2, 3), 5);
}
"#;

// Same fixture with a deliberately failing assertion so the Tests gate fails.
const FIXTURE_TEST_RS_FAIL: &str = r#"// @test:FR-HELLO-01
// @rel:verifies:FR-HELLO-01->FR-HELLO-01 "covers the requirement"

#[test]
fn add_ok() {
    assert_eq!(hello_fixture::add(2, 3), 6);
}
"#;

const FIXTURE_SPEC_MD: &str = "# Hello acceptance fixture\n\n@req:FR-HELLO-01:MUST Integer addition requirement for the offline acceptance smoke fixture.\n";

const TOOLCHAIN_DIR_NAME: &str = "nightly-2026-11-14";
const TOOLCHAIN_TRIPLE: &str = "x86_64-unknown-linux-gnu";
const GIT_BIN: &str = "/usr/bin/git";
const BWRAP_BIN: &str = "/usr/bin/bwrap";
const PROV_INSIDE: &str = "/tmp/prov";
const POLICY_PATH_INSIDE: &str = "/etc/asep/acceptance.json";
const ASEP_BIN_INSIDE: &str = "/asep-bin/asep";
