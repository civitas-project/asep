use asep::config::settings::{load_config, save_config, EditorPreference, EngineConfig};
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
