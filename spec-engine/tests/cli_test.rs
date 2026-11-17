use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

// @test:FR-CLI-PACK-01
#[test]
fn test_cli_pack_help() {
    let mut cmd = Command::cargo_bin("asep").expect("asep binary");
    cmd.arg("pack")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Install plugins and skills"));
}

// @test:FR-CLI-MCP-01
#[test]
fn test_cli_mcp_help() {
    let mut cmd = Command::cargo_bin("asep").expect("asep binary");
    cmd.arg("mcp")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Model Context Protocol"));
}

// @test:FR-CLI-PACK-02
#[test]
fn test_cli_pack_headless_execution() {
    let tmp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("asep").expect("asep binary");
    cmd.arg("pack")
        .arg("--local")
        .arg("--yes")
        .arg("--dir")
        .arg(tmp.path())
        .assert()
        .success();

    assert!(tmp
        .path()
        .join(".agents")
        .join("plugins")
        .join("asep")
        .exists());
}
