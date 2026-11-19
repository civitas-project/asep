use asep::config::settings::{load_config, save_config, CapabilityProfile, EditorPreference, EngineConfig};
use std::fs;
use tempfile::tempdir;

// @test:FR-CONFIG-LOAD-01
#[test]
fn test_load_default_config_when_missing() {
    let tmp = tempdir().unwrap();
    let config = load_config(tmp.path());

    // Default editor preference should be "Ask"
    assert_eq!(config.editor.preference, EditorPreference::Ask);
    assert_eq!(config.editor.custom_command, "");
}

// @test:FR-CONFIG-SAVE-01
#[test]
fn test_save_and_reload_config_toml() {
    let tmp = tempdir().unwrap();
    let mut config = EngineConfig::default();
    config.editor.preference = EditorPreference::Terminal;
    config.editor.custom_command = "nvim".to_string();

    save_config(tmp.path(), &config).expect("Config save must succeed");

    let config_file = tmp.path().join(".spec-engine").join("config.toml");
    assert!(config_file.exists(), "config.toml must be written");

    let raw_toml = fs::read_to_string(&config_file).unwrap();
    assert!(raw_toml.contains("preference = \"terminal\""));
    assert!(raw_toml.contains("custom_command = \"nvim\""));

    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.editor.preference, EditorPreference::Terminal);
    assert_eq!(reloaded.editor.custom_command, "nvim");
}

// @test:FR-CONFIG-LOAD-02
#[test]
fn test_corrupt_config_falls_back_to_defaults() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join(".spec-engine");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), "invalid [ toml syntax :::").unwrap();

    let config = load_config(tmp.path());
    assert_eq!(config.editor.preference, EditorPreference::Ask);
}

// @test:FR-CONFIG-LOAD-03
#[test]
fn test_capabilities_load_from_toml_with_defaults() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join(".spec-engine");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), "[capabilities]\nfile_read = true\nfile_write = false\n").unwrap();

    let config = load_config(tmp.path());
    assert!(config.capabilities.file_read);
    assert!(!config.capabilities.file_write);
    // Rest default to true.
    assert!(config.capabilities.terminal);
    assert!(config.capabilities.mcp);
    assert!(config.capabilities.subagents);
    assert!(config.capabilities.web);
    assert_eq!(config.capabilities.context_window, 200_000);
}

// @test:FR-CONFIG-CAP-MISSING-01
#[test]
fn test_capability_profile_missing_reports_disabled_only() {
    let mut profile = CapabilityProfile::default();
    profile.file_write = false;

    assert_eq!(profile.missing(&["file_write", "terminal"]), vec!["file_write"]);
    assert_eq!(
        profile.missing(&["file_write", "terminal", "web"]),
        vec!["file_write"]
    );
    assert!(CapabilityProfile::default().missing(&["file_read", "web"]).is_empty());
}
