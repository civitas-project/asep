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

    // Create spec with MUST requirement
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-AUTH-01:MUST User authentication via JWT -->\n",
    )
    .expect("write spec");

    // Create implementation
    fs::write(
        root.join("auth.rs"),
        "// @impl:FR-AUTH-01\npub fn authenticate() {}\n",
    )
    .expect("write impl");

    // Create test
    fs::write(
        root.join("auth_test.rs"),
        "// @test:FR-AUTH-01\n#[test]\nfn test_auth() {}\n",
    )
    .expect("write test");

    let mut cmd = Command::cargo_bin("spec-engine").expect("binary exists");
    cmd.arg("check").arg(root);
    cmd.assert().success().code(0);
}

// @test:FR-CLI-CHECK-01
#[test]
fn test_check_json_output_mode() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-AUTH-01:MUST User authentication via JWT -->\n",
    )
    .expect("write spec");
    fs::write(
        root.join("auth.rs"),
        "// @impl:FR-AUTH-01\npub fn authenticate() {}\n",
    )
    .expect("write impl");
    fs::write(
        root.join("auth_test.rs"),
        "// @test:FR-AUTH-01\n#[test]\nfn test_auth() {}\n",
    )
    .expect("write test");

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

    // 1. Create a DRAFT requirement with only implementation (no tests or design)
    fs::write(
        root.join("spec.md"),
        "<!-- @req:FR-RAD-01:DRAFT Rapid prototype checkout -->\n",
    )
    .expect("write spec");
    fs::write(
        root.join("checkout.rs"),
        "// @impl:FR-RAD-01\npub fn checkout() {}\n",
    )
    .expect("write impl");

    // 2. Running check in default or velocity mode passes (draft does not fail build)
    let mut cmd_velocity = Command::cargo_bin("asep").expect("binary exists");
    cmd_velocity
        .arg("check")
        .arg("--mode")
        .arg("velocity")
        .arg(root);
    cmd_velocity
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("FR-RAD-01"))
        .stdout(predicate::str::contains("velocity"));

    // 3. Running check with --deny-drafts fails on production trunk
    let mut cmd_deny = Command::cargo_bin("asep").expect("binary exists");
    cmd_deny.arg("check").arg("--deny-drafts").arg(root);
    cmd_deny
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("--deny-drafts"));
}
