use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// @test:FR-CLI-CHECK-01
#[test]
fn test_check_fails_on_broken_must_requirement() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Create a file with a MUST requirement but no implementation or tests
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-AUTH-01:MUST User authentication via JWT -->\n",
    )
    .expect("write spec");

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("check").arg(root);
    cmd.assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("FR-AUTH-01"));
}

// @test:FR-CLI-CHECK-02
#[test]
fn test_check_succeeds_on_closed_requirement() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Hardcoded tier T2 requires: closed relation graph + behavioral pass +
    // ACCEPTED semantic evidence for every Req node.
    write_t2_closed_project(root, "FR-AUTH-01");

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("check").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-CLI-CHECK-01
#[test]
fn test_check_json_output_mode() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-AUTH-01");

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("--json").arg("check").arg(root);
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("\"passed\": true"))
        .stdout(predicate::str::contains("\"total_requirements\": 1"));
}

// @test:FR-ACC-03-EVIDENCE-DIR
#[test]
fn test_check_uses_explicit_evidence_dir() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed T2 project with evidence kept out of the default evidence dir,
    // so `asep check --rigor T2` without --evidence-dir is Unproven (exit 3).
    write_t2_closed_project(root, "FR-EVID-01");
    fs::remove_dir_all(root.join("evidence")).expect("remove default evidence dir");

    // Valid semantic evidence in an external dir.
    let evidence_dir = dir.path().join("external-evidence");
    fs::create_dir_all(evidence_dir.join("semantic")).expect("create external evidence dir");
    fs::write(
        evidence_dir.join("semantic").join("FR-EVID-01.md"),
        "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: accepted\n",
    )
    .expect("write evidence");

    // 1. Without --evidence-dir: Unproven => failure, exit 3.
    let mut cmd_without = Command::cargo_bin("asep").expect("binary exists");
    cmd_without.arg("check").arg("--rigor").arg("T2").arg(root);
    cmd_without.assert().failure().code(3);

    // 2. With --evidence-dir: Pass => exit 0.
    let mut cmd_with = Command::cargo_bin("asep").expect("binary exists");
    cmd_with
        .arg("check")
        .arg("--rigor")
        .arg("T2")
        .arg("--evidence-dir")
        .arg(&evidence_dir)
        .arg(root);
    cmd_with.assert().success().code(0);
}

// @test:FR-ACC-05
#[test]
fn test_check_uses_config_evidence_dir() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed T2 project with evidence kept outside the default <root>/evidence
    // dir, so `asep check --rigor T2` without --evidence-dir must resolve the
    // evidence dir from config.toml (relative to target root) to pass.
    write_t2_closed_project(root, "FR-ACC-05");
    fs::remove_dir_all(root.join("evidence")).expect("remove default evidence dir");

    let config_evidence = root.join("config-evidence");
    fs::create_dir_all(config_evidence.join("semantic")).expect("create config evidence dir");
    fs::write(
        config_evidence.join("semantic").join("FR-ACC-05.md"),
        "reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: accepted\n",
    )
    .expect("write evidence");

    let mut config = asep::config::load_config(root);
    config.evidence_dir = Some("config-evidence".to_string());
    asep::config::save_config(root, &config).expect("save config");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T2").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-CLI-INIT-01
#[test]
fn test_cli_init_and_adr_commands() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // 1. Run asep init
    let mut cmd_init = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd_init.arg("init").arg(root);
    cmd_init.assert().success().code(0);
    assert!(root.join("protocol/CORE.md").exists());
    assert!(!root.join("docs/PRD.md").exists(), "init must not fabricate a MUST stub");

    // 2. Run asep adr new
    let mut cmd_adr = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd_adr
        .arg("adr")
        .arg("new")
        .arg("Database Sharding")
        .arg(root);
    cmd_adr.assert().success().code(0);
    assert!(root.join("docs/adr/0001-database-sharding.md").exists());
}

#[test]
fn test_check_velocity_mode_and_deny_drafts() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // DRAFT requirement with only implementation; relation edge closes COV-04
    // for the Impl node, behavioral + semantic evidence satisfy T2.
    write_t2_closed_project(root, "FR-RAD-01");
    // Override priority to DRAFT so the requirement stays a draft.
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-RAD-01:DRAFT Rapid prototype checkout -->\n",
    )
    .expect("write spec");

    // 1. Velocity mode passes (draft does not fail build)
    let mut cmd_velocity = Command::cargo_bin("asep").expect("binary exists");
    cmd_velocity.arg("check").arg("--rigor").arg("T0").arg(root);
    cmd_velocity
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("DRAFT"))
        .stdout(predicate::str::contains("T0"));

    // 2. --deny-drafts fails on production trunk
    let mut cmd_deny = Command::cargo_bin("asep").expect("binary exists");
    cmd_deny.arg("check").arg("--deny-drafts").arg(root);
    cmd_deny
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("--deny-drafts"));
}

/// Writes a minimal T2-closed project for `req_id`:
/// @req (MUST), @impl, @test, relation edges (satisfies + verifies with note),
/// `.spec-engine/behavioral.json {passed: true}`, and ACCEPTED semantic evidence.
fn write_t2_closed_project(root: &std::path::Path, req_id: &str) {
    fs::write(
        root.join("spec.md"),
        format!("<!-- @req:{req_id}:MUST Requirement {req_id} -->\n"),
    )
    .expect("write spec");

    fs::write(
        root.join("auth.rs"),
        format!(
            "// @impl:{req_id}\n// @rel:satisfies:{req_id}->{req_id} \"implementation satisfies requirement\"\npub fn authenticate() {{}}\n"
        ),
    )
    .expect("write impl");

    fs::write(
        root.join("auth_test.rs"),
        format!(
            "// @test:{req_id}\n// @rel:verifies:{req_id}->{req_id} \"test covers requirement\"\n#[test]\nfn test_auth() {{}}\n"
        ),
    )
    .expect("write test");

    let spec_engine_dir = root.join(".spec-engine");
    fs::create_dir_all(&spec_engine_dir).expect("create .spec-engine");
    fs::write(
        spec_engine_dir.join("behavioral.json"),
        r#"{"passed": true}"#,
    )
    .expect("write behavioral");

    let evidence_dir = root.join("evidence").join("semantic");
    fs::create_dir_all(&evidence_dir).expect("create evidence dir");
    fs::write(
        evidence_dir.join(format!("{req_id}.md")),
        format!("reviewer: qa\nevidence_state: PASS\nverdict: ACCEPTED\nrationale: accepted\n"),
    )
    .expect("write evidence");
}

// @test:FR-VERIFY-01
#[test]
fn test_missing_semantic_evidence_is_unproven() {
    use asep::graph::model::GraphReport;
    use asep::graph::model::{NodeKind, TraceNode};
    use asep::policy::TierId;
    use asep::verification::evaluate;
    use std::path::PathBuf;

    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed relation graph with a single Req node.
    let graph = GraphReport {
        nodes: vec![TraceNode {
            kind: NodeKind::Req,
            id: "FR-UNPROVEN-01".to_string(),
            file_path: PathBuf::from("spec.md"),
            line_number: 1,
        }],
        edges: vec![],
        violations: vec![],
        is_closed: true,
    };

    let behavioral = root.join(".spec-engine").join("behavioral.json");
    fs::create_dir_all(behavioral.parent().unwrap()).expect("create dir");
    fs::write(&behavioral, r#"{"passed": true}"#).expect("write behavioral");

    // Direct unit-level assertion: no semantic evidence => Unproven.
    let report = evaluate(
        &graph,
        root.join("evidence").as_path(),
        TierId::T2,
        &behavioral,
    );
    assert_eq!(
        report.status,
        asep::verification::VerificationStatus::Unproven
    );
    assert!(report.structural);
    assert!(report.behavioral);
    assert!(!report.semantic);

    // CLI end-to-end: `asep check` exits 3.
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-UNPROVEN-01:MUST Unproven requirement -->\n<!-- @rigor:".to_string()
            + "T2 -->\n",
    )
    .expect("write spec");
    fs::write(
        root.join("auth.rs"),
        "// @impl:FR-UNPROVEN-01\n// @rel:satisfies:FR-UNPROVEN-01->FR-UNPROVEN-01 \"x\"\npub fn x() {}\n",
    )
    .expect("write impl");
    fs::write(
        root.join("auth_test.rs"),
        "// @test:FR-UNPROVEN-01\n// @rel:verifies:FR-UNPROVEN-01->FR-UNPROVEN-01 \"x\"\n#[test]\nfn t() {}\n",
    )
    .expect("write test");
    // behavioral.json already written; no evidence/semantic/FR-UNPROVEN-01.md.

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("check").arg(root);
    cmd.assert().failure().code(3);
}

// @test:FR-CLI-CHECK-MESSAGES-01
#[test]
fn test_check_messages_rejects_bad_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-MSG-01");

    let messages_dir = dir.path().join("messages");
    fs::create_dir_all(&messages_dir).expect("create messages dir");
    fs::write(
        messages_dir.join("msg-bad.json"),
        r#"{"v": 99, "id": "msg-bad", "from": "leader", "to": "executor", "kind": "TASK_DISPATCH", "reply_to": null, "ref": null, "status": "sent", "ts": "2026-11-20T10:00:00Z"}"#,
    )
    .expect("write bad envelope");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg(root).arg("--messages").arg(root);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown envelope version"));
}

// @test:FR-CLI-CHECK-MESSAGES-02
#[test]
fn test_check_messages_accepts_valid_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-MSG-02");

    let store = asep::agent_protocol::MessageStore::new(dir.path());
    store
        .send(
            asep::agent_protocol::Role::Leader,
            asep::agent_protocol::Role::Executor,
            asep::agent_protocol::Kind::TaskDispatch,
            serde_json::json!({
                "task": "implement feature",
                "files": ["src/feature.rs"],
                "acceptance": "feature works",
                "verification_command": "cargo test -p asep"
            }),
            None,
            None,
        )
        .expect("send valid message");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg(root).arg("--messages").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-CLI-CHECK-MESSAGES-03
#[test]
fn test_check_messages_missing_dir_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-MSG-03");

    let missing = root.join("does_not_exist");
    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg(root).arg("--messages").arg(&missing);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "messages directory missing or unreadable",
        ));
}

// @test:FR-CLI-CHECK-MESSAGES-04
#[test]
fn test_check_messages_wrong_messages_folder_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-MSG-04");

    let messages_dir = root.join("messages");
    fs::create_dir_all(&messages_dir).expect("create messages dir");

    // Passing the messages/ folder itself (not its parent run dir) must fail closed.
    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check")
        .arg(root)
        .arg("--messages")
        .arg(&messages_dir);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "messages directory missing or unreadable",
        ));
}

// @test:FR-CLI-CHECK-ANTISLOP-01
#[test]
fn test_check_antislop_warning_only_passes_without_strict() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed T2 project so the exit code reflects anti-slop only.
    write_t2_closed_project(root, "FR-ANTISLOP-WARN");
    // Only a Warning-severity slop phrase ("very" filler) in Markdown.
    fs::write(
        root.join("notes.md"),
        "## Notes\nThis is very important documentation.\n",
    )
    .expect("write notes.md");

    // T1, no --strict: Warning does not block => pass, reported on stderr.
    let mut cmd_lenient = Command::cargo_bin("asep").expect("binary exists");
    cmd_lenient.arg("check").arg("--rigor").arg("T1").arg("--validator").arg("anti-slop").arg(root);
    cmd_lenient
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Filler wording"));

    // T1, --strict: Warning is promoted to failure => fail.
    let mut cmd_strict = Command::cargo_bin("asep").expect("binary exists");
    cmd_strict
        .arg("check")
        .arg("--rigor")
        .arg("T1")
        .arg("--strict")
        .arg("--validator")
        .arg("anti-slop")
        .arg(root);
    cmd_strict
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("Filler wording"));
}

// @test:FR-CLI-CHECK-ANTISLOP-02
#[test]
fn test_check_antislop_error_banner_fails_at_t1() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed T2 project so the exit code reflects anti-slop only.
    write_t2_closed_project(root, "FR-ANTISLOP-ERR");
    // Decorative banner comment is Error-severity and blocks at T1.
    fs::write(root.join("main.rs"), "// ==========\nfn main() {}\n").expect("write main.rs");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T1").arg("--validator").arg("anti-slop").arg(root);
    cmd.assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("=="));
}

// @test:FR-VERIFY-01
#[test]
fn test_broken_coverage_is_fail() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // MUST requirement with no relation edges => structural failure => exit 1.
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-BROKEN-01:MUST Broken coverage requirement -->\n",
    )
    .expect("write spec");

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("check").arg(root);
    cmd.assert().failure().code(1);
}

// @test:FR-ANTISLOP-05 @test:FR-ANTISLOP-06
#[test]
fn check_fails_on_guard_clause_violations() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed T2 project so the exit code reflects anti-slop only.
    write_t2_closed_project(root, "FR-GUARD-GATE");
    // Violation of AS-CODE-ELSE (`else` branch) and AS-CODE-NESTING (nested `if`).
    fs::write(
        root.join("guard.rs"),
        "fn f(a: bool, b: bool) { if a { return; } else { } if b { if true { } } }\n",
    )
    .expect("write guard.rs");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T1").arg(root);
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Else branch forbidden"))
        .stdout(predicate::str::contains("Excessive if nesting depth"));

    // Guard-clause style: sequential top-level `if`s + early returns,
    // no `else`, no nested `if`.
    fs::write(
        root.join("guard.rs"),
        "fn f(a: bool, b: bool) { if a { return; } if b { return; } }\n",
    )
    .expect("rewrite guard.rs");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T1").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-AMB-06
#[test]
fn test_ambient_skills_violations_fail_outside_t0() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Closed project so the exit code reflects ambient skills only.
    write_t2_closed_project(root, "FR-AMBIENT-01");

    // Malformed: missing pointer block, and description does not start with `Use when`.
    fs::create_dir_all(root.join("skills").join("alpha")).expect("create skills dir");
    fs::write(
        root.join("skills").join("alpha").join("SKILL.md"),
        "---\ndescription: Handles alpha routing\n---\n# Alpha\nBody text.\n",
    )
    .expect("write SKILL.md");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T1").arg("--validator").arg("ambient-skills").arg(root);
    cmd.assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("Ambient Skills Violations"))
        .stdout(predicate::str::contains("Use when"));
}

// @test:FR-AMB-06
#[test]
fn test_ambient_skills_valid_input_passes() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    write_t2_closed_project(root, "FR-AMBIENT-02");

    // Valid: pointer block present and a skill description starting with "Use when".
    fs::write(
        root.join("AGENTS.md"),
        "<!-- asep:start -->\nASEP ambient context.\n<!-- asep:end -->\n",
    )
    .expect("write AGENTS.md");
    fs::create_dir_all(root.join("skills").join("custom-skill")).expect("create skills dir");
    fs::write(
        root.join("skills").join("custom-skill").join("SKILL.md"),
        "---\ndescription: Use when custom skill applies\n---\n\
         # custom-skill\nBody.\n",
    )
    .expect("write SKILL.md");

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("check").arg("--rigor").arg("T1").arg("--validator").arg("ambient-skills").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-CTX-03
#[test]
fn test_context_command_emits_json_bundle() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    write_ctx_fixture(root);

    let mut cmd = Command::cargo_bin("asep").expect("binary exists");
    cmd.arg("context")
        .arg("src/lib.rs")
        .arg("--root")
        .arg(root)
        .arg("--json");
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("objective"))
        .stdout(predicate::str::contains("impacted"))
        .stdout(predicate::str::contains("excerpts"))
        .stdout(predicate::str::contains("truncation"))
        .stdout(predicate::str::contains("commands"));
}

/// Writes a minimal project with a requirement, impl, test, and tag relations.
fn write_ctx_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");

    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-CTX-03:MUST Context requirement -->\n",
    )
    .expect("write spec");

    fs::write(
        root.join("src").join("lib.rs"),
        "// @impl:FR-CTX-03\n// @rel:satisfies:FR-CTX-03->FR-CTX-03 \"implementation satisfies requirement\"\npub fn run() {}\n",
    )
    .expect("write lib");

    fs::write(
        root.join("tests").join("lib_test.rs"),
        format!(
            "// {}FR-CTX-03\n// relation verifying the requirement\n#[test]\nfn t() {{}}\n",
            "@test:"
        )
        .as_str(),
    )
    .expect("write test");
}
