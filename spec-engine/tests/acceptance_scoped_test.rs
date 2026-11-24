// @test:FR-ACC-02 @test:FR-ACC-04
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use asep::acceptance_scoped::{affected_test_commands, bwrap_args, run_scoped_acceptance};

#[test]
fn scoped_acceptance_writes_accepted_evidence() {
    let root = tempfile::tempdir().expect("temp dir");
    let root_path = root.path();

    // Minimal temp git repo.
    run(
        root_path,
        &["git", "init", "-q", root_path.to_str().unwrap()],
    );
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .status()
        .expect("git config email");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.name")
        .arg("test")
        .status()
        .expect("git config name");
    fs::write(root_path.join("README.md"), "# hello\n").expect("write README");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("add")
        .arg("README.md")
        .status()
        .expect("git add");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("commit")
        .arg("-q")
        .arg("-m")
        .arg("init")
        .status()
        .expect("git commit");

    // Policy + PRD with one SHOULD (excluded) and two MUST requirements.
    fs::create_dir_all(root_path.join("docs")).expect("docs dir");
    fs::write(
        root_path.join("docs/PRD.md"),
        "# PRD\n\n@req:REQ-ALPHA:MUST\n@req:REQ-BETA:MUST\n@req:REQ-GAMMA:SHOULD\n",
    )
    .expect("write PRD");
    fs::create_dir_all(root_path.join(".spec-engine")).expect("spec-engine dir");
    let policy = r#"{
        "approved_commit": "abc123",
        "changed_paths": ["docs/PRD.md"],
        "evidence_dir": "evidence"
    }"#;
    fs::write(root_path.join(".spec-engine/acceptance.json"), policy).expect("write policy");

    // Fake bwrap shim: every gate exits 0 without running anything.
    let shim_path = bwrap_shim();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    let commit = "abc123";
    let evidence_dir = run_scoped_acceptance(root_path, commit)
        .unwrap_or_else(|e| panic!("run_scoped_acceptance failed: {e}"));
    assert_eq!(evidence_dir, root_path.join("evidence"));

    let semantic = evidence_dir.join("semantic");
    for id in ["REQ-ALPHA", "REQ-BETA"] {
        let file = semantic.join(format!("{id}.md"));
        assert!(file.is_file(), "missing evidence for {id}");
        let content =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {id} evidence: {e}"));
        assert!(
            content.contains("reviewer: scoped-trusted-runner"),
            "{id} reviewer"
        );
        assert!(content.contains("verdict: ACCEPTED"), "{id} verdict");
        assert!(
            content.contains(&format!(
                "rationale: commit {commit}, scoped gates passed, changed paths docs/PRD.md"
            )),
            "{id} rationale"
        );
    }
    assert!(
        !semantic.join("REQ-GAMMA.md").exists(),
        "SHOULD must not have evidence"
    );
}

#[test]
fn bwrap_args_binds_evidence_files_when_present() {
    let root = tempfile::tempdir().expect("temp dir");
    let root_path = root.path();
    fs::create_dir_all(root_path.join(".spec-engine")).expect("spec-engine dir");
    fs::write(root_path.join(".spec-engine/behavioral.json"), "{}").expect("write behavioral.json");
    fs::write(root_path.join(".spec-engine/verification.json"), "{}")
        .expect("write verification.json");

    let behavioral = root_path.join(".spec-engine/behavioral.json");
    let verification = root_path.join(".spec-engine/verification.json");

    let args = bwrap_args(
        root_path,
        Path::new("/toolchain"),
        Path::new("/workspace/target"),
    );
    let windows: Vec<Vec<String>> = args.windows(3).map(|w| w.to_vec()).collect();

    let exe = std::env::current_exe().expect("current exe path");
    assert!(
        args.windows(2)
            .any(|w| w == ["--tmpfs", "/workspace/.spec-engine"]),
        "tmpfs hides .spec-engine"
    );
    assert!(
        args.windows(7).any(|w| {
            w == [
                "--dir".to_string(),
                "/asep".to_string(),
                "--dir".to_string(),
                "/asep/bin".to_string(),
                "--ro-bind".to_string(),
                exe.to_str().unwrap().to_string(),
                "/asep/bin/asep".to_string(),
            ]
        }),
        "missing --dir /asep --dir /asep/bin --ro-bind <exe> /asep/bin/asep"
    );
    assert!(
        windows.contains(&vec![
            "--ro-bind".to_string(),
            behavioral.to_str().unwrap().to_string(),
            "/workspace/.spec-engine/behavioral.json".to_string(),
        ]),
        "behavioral.json bind missing"
    );
    assert!(
        !windows.iter().any(|w| {
            w == &vec![
                "--ro-bind".to_string(),
                verification.to_str().unwrap().to_string(),
                "/workspace/.spec-engine/verification.json".to_string(),
            ]
        }),
        "verification.json must not be ro-bind (check writes it inside the sandbox)"
    );
    let tmpfs_pos = args
        .iter()
        .position(|a| a == "--tmpfs")
        .expect("tmpfs flag present");
    let behavioral_pos = args
        .iter()
        .position(|a| a == "/workspace/.spec-engine/behavioral.json")
        .expect("behavioral bind target present");
    assert!(
        tmpfs_pos < behavioral_pos,
        "binds must come after tmpfs overlay"
    );
    // The acceptance policy and the whole .spec-engine dir stay hidden.
    assert!(!args
        .iter()
        .any(|a| a == "acceptance.json" || a.ends_with("/acceptance.json")));
    assert!(
        !args
            .windows(2)
            .any(|w| w[0] == "--ro-bind" && w[1].ends_with("/.spec-engine")),
        ".spec-engine dir must not be bound wholesale"
    );

    // Files absent => no binds.
    let empty = tempfile::tempdir().expect("temp dir");
    let empty_args = bwrap_args(
        empty.path(),
        Path::new("/toolchain"),
        Path::new("/workspace/target"),
    );
    assert!(
        !empty_args
            .iter()
            .any(|a| a.contains("behavioral.json") || a.contains("verification.json")),
        "binds must not appear when evidence files are missing"
    );
}

/// Gate commands must invoke /asep/bin/asep (the exe bound into the
/// sandbox), not a host path invisible inside bwrap.
#[test]
fn scoped_gate_commands_use_sandbox_exe() {
    let root = scoped_acceptance_root();
    let root_path = root.path();

    // Recording bwrap shim: log every invocation, then exit 0 so gates
    // "succeed" without a real sandbox.
    let record = bwrap_log();
    fs::remove_file(&record).ok();
    let shim_path = bwrap_shim();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    run_scoped_acceptance(root_path, "abc123")
        .unwrap_or_else(|e| panic!("run_scoped_acceptance failed: {e}"));

    let log = fs::read_to_string(&record).expect("read gate command log");
    assert!(
        log.lines()
            .any(|line| line.contains("/asep/bin/asep ci --gate 0 --json")),
        "gate (a) did not use /asep/bin/asep: {log}"
    );
    assert!(
        log.lines()
            .any(|line| line.contains("/asep/bin/asep check --rigor T1")),
        "gate (b) did not use /asep/bin/asep: {log}"
    );
    assert!(
        log.lines()
            .any(|line| line.contains("/asep/bin/asep affected docs/PRD.md")),
        "gate (c) did not use /asep/bin/asep: {log}"
    );
}

fn run(root: &Path, args: &[&str]) {
    let status = Command::new(args[0])
        .args(&args[1..])
        .current_dir(root)
        .status();
    let status = status.unwrap_or_else(|e| panic!("failed to run {}: {e}", args[0]));
    assert!(status.success(), "command {:?} failed", args);
}

/// Persistent bwrap shim shared by all tests in this binary. Tests run in
/// parallel and `ASEP_BWRAP` is process-global, so every test must point at
/// the same path that outlives any single test's temp dirs.
fn bwrap_shim() -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap");
    let log = bwrap_log();
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(
        shim,
        "#!/bin/sh\necho \"$@\" >> {}\nexit 0\n",
        log.display()
    )
    .expect("write shim");
    drop(shim);
    let mut perms = fs::metadata(&shim_path)
        .expect("shim metadata")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).expect("shim perms");
    shim_path
}

fn bwrap_log() -> PathBuf {
    std::env::temp_dir().join("asep-acceptance-test-bwrap.log")
}

/// Minimal temp git repo with an acceptance policy, a PRD with two MUST
/// requirements and one SHOULD, and an ASEP_BWRAP shim that makes every gate
/// exit 0 without running anything.
fn scoped_acceptance_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temp dir");
    let root_path = root.path();

    run(
        root_path,
        &["git", "init", "-q", root_path.to_str().unwrap()],
    );
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .status()
        .expect("git config email");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.name")
        .arg("test")
        .status()
        .expect("git config name");
    fs::write(root_path.join("README.md"), "# hello\n").expect("write README");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("add")
        .arg("README.md")
        .status()
        .expect("git add");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("commit")
        .arg("-q")
        .arg("-m")
        .arg("init")
        .status()
        .expect("git commit");

    fs::create_dir_all(root_path.join("docs")).expect("docs dir");
    fs::write(
        root_path.join("docs/PRD.md"),
        "# PRD\n\n@req:REQ-ALPHA:MUST\n@req:REQ-BETA:MUST\n@req:REQ-GAMMA:SHOULD\n",
    )
    .expect("write PRD");
    fs::create_dir_all(root_path.join(".spec-engine")).expect("spec-engine dir");
    let policy = r#"{
        "approved_commit": "abc123",
        "changed_paths": ["docs/PRD.md"],
        "evidence_dir": "evidence"
    }"#;
    fs::write(root_path.join(".spec-engine/acceptance.json"), policy).expect("write policy");

    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());

    root
}

/// Locate the `asep` binary so the test can exercise the real CLI dispatch.
fn asep_bin() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root parent");
    let bin = workspace_root.join("target").join("debug").join("asep");
    if !bin.exists() {
        let status = Command::new("cargo")
            .args(["build", "-p", "asep"])
            .current_dir(workspace_root)
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build -p asep failed");
    }
    bin
}

#[test]
fn accept_scoped_cli_writes_evidence() {
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    let commit = "abc123";

    let output = Command::new(asep_bin())
        .arg("accept-scoped")
        .arg(commit)
        .current_dir(root_path)
        .output()
        .expect("spawn asep accept-scoped");

    assert!(
        output.status.success(),
        "accept-scoped failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let evidence_dir = root_path.join("evidence");
    assert!(
        stdout.trim().ends_with("evidence"),
        "evidence dir path not printed: {stdout}"
    );
    assert!(
        evidence_dir.join("semantic/REQ-ALPHA.md").is_file(),
        "missing MUST evidence"
    );
    assert!(
        !evidence_dir.join("semantic/REQ-GAMMA.md").exists(),
        "SHOULD must not have evidence"
    );
}

#[test]
fn accept_scoped_cli_rejects_unapproved_commit() {
    let _root = scoped_acceptance_root();
    let root_path = _root.path();

    let output = Command::new(asep_bin())
        .arg("accept-scoped")
        .arg("unapproved-commit")
        .current_dir(root_path)
        .output()
        .expect("spawn asep accept-scoped");

    assert!(
        !output.status.success(),
        "expected failure for unapproved commit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not the approved commit"),
        "missing diagnostic: {stderr}"
    );
}

#[test]
fn affected_test_commands_filters_nonexistent_test_targets() {
    // Fake `asep affected` output: two commands naming nonexistent test
    // targets plus one valid command. Only the valid command must be returned.
    let shim_path = fake_affected_bwrap_shim(&[
        "cargo test --test TRACEABILITY",
        "cargo test --test traceability_tests",
        "cargo test --test scoped_traceability",
    ]);
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    let base = bwrap_args(
        Path::new("/workspace"),
        Path::new("/toolchain"),
        Path::new("/workspace/target"),
    );
    let commands = affected_test_commands(&base, &["docs/PRD.md".to_string()])
        .unwrap_or_else(|e| panic!("affected_test_commands failed: {e}"));

    assert_eq!(
        commands,
        vec!["cargo test --test scoped_traceability".to_string()],
        "only the valid test command must survive the filter"
    );

    // Restore the shared shim so other tests see the default bwrap behavior.
    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());
}

/// bwrap shim that prints `lines` to stdout and exits 0, faking `asep
/// affected` output without a real sandbox.
fn fake_affected_bwrap_shim(lines: &[&str]) -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap-fake-affected");
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(shim, "#!/bin/sh").expect("write shebang");
    for line in lines {
        writeln!(shim, "echo {}", line).expect("write echo");
    }
    writeln!(shim, "exit 0").expect("write exit");
    drop(shim);
    let mut perms = fs::metadata(&shim_path)
        .expect("shim metadata")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).expect("shim perms");
    shim_path
}
