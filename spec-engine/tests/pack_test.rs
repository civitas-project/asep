// @test:FR-AMB-02
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

    assert!(
        !plugin_dir.join("skills").exists(),
        "skills directory must not be created by pack"
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

    assert!(
        !tmp.path().join(".opencode").join("skills").exists(),
        ".opencode/skills must not be created by pack"
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
    assert!(plugin_dir.join("plugin.json").exists(), "plugin.json must exist");
    assert!(
        plugin_dir.join("mcp_config.json").exists(),
        "mcp_config.json must exist"
    );
    assert!(
        !plugin_dir.join("skills").exists(),
        "skills_filter is ignored; no skills directory must be created"
    );
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
    assert!(!cursor_rules.exists(), "Cursor rules must not be created by pack");

    let zed_rules = tmp.path().join(".zed").join("rules");
    assert!(!zed_rules.exists(), "Zed rules must not be created by pack");
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

    // Pack must not write skill files even for auto-detected agents
    assert!(
        !tmp
            .path()
            .join(".zed")
            .join("rules")
            .join("code-contracts.md")
            .exists(),
        "no skill files must be written by pack"
    );
}
