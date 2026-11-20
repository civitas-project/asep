use asep::config::settings::CapabilityProfile;
use asep::pack::manifest::{
    generate_plugin_json, scaffold_opencode_agents, update_opencode_json,
};
use asep::pack::targets::{resolve_target_paths, InstallScope, TargetAgent};
use std::collections::HashSet;
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
    assert!(lead_system.contains("docs/PRD.md"), "lead prompt lacks PRD reference");
    assert!(lead_system.contains("docs/TDD.md"), "lead prompt lacks TDD reference");
    assert!(lead_system.contains(".superpowers/"), "lead prompt lacks superpowers prohibition");

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

// @test:FR-PACK-MANIFEST-05
#[test]
fn test_scaffold_opencode_agents_scopes_agent_message_tools_per_role() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    scaffold_opencode_agents(&config_file, &CapabilityProfile::default()).unwrap();

    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_file).unwrap()).unwrap();

    let perms = |agent: &str| {
        parsed["agents"][agent]["permissions"]
            .as_array()
            .expect("agent permissions must be an array")
    };
    let has = |rules: &[serde_json::Value], action: &str, resource: &str, effect: &str| -> bool {
        rules.iter().any(|r| {
            r["action"].as_str() == Some(action)
                && r["resource"].as_str() == Some(resource)
                && r["effect"].as_str() == Some(effect)
        })
    };

    // Lead may send/recv/read/ack, no kind restriction.
    let lead = perms("lead");
    assert!(has(lead, "asep_agent_send", "*", "allow"), "lead: asep_agent_send allow");
    assert!(has(lead, "asep_agent_recv", "*", "allow"), "lead: asep_agent_recv allow");
    assert!(has(lead, "asep_agent_read", "*", "allow"), "lead: asep_agent_read allow");
    assert!(has(lead, "asep_agent_ack", "*", "allow"), "lead: asep_agent_ack allow");

    // Executor: send only TASK_REPORT and recv; no read/ack; subagent stays denied.
    let executor = perms("executor");
    assert!(has(executor, "asep_agent_send", "kind=TASK_REPORT", "allow"));
    assert!(has(executor, "asep_agent_recv", "*", "allow"));
    assert!(!has(executor, "asep_agent_read", "*", "allow"));
    assert!(!has(executor, "asep_agent_ack", "*", "allow"));
    assert!(has(executor, "subagent", "*", "deny"));

    // Judge: send only VERDICT and read; no recv/ack; subagent stays denied.
    let judge = perms("judge");
    assert!(has(judge, "asep_agent_send", "kind=VERDICT", "allow"));
    assert!(has(judge, "asep_agent_read", "*", "allow"));
    assert!(!has(judge, "asep_agent_recv", "*", "allow"));
    assert!(!has(judge, "asep_agent_ack", "*", "allow"));
    assert!(has(judge, "subagent", "*", "deny"));

    // Explorer: send only MAP_REPORT; no recv/read/ack; subagent stays denied.
    let explorer = perms("explorer");
    assert!(has(explorer, "asep_agent_send", "kind=MAP_REPORT", "allow"));
    assert!(!has(explorer, "asep_agent_recv", "*", "allow"));
    assert!(!has(explorer, "asep_agent_read", "*", "allow"));
    assert!(!has(explorer, "asep_agent_ack", "*", "allow"));
    assert!(has(explorer, "subagent", "*", "deny"));
}

// @test:FR-PACK-MANIFEST-04
#[test]
fn test_scaffold_opencode_agents_keeps_read_only_agents_read_only() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    // Default profile is all capabilities true; read-only agents must still
    // be denied write/shell/web/mcp/subagent/edit (last-match-wins).
    scaffold_opencode_agents(&config_file, &CapabilityProfile::default()).unwrap();

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

    // Judge stays read-only: deny write-like actions, keep read/glob/grep.
    for action in ["edit", "write", "shell", "subagent", "webfetch", "websearch", "mcp"] {
        assert!(
            has("judge", action, "deny"),
            "judge: {action} must be denied under default profile"
        );
    }
    assert!(has("judge", "read", "allow"), "judge: read must be allowed");
    assert!(has("judge", "glob", "allow"), "judge: glob must be allowed");
    assert!(has("judge", "grep", "allow"), "judge: grep must be allowed");

    // Explorer stays read-only: deny write-like actions, keep its tools.
    for action in ["edit", "write", "shell", "subagent", "webfetch", "websearch", "mcp"] {
        assert!(
            has("explorer", action, "deny"),
            "explorer: {action} must be denied under default profile"
        );
    }
    assert!(has("explorer", "read", "allow"), "explorer: read must be allowed");
    assert!(has("explorer", "glob", "allow"), "explorer: glob must be allowed");
    assert!(has("explorer", "grep", "allow"), "explorer: grep must be allowed");
    assert!(
        has("explorer", "execute", "allow"),
        "explorer: execute must be allowed"
    );
    assert!(
        has("explorer", "asep_srcdump", "allow"),
        "explorer: asep_srcdump must be allowed"
    );
    assert!(
        has("explorer", "asep_srcidx", "allow"),
        "explorer: asep_srcidx must be allowed"
    );

    // Lead and executor keep capability-derived allows under default profile.
    assert!(has("lead", "edit", "allow"), "lead: edit must be allowed");
    assert!(has("lead", "shell", "allow"), "lead: shell must be allowed");
    assert!(has("executor", "edit", "allow"), "executor: edit must be allowed");
    assert!(has("executor", "shell", "allow"), "executor: shell must be allowed");
}

// @test:FR-PACK-MANIFEST-06
#[test]
fn test_scaffold_opencode_agents_merges_missing_permissions_into_existing_agent() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("opencode.json");

    // Pre-existing executor agent with a subset of the default rules (one of
    // them intentionally identical to a default rule to exercise dedup).
    let initial_json = r#"{
        "provider": "openai",
        "agents": {
            "executor": {
                "system": "Custom executor system prompt.",
                "steps": 99,
                "description": "Custom executor agent",
                "mode": "subagent",
                "permissions": [
                    {"action": "read", "resource": "*", "effect": "allow"},
                    {"action": "asep_agent_send", "resource": "kind=TASK_REPORT", "effect": "allow"}
                ]
            }
        }
    }"#;
    fs::write(&config_file, initial_json).unwrap();

    scaffold_opencode_agents(&config_file, &CapabilityProfile::default()).unwrap();

    let updated_content = fs::read_to_string(&config_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&updated_content).unwrap();

    let executor = &parsed["agents"]["executor"];

    // Existing agent fields must be preserved unchanged.
    assert_eq!(executor["system"], "Custom executor system prompt.");
    assert_eq!(executor["steps"], 99);
    assert_eq!(executor["description"], "Custom executor agent");
    assert_eq!(executor["mode"], "subagent");

    let perms = executor["permissions"].as_array().expect("permissions must be an array");
    let has = |action: &str, resource: &str, effect: &str| -> bool {
        perms.iter().any(|r| {
            r["action"].as_str() == Some(action)
                && r["resource"].as_str() == Some(resource)
                && r["effect"].as_str() == Some(effect)
        })
    };

    // Missing default permission rules were merged in (last-match-wins: appended after existing rules).
    assert!(has("asep_agent_send", "kind=TASK_REPORT", "allow"));
    assert!(has("asep_agent_recv", "*", "allow"));
    assert!(has("read", "*", "allow"), "existing rule kept");

    // No duplicate rules (identical action+resource+effect).
    let mut seen: HashSet<(&str, &str, &str)> = HashSet::new();
    for rule in perms {
        let key = (
            rule["action"].as_str().unwrap_or(""),
            rule["resource"].as_str().unwrap_or(""),
            rule["effect"].as_str().unwrap_or(""),
        );
        assert!(seen.insert(key), "duplicate permission rule: {:?}", key);
    }

    // Top-level keys untouched.
    assert_eq!(parsed["provider"], "openai");
}
