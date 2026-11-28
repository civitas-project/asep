use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// @test:FR-SKILL-CLI-01
#[test]
fn test_cli_skill_help_and_subcommands() {
    let mut cmd = Command::cargo_bin("spec-engine").unwrap();
    cmd.arg("skill").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("edit"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("restore"))
        .stdout(predicate::str::contains("export"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("config"));
}

// @test:FR-SKILL-CLI-02
#[test]
fn test_cli_pack_help_contains_skills_flag() {
    let mut cmd = Command::cargo_bin("spec-engine").unwrap();
    cmd.arg("pack").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("skills"));
}

// @test:FR-SKILL-CLI-03
#[test]
fn test_cli_skill_crud_and_restore_flow() {
    let tmp = tempdir().unwrap();
    let project_dir = tmp.path();
    let skills_dir = project_dir
        .join(".agents")
        .join("plugins")
        .join("spec-engine")
        .join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // 1. Add skill via CLI
    let mut add_cmd = Command::cargo_bin("spec-engine").unwrap();
    add_cmd
        .arg("skill")
        .arg("add")
        .arg("my-new-skill")
        .arg("--name")
        .arg("My New Skill")
        .arg("--description")
        .arg("Test custom skill from CLI")
        .arg(project_dir);
    add_cmd.assert().success();

    assert!(skills_dir.join("my-new-skill").join("SKILL.md").exists());

    // 2. List skills via CLI
    let mut list_cmd = Command::cargo_bin("spec-engine").unwrap();
    list_cmd.arg("skill").arg("list").arg(project_dir);
    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("my-new-skill"))
        .stdout(predicate::str::contains("Custom"));

    // 3. Export skill via CLI
    let export_path = tmp.path().join("exported-skill.md");
    let mut export_cmd = Command::cargo_bin("spec-engine").unwrap();
    export_cmd
        .arg("skill")
        .arg("export")
        .arg("my-new-skill")
        .arg(&export_path)
        .arg(project_dir);
    export_cmd.assert().success();
    assert!(export_path.exists());

    // 4. Delete skill via CLI
    let mut del_cmd = Command::cargo_bin("spec-engine").unwrap();
    del_cmd
        .arg("skill")
        .arg("delete")
        .arg("my-new-skill")
        .arg("-y")
        .arg(project_dir);
    del_cmd.assert().success();
    assert!(!skills_dir.join("my-new-skill").exists());

    // 5. Restore built-ins via CLI
    let mut restore_cmd = Command::cargo_bin("spec-engine").unwrap();
    restore_cmd
        .arg("skill")
        .arg("restore")
        .arg("--all")
        .arg(project_dir);
    restore_cmd.assert().success();

    assert!(skills_dir.join("code-contracts").join("SKILL.md").exists());
}

#[test]
// @test:N03
fn install_skill_never_overwrites_unmanaged_skill_in_generated_dir() {
    let dir = tempfile::tempdir().unwrap();
    let skills_dir = dir.path().join(".agents").join("plugins").join("asep").join("skills");
    let skill_dir = skills_dir.join("code-contracts");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# USER CONTENT").unwrap();

    asep::skill::install::install_skill(&skills_dir, "code-contracts", false).unwrap();
    assert_eq!(
        std::fs::read_to_string(skill_dir.join("SKILL.md")).unwrap(),
        "# USER CONTENT",
        "unmanaged artifact must be preserved even in a generated directory"
    );
}
