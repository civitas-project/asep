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

// @test:FR-CLI-INIT-01
#[test]
fn test_cli_init_and_adr_commands() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // 1. Run asep init
    let mut cmd_init = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd_init.arg("init").arg(root);
    cmd_init.assert().success().code(0);
    assert!(root.join("docs/PRD.md").exists());

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
    cmd_velocity
        .arg("check")
        .arg("--rigor")
        .arg("T0")
        .arg(root);
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
    fs::write(spec_engine_dir.join("behavioral.json"), r#"{"passed": true}"#).expect("write behavioral");

    let evidence_dir = root.join("evidence").join("semantic");
    fs::create_dir_all(&evidence_dir).expect("create evidence dir");
    fs::write(
        evidence_dir.join(format!("{req_id}.md")),
        format!("reviewer: qa\nverdict: ACCEPTED\nrationale: accepted\n"),
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
    assert_eq!(report.status, asep::verification::VerificationStatus::Unproven);
    assert!(report.structural);
    assert!(report.behavioral);
    assert!(!report.semantic);

    // CLI end-to-end: `asep check` exits 3.
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-UNPROVEN-01:MUST Unproven requirement -->\n<!-- @rigor:T2 -->\n",
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
