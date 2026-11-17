use assert_cmd::Command;

// @test:FR-CLI-SCAN-01
#[test]
fn test_cli_version_and_help() {
    let mut cmd_asep = Command::cargo_bin("asep").expect("asep binary must be built");
    cmd_asep.arg("--help").assert().success();

    let mut cmd_spec_engine =
        Command::cargo_bin("spec-engine").expect("spec-engine alias binary must be built");
    cmd_spec_engine.arg("--help").assert().success();
}
