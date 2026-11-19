use assert_cmd::Command;
use clap::Parser;
use predicates::prelude::*;
use tempfile::tempdir;

use asep::cli::{Cli, Commands};

// @test:FR-CLI-RIGOR-01
#[test]
fn test_cli_check_accepts_rigor_flag() {
    let cli = Cli::try_parse_from(["asep", "check", "--rigor", "T2", "."]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Check { rigor, .. }) if rigor.as_deref() == Some("T2")
    ));
}

// @test:FR-CLI-RIGOR-02
#[test]
fn test_cli_check_rejects_legacy_mode_flag() {
    let err = Cli::try_parse_from(["asep", "check", "--mode", "axiomatic", "."]).unwrap_err();
    assert!(err.to_string().contains("mode"));
}

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
