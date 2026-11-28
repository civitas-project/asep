// @test:FR-ACC-02 @test:FR-ACC-04
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static BWRAP_ENV_LOCK: Mutex<()> = Mutex::new(());

use asep::acceptance_scoped::{build_test_plan, bwrap_args, run_scoped_acceptance};

#[test]
fn scoped_acceptance_writes_accepted_evidence() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    let commit = head_of(root_path);

    let evidence_dir = run_scoped_acceptance(root_path, &commit)
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
        let verdict = match id {
            "REQ-ALPHA" => "ACCEPTED",
            _ => "UNPROVEN",
        };
        let scope = match id {
            "REQ-ALPHA" => "in_scope",
            _ => "out_of_scope",
        };
        assert!(
            content.contains(&format!("verdict: {verdict}")),
            "{id} verdict: {content}"
        );
        assert!(content.contains(&format!("scope: {scope}")), "{id} scope");
        assert!(
            content.contains(&format!(
                "rationale: commit {commit}, scoped gates passed, changed paths docs/PRD.md"
            )),
            "{id} rationale"
        );
        assert!(content.contains("schema_version: 1"), "{id} schema_version");
        assert!(content.contains("revision:"), "{id} revision");
        assert!(content.contains("tree_digest:"), "{id} tree_digest");
        assert!(
            content.contains("changed_paths_digest:"),
            "{id} changed_paths_digest"
        );
        assert!(content.contains("policy_digest:"), "{id} policy_digest");
        assert!(content.contains("toolchain:"), "{id} toolchain");
        assert!(
            content.contains("tests_selected_for_requirement:"),
            "{id} per-requirement selection"
        );
        assert!(
            content.contains("tests_passed_for_requirement:"),
            "{id} per-requirement passes"
        );
        if id == "REQ-ALPHA" {
            assert!(
                content.contains("tests_passed_for_requirement: scoped_alpha_passes"),
                "REQ-ALPHA must name its actually executed passing test"
            );
        } else {
            assert!(
                !content.contains("tests_passed_for_requirement: scoped_alpha_passes"),
                "unmapped requirement must not inherit another test's pass"
            );
        }
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
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = scoped_acceptance_root();
    let root_path = root.path();

    // Recording bwrap shim: log every invocation, then exit 0 so gates
    // "succeed" without a real sandbox.
    let record = bwrap_log();
    fs::remove_file(&record).ok();
    let shim_path = bwrap_shim();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    run_scoped_acceptance(root_path, &head_of(root_path))
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
/// `asep affected --json` shim output: only REQ-ALPHA is in scope.
const AFFECTED_JSON: &str = "{\"requirements\":[{\"id\":\"REQ-ALPHA\"}],\"impls\":[],\"tests\":[{\"id\":\"test-alpha\",\"function\":\"scoped_alpha_passes\",\"file\":\"tests/scoped_traceability.rs\",\"line\":1}],\"adrs\":[],\"unmapped\":[],\"commands\":[\"cargo test --test scoped_traceability\"],\"provenance\":{\"Test:test-alpha\":[{\"node\":\"Test:test-alpha\",\"edge\":null},{\"node\":\"Req:REQ-ALPHA\",\"edge\":\"Satisfies:forward\"}]}}";

fn bwrap_shim() -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap");
    let log = bwrap_log();
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(shim, "#!/bin/sh").expect("write shebang");
    writeln!(shim, "echo \"$@\" >> {}", log.display()).expect("write log line");
    writeln!(shim, "case \"$*\" in").expect("write case open");
    writeln!(
        shim,
        "  *affected*) printf '%s\\n' '{}' 'cargo test --test scoped_traceability' ;;",
        AFFECTED_JSON
    )
    .expect("write affected arm");
    writeln!(
        shim,
        "  *\"cargo test\"*) echo 'test scoped_alpha_passes ... ok'; echo 'test result: ok. 1 passed; 0 failed' ;;"
    )
    .expect("write cargo test arm");
    writeln!(shim, "esac").expect("write case close");
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

fn bwrap_log() -> PathBuf {
    std::env::temp_dir().join("asep-acceptance-test-bwrap.log")
}

/// A bwrap shim with a caller-supplied test result, for failure-path evidence tests.
fn bwrap_shim_with_test_result(name: &str, test_line: &str) -> PathBuf {
    let shim_path = std::env::temp_dir().join(format!("asep-acceptance-test-bwrap-{name}"));
    let mut shim = fs::File::create(&shim_path).expect("create custom shim");
    writeln!(shim, "#!/bin/sh").expect("shebang");
    writeln!(shim, "case \"$*\" in").expect("case");
    writeln!(
        shim,
        "  *affected*) printf '%s\\n' '{}' 'cargo test --test scoped_traceability' ;;",
        AFFECTED_JSON
    )
    .expect("affected arm");
    writeln!(shim, "  *\"cargo test\"*) echo '{}' ;;", test_line).expect("test arm");
    writeln!(shim, "esac").expect("esac");
    writeln!(shim, "exit 0").expect("exit");
    drop(shim);
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&shim_path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).expect("perms");
    shim_path
}

/// `git -C <root> rev-parse HEAD`; panics on failure. Trimmed stdout.
fn head_of(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse HEAD");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    String::from_utf8(output.stdout)
        .expect("git rev-parse HEAD output not utf8")
        .trim()
        .to_string()
}

/// Minimal temp git repo with an acceptance policy, a PRD with two MUST
/// requirements and one SHOULD, and an ASEP_BWRAP shim that makes every gate
/// exit 0 without running anything. The policy approves the actual HEAD
/// commit so the snapshot-identity check passes.
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
    fs::write(root_path.join(".gitignore"), "acceptance.json\nevidence/\n")
        .expect("write .gitignore");
    fs::write(root_path.join("README.md"), "# hello\n").expect("write README");
    fs::create_dir_all(root_path.join("docs")).expect("docs dir");
    fs::write(
        root_path.join("docs/PRD.md"),
        "# PRD\n\n@req:REQ-ALPHA:MUST\n@req:REQ-BETA:MUST\n@req:REQ-GAMMA:SHOULD\n",
    )
    .expect("write PRD");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("add")
        .arg("README.md")
        .arg("docs/PRD.md")
        .arg(".gitignore")
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

    fs::create_dir_all(root_path.join(".spec-engine")).expect("spec-engine dir");
    fs::write(
        root_path.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[test]]\nname = \"scoped_traceability\"\npath = \"tests/scoped_traceability.rs\"\n",
    )
    .expect("Cargo.toml");
    let test_dir = root_path.join("crates/spec-engine/tests");
    fs::create_dir_all(&test_dir).expect("test dir");
    fs::create_dir_all(root_path.join("tests")).expect("tests dir");
    fs::write(root_path.join("tests/scoped_traceability.rs"), "// fixture").expect("metadata test fixture");
    fs::write(test_dir.join("scoped_traceability.rs"), "// fixture").expect("test fixture");
    run(root_path, &["git", "add", "crates", "tests", "Cargo.toml"]);
    run(root_path, &["git", "commit", "-q", "-m", "test target fixture"]);
    let commit = head_of(root_path);
    let policy = format!(
        r#"{{
        "approved_commit": "{}",
        "changed_paths": ["docs/PRD.md"],
        "evidence_dir": "evidence"
    }}"#,
        commit
    );
    fs::write(root_path.join(".spec-engine/acceptance.json"), policy).expect("write policy");

    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());

    root
}

/// Locate the `asep` binary so the test can exercise the real CLI dispatch.
fn asep_bin() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root parent");
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
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    let commit = head_of(root_path);

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
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

// @test:FR-AUD-05
#[test]
fn build_test_plan_uses_json_commands_and_adds_package() {
    let root = tempfile::tempdir().expect("temp dir");
    let root_path = root.path();
    fs::create_dir_all(root_path.join("crates").join("spec-engine").join("tests")).expect("tests dir");
    fs::write(
        root_path.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[test]]\nname = \"alpha\"\npath = \"tests/alpha.rs\"\n",
    )
    .expect("Cargo.toml");
    fs::create_dir_all(root_path.join("tests")).expect("tests source dir");
    fs::write(root_path.join("tests/alpha.rs"), "// fixture").expect("alpha source");
    fs::write(
        root_path.join("crates/spec-engine/tests/alpha.rs"),
        "#[test]\nfn alpha() {}\n",
    )
    .expect("write alpha.rs");

    let shim_path =
        affected_json_bwrap_shim(&["cargo test --test alpha", "cargo test --test ghost"]);
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    let base = bwrap_args(
        Path::new("/workspace"),
        Path::new("/toolchain"),
        Path::new("/workspace/target"),
    );
    let plan = build_test_plan(&base, &["docs/PRD.md".to_string()], root_path)
        .unwrap_or_else(|e| panic!("build_test_plan failed: {e}"));

    assert_eq!(plan.selected, 2, "plan must select both commands");
    assert_eq!(plan.commands.len(), 2);
    assert_eq!(
        plan.commands[0], "cargo test -p fixture --test alpha",
        "existing target kept with authoritative package: {}",
        plan.commands[0]
    );
    assert_eq!(
        plan.commands[1], "cargo test -p fixture --lib",
        "nonexistent target converted to --lib fallback: {}",
        plan.commands[1]
    );

    // Restore the shared shim so other tests see the default bwrap behavior.
    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());
}

/// bwrap shim that prints one JSON object whose `commands` array holds `cmds`
/// and exits 0, faking `asep affected` output without a real sandbox.
fn affected_json_bwrap_shim(cmds: &[&str]) -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap-affected-json");
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(shim, "#!/bin/sh").expect("write shebang");
    let joined = cmds.join("\", \"");
    writeln!(
        shim,
        "printf '%s\\n' '{{\"requirements\":[],\"impls\":[],\"tests\":[],\"adrs\":[],\"commands\":[\"{joined}\"],\"provenance\":{{}}}}'",
    )
    .expect("write affected json");
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

// @test:FR-AUD-03
#[test]
fn zero_executed_tests_is_error() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    let commit = head_of(root_path);

    std::env::set_var("ASEP_BWRAP", zero_tests_bwrap_shim().to_str().unwrap());

    let result = run_scoped_acceptance(root_path, &commit);
    assert!(result.is_err(), "expected zero-tests error: {result:?}");
    let error = result.unwrap_err();
    assert!(
        error.contains("zero tests"),
        "error must mention zero tests: {error}"
    );
}

/// bwrap shim: `asep affected` prints the affected JSON plus a cargo test
/// command line, but scoped test invocations print no `test result:` line,
/// so zero tests execute and run_scoped_acceptance must fail.
fn zero_tests_bwrap_shim() -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap-zero-tests");
    let log = bwrap_log();
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(shim, "#!/bin/sh").expect("write shebang");
    writeln!(shim, "echo \"$@\" >> {}", log.display()).expect("write log line");
    writeln!(shim, "case \"$*\" in").expect("write case open");
    writeln!(
        shim,
        "  *affected*) printf '%s\\n' '{}' 'cargo test --test scoped_traceability' ;;",
        AFFECTED_JSON
    )
    .expect("write affected arm");
    writeln!(shim, "esac").expect("write case close");
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

// @test:FR-AUD-02
#[test]
fn dirty_tree_evidence_is_rejected() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    fs::write(root_path.join("dirty.txt"), "dirty\n").expect("write dirty.txt");
    let result = run_scoped_acceptance(root_path, &head_of(root_path));
    assert!(result.is_err(), "expected dirty tree error: {result:?}");
    let error = result.unwrap_err();
    assert!(
        error.contains("dirty") || error.contains("untracked"),
        "error must mention dirty/untracked: {error}"
    );
    assert!(
        !root_path.join("evidence/semantic/REQ-ALPHA.md").is_file(),
        "no evidence written for REQ-ALPHA"
    );
}

#[test]
fn head_mismatch_is_rejected() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = scoped_acceptance_root();
    let root_path = _root.path();
    let policy_path = root_path.join(".spec-engine/acceptance.json");
    let policy = format!(
        r#"{{
        "approved_commit": "deadbeef",
        "changed_paths": ["docs/PRD.md"],
        "evidence_dir": "evidence"
    }}"#
    );
    fs::write(&policy_path, policy).expect("rewrite policy");
    let result = run_scoped_acceptance(root_path, "deadbeef");
    assert!(result.is_err(), "expected HEAD mismatch error: {result:?}");
    let error = result.unwrap_err();
    assert!(
        error.contains("HEAD") || error.contains("commit"),
        "error must mention HEAD or commit: {error}"
    );
}

#[test]
fn failed_test_does_not_produce_acceptance_evidence() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = scoped_acceptance_root();
    let root_path = root.path();
    let commit = head_of(root_path);
    let shim = bwrap_shim_with_test_result("failed", "test scoped_alpha_passes ... FAILED");
    std::env::set_var("ASEP_BWRAP", shim.to_str().unwrap());

    let evidence = run_scoped_acceptance(root_path, &commit).expect("acceptance run succeeds");
    let alpha = fs::read_to_string(evidence.join("semantic").join("REQ-ALPHA.md"))
        .expect("read REQ-ALPHA evidence");
    assert!(
        alpha.contains("verdict: UNPROVEN"),
        "a failed test must not produce ACCEPTED evidence: {alpha}"
    );
    assert!(
        !alpha.contains("tests_passed_for_requirement: scoped_alpha_passes"),
        "failed test must not be recorded as passed"
    );

    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());
}

#[test]
// @test:N01
fn test_identity_is_package_target_qualified_and_isolated() {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = scoped_acceptance_root();
    let root_path = root.path();
    let commit = head_of(root_path);

    fs::write(
        root_path.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[test]]\nname = \"target_a\"\npath = \"tests/target_a.rs\"\n\n[[test]]\nname = \"target_b\"\npath = \"tests/target_b.rs\"\n",
    ).unwrap();
    let test_dir = root_path.join("crates/spec-engine/tests");
    fs::create_dir_all(&test_dir).unwrap();
    fs::create_dir_all(root_path.join("tests")).unwrap();
    for name in ["target_a", "target_b"] {
        fs::write(root_path.join(format!("tests/{name}.rs")), "// fixture").unwrap();
        fs::write(test_dir.join(format!("{name}.rs")), "// fixture").unwrap();
    }
    run(root_path, &["git", "add", "crates", "tests", "Cargo.toml"]);
    run(root_path, &["git", "commit", "-q", "-m", "fixture targets"]);
    let commit = head_of(root_path);
    let policy = format!(
        r#"{{"approved_commit": "{}", "changed_paths": ["docs/PRD.md"], "evidence_dir": "evidence"}}"#,
        commit
    );
    fs::write(root_path.join(".spec-engine/acceptance.json"), policy).unwrap();
    let affected = r#"{"requirements":[{"id":"REQ-ALPHA"},{"id":"REQ-BETA"}],"tests":[{"id":"test-alpha","function":"same_name","file":"tests/target_a.rs","line":1},{"id":"test-beta","function":"same_name","file":"tests/target_b.rs","line":1}],"provenance":{"Test:test-alpha":[{"node":"Test:test-alpha","edge":null},{"node":"Req:REQ-ALPHA","edge":"Satisfies:forward"}],"Test:test-beta":[{"node":"Test:test-beta","edge":null},{"node":"Req:REQ-BETA","edge":"Satisfies:forward"}]},"commands":["cargo test -p fixture --test target_a -- same_name --exact","cargo test -p fixture --test target_b -- same_name --exact"]}"#;
    let shim_path = std::env::temp_dir().join("asep-acceptance-test-bwrap-identity");
    let mut shim = fs::File::create(&shim_path).unwrap();
    writeln!(shim, "#!/bin/sh").unwrap();
    writeln!(shim, "case \"$*\" in").unwrap();
    writeln!(shim, "  *affected*) printf '%s\\n' '{}' 'cargo test -p pkg-a --test target_a' 'cargo test -p pkg-b --test target_b' ;;", affected).unwrap();
    writeln!(shim, "  *target_a*) echo 'test same_name ... ok'; echo 'test unrelated_name ... ok' ;;").unwrap();
    writeln!(shim, "  *target_b*) echo 'test same_name ... FAILED' ;;").unwrap();
    writeln!(shim, "esac").unwrap();
    writeln!(shim, "exit 0").unwrap();
    drop(shim);
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&shim_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).unwrap();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    let evidence = run_scoped_acceptance(root_path, &commit).expect("acceptance run succeeds");
    let semantic = evidence.join("semantic");
    let alpha = fs::read_to_string(semantic.join("REQ-ALPHA.md")).unwrap();
    let beta = fs::read_to_string(semantic.join("REQ-BETA.md")).unwrap();
    assert!(alpha.contains("verdict: ACCEPTED"), "alpha: {alpha}");
    assert!(alpha.contains("fixture::target_a::same_name"), "actual package/target missing: {alpha}");
    assert!(!alpha.contains("unrelated_name"), "unrelated test must not appear as proof: {alpha}");
    assert!(beta.contains("verdict: UNPROVEN"), "same-name failure in other target must not prove beta: {beta}");

    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());
}

fn run_acceptance_with_shim(
    name: &str,
    affected: &str,
    commands: &[&str],
    outputs: &[(&str, &str)],
) -> String {
    let _guard = BWRAP_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = scoped_acceptance_root();
    let root_path = root.path();
    let mut manifest = String::from("[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n");
    let test_dir = root_path.join("crates/spec-engine/tests");
    fs::create_dir_all(&test_dir).unwrap();
    fs::create_dir_all(root_path.join("tests")).unwrap();
    let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();
    for command in commands {
        if let Some(target) = command.split_whitespace().skip_while(|t| *t != "--test").nth(1) {
            fs::write(root_path.join(format!("tests/{target}.rs")), "// fixture").unwrap();
            fs::write(test_dir.join(format!("{target}.rs")), "// fixture").unwrap();
            if declared.insert(target.to_string()) {
                manifest.push_str(&format!(
                    "\n[[test]]\nname = \"{target}\"\npath = \"tests/{target}.rs\"\n"
                ));
            }
        }
    }
    fs::write(root_path.join("Cargo.toml"), manifest).unwrap();
    run(root_path, &["git", "add", "crates", "tests", "Cargo.toml"]);
    run(root_path, &["git", "commit", "-q", "-m", "fixture"]);
    let commit = head_of(root_path);
    fs::write(
        root_path.join(".spec-engine/acceptance.json"),
        format!(
            r#"{{"approved_commit":"{commit}","changed_paths":["docs/PRD.md"],"evidence_dir":"evidence"}}"#
        ),
    )
    .unwrap();
    let shim_path = std::env::temp_dir().join(format!("asep-acceptance-collision-{name}"));
    let mut shim = fs::File::create(&shim_path).unwrap();
    writeln!(shim, "#!/bin/sh").unwrap();
    writeln!(shim, "case \"$*\" in").unwrap();
    writeln!(shim, "  *affected*) printf '%s\\n' '{}' {} ;;", affected, commands.iter().map(|c| format!("'{c}'")).collect::<Vec<_>>().join(" ")).unwrap();
    for (needle, output) in outputs {
        writeln!(shim, "  *{needle}*) printf '%s\\n' {} ;;", output.lines().map(|line| format!("'{line}'")).collect::<Vec<_>>().join(" ")).unwrap();
    }
    writeln!(shim, "esac").unwrap();
    writeln!(shim, "exit 0").unwrap();
    drop(shim);
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&shim_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).unwrap();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());
    let evidence = run_scoped_acceptance(root_path, &commit).expect("acceptance run succeeds");
    let content = fs::read_to_string(evidence.join("semantic").join("REQ-ALPHA.md")).unwrap();
    std::env::set_var("ASEP_BWRAP", bwrap_shim().to_str().unwrap());
    content
}

#[test]
// @test:N01
fn collisions_and_unknown_identities_are_rejected() {
    let affected = |file: &str| format!(
        r#"{{"requirements":[{{"id":"REQ-ALPHA"}}],"tests":[{{"id":"t","function":"same","file":"{file}","line":1}}],"provenance":{{"Test:t":[{{"node":"Test:t","edge":null}},{{"node":"Req:REQ-ALPHA","edge":"Satisfies:forward"}}]}},"commands":["cargo test -p pkg-a --test alpha -- same --exact","cargo test -p pkg-b --test alpha -- same --exact"]}}"#
    );
    let cross_package = run_acceptance_with_shim(
        "package",
        &affected("tests/alpha.rs"),
        &[
            "cargo test -p pkg-a --test alpha -- same --exact",
            "cargo test -p pkg-b --test alpha -- same --exact",
        ],
        &[
            ("pkg-a", "test same ... ok
test result: ok. 1 passed"),
            ("pkg-b", "test same ... ok
test result: ok. 1 passed"),
        ],
    );
    assert!(cross_package.contains("verdict: UNPROVEN"), "cross-package collision: {cross_package}");
    assert!(cross_package.contains("unresolved_test_identities:"), "reason missing: {cross_package}");

    let cross_module = run_acceptance_with_shim(
        "module",
        &affected("tests/alpha.rs"),
        &["cargo test -p pkg-a --test alpha -- same --exact"],
        &[("pkg-a", "test mod1::same ... ok
test mod2::same ... ok
test result: ok. 2 passed")],
    );
    assert!(cross_module.contains("verdict: UNPROVEN"), "cross-module collision: {cross_module}");


}
