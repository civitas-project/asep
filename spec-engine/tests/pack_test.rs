use asep::pack::installer::{run_pack_installer, PackOptions};
use asep::pack::targets::{InstallScope, TargetAgent};
use std::fs;
use tempfile::tempdir;

// @test:FR-PACK-INSTALL-01
#[test]
fn test_run_pack_installer_headless_local_antigravity() {
    let tmp = tempdir().unwrap();
    let opts = PackOptions {
        scope: Some(InstallScope::Local(tmp.path().to_path_buf())),
        agents: vec![TargetAgent::Antigravity],
        non_interactive: true,
        target_dir: Some(tmp.path().to_path_buf()),
        skills_filter: None,
    };

    run_pack_installer(opts).expect("Installer must succeed in headless mode");

    let plugin_dir = tmp.path().join(".agents").join("plugins").join("asep");
    assert!(plugin_dir.exists(), "Plugin directory must exist");
    assert!(
        plugin_dir.join("plugin.json").exists(),
        "plugin.json must exist"
    );
    assert!(
        plugin_dir.join("mcp_config.json").exists(),
        "mcp_config.json must exist"
    );

    let skills_dir = plugin_dir.join("skills");
    assert!(skills_dir.exists(), "skills directory must exist");
    assert!(
        skills_dir
            .join("system-architecture")
            .join("SKILL.md")
            .exists(),
        "system-architecture skill must exist"
    );
    assert!(
        skills_dir.join("code-contracts").join("SKILL.md").exists(),
        "code-contracts skill must exist"
    );
    assert!(
        skills_dir
            .join("engineering-specs")
            .join("SKILL.md")
            .exists(),
        "engineering-specs skill must exist"
    );
    assert!(
        skills_dir
            .join("repository-governance")
            .join("SKILL.md")
            .exists(),
        "repository-governance skill must exist"
    );
    assert!(
        skills_dir.join("design-tokens").join("SKILL.md").exists(),
        "design-tokens skill must exist"
    );
}

// @test:FR-PACK-INSTALL-02
#[test]
fn test_run_pack_installer_headless_local_opencode() {
    let tmp = tempdir().unwrap();
    let opts = PackOptions {
        scope: Some(InstallScope::Local(tmp.path().to_path_buf())),
        agents: vec![TargetAgent::OpenCode],
        non_interactive: true,
        target_dir: Some(tmp.path().to_path_buf()),
        skills_filter: None,
    };

    run_pack_installer(opts).expect("Installer must succeed in headless mode");

    let skills_dir = tmp.path().join(".opencode").join("skills");
    assert!(skills_dir.exists(), ".opencode/skills must exist");
    assert!(
        skills_dir
            .join("system-architecture")
            .join("SKILL.md")
            .exists(),
        "system-architecture skill must exist"
    );

    let opencode_json = tmp.path().join("opencode.json");
    assert!(opencode_json.exists(), "opencode.json must be generated");
    let content = fs::read_to_string(&opencode_json).unwrap();
    assert!(content.contains("\"asep\""));
    assert!(content.contains("\"mcp\""));
}

// @test:FR-PACK-SELECTIVE-01
#[test]
fn test_run_pack_installer_with_selective_skills() {
    let tmp = tempdir().unwrap();
    let opts = PackOptions {
        scope: Some(InstallScope::Local(tmp.path().to_path_buf())),
        agents: vec![TargetAgent::Antigravity],
        non_interactive: true,
        target_dir: Some(tmp.path().to_path_buf()),
        skills_filter: Some(vec!["code-contracts".to_string()]),
    };

    run_pack_installer(opts).expect("Selective installer must succeed");

    let plugin_dir = tmp.path().join(".agents").join("plugins").join("asep");
    let skills_dir = plugin_dir.join("skills");
    assert!(skills_dir.join("code-contracts").join("SKILL.md").exists());
    assert!(!skills_dir.join("design-tokens").exists());
}

#[test]
fn test_run_pack_installer_headless_cursor_and_zed() {
    let tmp = tempdir().unwrap();
    let opts = PackOptions {
        scope: Some(InstallScope::Local(tmp.path().to_path_buf())),
        agents: vec![TargetAgent::Cursor, TargetAgent::Zed],
        non_interactive: true,
        target_dir: Some(tmp.path().to_path_buf()),
        skills_filter: None,
    };

    run_pack_installer(opts).expect("Installer must succeed for Cursor and Zed");

    let cursor_rules = tmp.path().join(".cursor").join("rules");
    assert!(cursor_rules.exists());
    assert!(cursor_rules.join("code-contracts.mdc").exists());
    assert!(cursor_rules.join("system-architecture.mdc").exists());

    let zed_rules = tmp.path().join(".zed").join("rules");
    assert!(zed_rules.exists());
    assert!(zed_rules.join("code-contracts.md").exists());
    assert!(zed_rules.join("system-architecture.md").exists());
}

#[test]
fn test_run_pack_installer_headless_detection_default() {
    let tmp = tempdir().unwrap();
    // Simulate an existing Zed project workspace
    fs::create_dir_all(tmp.path().join(".zed")).unwrap();

    let opts = PackOptions {
        scope: Some(InstallScope::Local(tmp.path().to_path_buf())),
        agents: vec![], // Empty triggers auto-detection
        non_interactive: true,
        target_dir: Some(tmp.path().to_path_buf()),
        skills_filter: None,
    };

    run_pack_installer(opts).expect("Installer must succeed with auto-detection");

    // Zed was detected, so its rules directory must be populated
    assert!(tmp
        .path()
        .join(".zed")
        .join("rules")
        .join("code-contracts.md")
        .exists());
}
