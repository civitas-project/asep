use asep::config::settings::CapabilityProfile;
use asep::pack::manifest::{
    generate_plugin_json, scaffold_opencode_agents, update_opencode_json,
};
use asep::pack::targets::{resolve_target_paths, InstallScope, TargetAgent};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// @test:FR-PACK-TARGETS-01
#[test]
fn test_resolve_local_paths() {
    let base = PathBuf::from("/mock/project");
    let scope = InstallScope::Local(base.clone());

    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Antigravity),
        base.join(".agents").join("plugins").join("asep")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::OpenCode),
        base.join(".opencode").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::ClaudeCode),
        base.join(".claude").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Cursor),
        base.join(".cursor").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Windsurf),
        base.join(".windsurf").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Continue),
        base.join(".continue").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::RooCode),
        base.join(".roo").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Copilot),
        base.join(".github").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Goose),
        base.join(".goose").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Zed),
        base.join(".zed").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Aider),
        base.join(".aider").join("conventions")
    );
}

// @test:FR-PACK-TARGETS-02
#[test]
fn test_resolve_global_paths() {
    let mock_home = PathBuf::from("/home/user");
    let scope = InstallScope::Global(mock_home.clone());

    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Antigravity),
        mock_home
            .join(".gemini")
            .join("config")
            .join("plugins")
            .join("asep")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::OpenCode),
        mock_home.join(".config").join("opencode").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::ClaudeCode),
        mock_home.join(".claude").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Cursor),
        mock_home.join(".cursor").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Windsurf),
        mock_home.join(".codeium").join("windsurf").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Continue),
        mock_home.join(".continue").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::RooCode),
        mock_home.join(".roo").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Copilot),
        mock_home
            .join(".config")
            .join("github-copilot")
            .join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Goose),
        mock_home.join(".config").join("goose").join("skills")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Zed),
        mock_home.join(".config").join("zed").join("rules")
    );
    assert_eq!(
        resolve_target_paths(&scope, TargetAgent::Aider),
        mock_home.join(".aider").join("conventions")
    );
}

#[test]
fn test_agent_detection_logic() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // In an empty directory, local detection should be false
    let scope_local = InstallScope::Local(root.to_path_buf());
    assert!(!TargetAgent::Cursor.is_detected(&scope_local));
    assert!(!TargetAgent::Zed.is_detected(&scope_local));

    // Create markers
    fs::create_dir_all(root.join(".cursor")).unwrap();
    fs::create_dir_all(root.join(".zed")).unwrap();

    assert!(TargetAgent::Cursor.is_detected(&scope_local));
    assert!(TargetAgent::Zed.is_detected(&scope_local));
    assert!(!TargetAgent::Goose.is_detected(&scope_local));

    // Global scope detection
    let global_root = tempdir().unwrap();
    let scope_global = InstallScope::Global(global_root.path().to_path_buf());
    if !asep::pack::targets::is_binary_in_path("opencode") {
        assert!(!TargetAgent::OpenCode.is_detected(&scope_global));
    }
    if !asep::pack::targets::is_binary_in_path("goose") {
        assert!(!TargetAgent::Goose.is_detected(&scope_global));
        fs::create_dir_all(global_root.path().join(".config").join("goose")).unwrap();
        assert!(TargetAgent::Goose.is_detected(&scope_global));
    }

    fs::create_dir_all(global_root.path().join(".config").join("opencode")).unwrap();
    assert!(TargetAgent::OpenCode.is_detected(&scope_global));
}

// @test:FR-PACK-MANIFEST-01
#[test]
fn test_generate_plugin_json() {
    let json_str = generate_plugin_json();
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("Valid plugin manifest JSON");
    assert_eq!(val["name"], "asep");
}

// @test:FR-PACK-MANIFEST-02
#[test]
fn test_update_opencode_json_preserves_existing_keys_and_merges_mcp() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    // Existing user config with custom plugin and settings
    let initial_json = r#"{
        "plugin": ["some-existing-plugin"],
        "customSetting": true
    }"#;
    fs::write(&config_file, initial_json).unwrap();

    update_opencode_json(&config_file, "asep").unwrap();

    let updated_content = fs::read_to_string(&config_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&updated_content).unwrap();

    // Verify existing keys remain intact
    assert_eq!(parsed["customSetting"], true);
    assert_eq!(parsed["plugin"][0], "some-existing-plugin");

    // Verify MCP config is properly added
    assert!(parsed["mcp"]["asep"].is_object());
    assert_eq!(parsed["mcp"]["asep"]["type"], "local");
    assert_eq!(parsed["mcp"]["asep"]["command"], "asep");
    assert_eq!(parsed["mcp"]["asep"]["args"][0], "mcp");
    assert_eq!(parsed["mcp"]["asep"]["enabled"], true);
}

// @test:ARCH-06
#[test]
fn test_scaffold_opencode_agents_merges_without_clobbering_existing_agents() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    let initial_json = r#"{
        "provider": "openai",
        "agents": {
            "reviewer": {
                "description": "Custom reviewer agent",
                "mode": "subagent"
            }
        }
    }"#;
    fs::write(&config_file, initial_json).unwrap();

    scaffold_opencode_agents(&config_file, &CapabilityProfile::default()).unwrap();

    let updated_content = fs::read_to_string(&config_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&updated_content).unwrap();

    // Pre-existing top-level keys are preserved.
    assert_eq!(parsed["provider"], "openai");

    // Pre-existing agent is preserved, untouched.
    assert!(parsed["agents"]["reviewer"].is_object());
    assert_eq!(
        parsed["agents"]["reviewer"]["description"],
        "Custom reviewer agent"
    );
    assert_eq!(parsed["agents"]["reviewer"]["mode"], "subagent");

    // Default agents are added.
    assert_eq!(
        parsed["agents"]["lead"]["description"],
        "Orchestrator for the bounded executor-judge workflow."
    );
    assert_eq!(parsed["agents"]["lead"]["mode"], "primary");
    assert_eq!(
        parsed["agents"]["executor"]["description"],
        "Fast coding executor in a bounded executor-judge pipeline."
    );
    assert_eq!(parsed["agents"]["executor"]["mode"], "subagent");
    assert_eq!(
        parsed["agents"]["judge"]["description"],
        "Read-only reviewer verifying executor output against task, diff, and test evidence."
    );
    assert_eq!(parsed["agents"]["judge"]["mode"], "subagent");
    assert_eq!(
        parsed["agents"]["explorer"]["description"],
        "Fast explorer subagent for codebase exploration."
    );
    assert_eq!(parsed["agents"]["explorer"]["mode"], "subagent");

    // Backup of the original file was created.
    assert!(tmp.path().join("opencode.json.bak").exists());
}

#[test]
fn test_scaffold_opencode_agents_writes_full_presets() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    scaffold_opencode_agents(&config_file, &CapabilityProfile::default()).unwrap();

    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_file).unwrap()).unwrap();

    for name in ["lead", "executor", "judge", "explorer"] {
        let agent = &parsed["agents"][name];
        assert!(agent["system"].as_str().is_some_and(|s| !s.is_empty()), "{name}: system missing");
        assert!(agent["steps"].is_u64(), "{name}: steps missing");
        assert!(agent["permissions"].is_array(), "{name}: permissions missing");
    }

    // Lead must know when to delegate mapping to the explorer.
    let lead_system = parsed["agents"]["lead"]["system"].as_str().unwrap();
    assert!(lead_system.contains("`explorer` subagent"), "lead prompt lacks explorer rule");

    // Executor and judge carry their own scope and step rules.
    let exec_system = parsed["agents"]["executor"]["system"].as_str().unwrap();
    assert!(exec_system.contains("BLOCKED: task needs smaller scope"));
    let judge_system = parsed["agents"]["judge"]["system"].as_str().unwrap();
    assert!(judge_system.contains("at most 20 steps"));

    // Explorer must stay read-only while keeping read + srcdump/srcidx access.
    let explorer_perms = parsed["agents"]["explorer"]["permissions"].as_array().unwrap();
    let has = |action: &str, effect: &str| explorer_perms.iter().any(|r| {
        r["action"].as_str() == Some(action) && r["effect"].as_str() == Some(effect)
    });
    assert!(has("*", "deny"), "explorer must have a broad deny rule");
    assert!(has("execute", "allow"), "explorer needs execute for srcdump");
    assert!(has("asep_srcdump", "allow"), "explorer needs srcdump access");
    assert!(has("asep_srcidx", "allow"), "explorer needs srcidx access");
    assert!(!has("asep_adr_new", "allow"), "explorer must not be allowed to write ADRs");
}

// @test:FR-PACK-MANIFEST-03
#[test]
fn test_scaffold_opencode_agents_denies_disabled_capabilities() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    let mut profile = CapabilityProfile::default();
    profile.file_write = false;

    scaffold_opencode_agents(&config_file, &profile).unwrap();

    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_file).unwrap()).unwrap();

    let has = |agent: &str, action: &str, effect: &str| {
        parsed["agents"][agent]
            ["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["action"].as_str() == Some(action) && r["effect"].as_str() == Some(effect))
    };

    // file_write = false must deny edit for every agent.
    for name in ["lead", "executor", "judge", "explorer"] {
        assert!(has(name, "edit", "deny"), "{name}: edit must be denied");
    }

    // Read capabilities remain allowed for every agent.
    for name in ["lead", "executor", "judge", "explorer"] {
        assert!(has(name, "read", "allow"), "{name}: read must be allowed");
        assert!(has(name, "glob", "allow"), "{name}: glob must be allowed");
        assert!(has(name, "grep", "allow"), "{name}: grep must be allowed");
    }
}
