// @test:FR-CAP-01 // @test:FR-CAP-03 // @test:FR-CAP-04 // @test:FR-CAP-08
// @test:FR-LEVEL-02
use asep::config::settings::{
    load_config, save_config, CapabilityProfile, EditorPreference, EngineConfig,
};
use asep::runtime::EngineerLevel;
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

// @test:FR-ACC-05-CONFIG-ROUNDTRIP
#[test]
fn test_evidence_dir_round_trip() {
    let tmp = tempdir().unwrap();
    let mut config = EngineConfig::default();
    config.evidence_dir = Some("custom-evidence".to_string());

    save_config(tmp.path(), &config).expect("Config save must succeed");

    let config_file = tmp.path().join(".spec-engine").join("config.toml");
    assert!(config_file.exists(), "config.toml must be written");

    let raw_toml = fs::read_to_string(&config_file).unwrap();
    assert!(raw_toml.contains("evidence_dir = \"custom-evidence\""));

    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.evidence_dir, Some("custom-evidence".to_string()));
}

// @test:FR-ACC-05-CONFIG-DEFAULT
#[test]
fn test_evidence_dir_defaults_to_none() {
    let tmp = tempdir().unwrap();
    let config = load_config(tmp.path());
    assert_eq!(config.evidence_dir, None);
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
    fs::write(
        dir.join("config.toml"),
        "[capabilities]\nfile_read = true\nfile_write = false\n",
    )
    .unwrap();

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

    assert_eq!(
        profile.missing(&["file_write", "terminal"]),
        vec!["file_write"]
    );
    assert_eq!(
        profile.missing(&["file_write", "terminal", "web"]),
        vec!["file_write"]
    );
    assert!(CapabilityProfile::default()
        .missing(&["file_read", "web"])
        .is_empty());
}

// @test:FR-CONFIG-CAP-MEDIA-01
#[test]
fn test_supports_media_empty_returns_true() {
    let profile = CapabilityProfile::default();
    assert!(profile.supports_media(""));
}

// @test:FR-CONFIG-CAP-MEDIA-02
#[test]
fn test_supports_media_supported_returns_true() {
    let profile = CapabilityProfile::default();
    assert!(profile.supports_media("text"));
    assert!(profile.supports_media("code"));
}

// @test:FR-CONFIG-CAP-MEDIA-03
#[test]
fn test_supports_media_unsupported_returns_false() {
    let profile = CapabilityProfile::default();
    assert!(!profile.supports_media("image"));
    assert!(!profile.supports_media("audio"));
}

// @test:FR-CONFIG-CAP-COVER-01
#[test]
fn test_covers_missing_capability_returns_err() {
    let mut profile = CapabilityProfile::default();
    profile.file_write = false;

    let err = profile.covers(&["file_write"], None).unwrap_err();
    assert_eq!(err, "BLOCKED: requires file_write");
}

// @test:FR-CONFIG-CAP-COVER-02
#[test]
fn test_covers_unsupported_media_returns_err() {
    let profile = CapabilityProfile::default();

    let err = profile.covers(&["file_read"], Some("image")).unwrap_err();
    assert_eq!(err, "BLOCKED: requires media:image");
}

// @test:FR-CONFIG-CAP-COVER-03
#[test]
fn test_covers_all_satisfied_returns_ok() {
    let profile = CapabilityProfile::default();

    assert!(profile.covers(&["file_read", "web"], None).is_ok());
    assert!(profile.covers(&["file_read", "web"], Some("text")).is_ok());
    assert!(profile.covers(&[], Some("")).is_ok());
    assert!(profile.covers(&[], None).is_ok());
}

// @test:IN-01-OVERHEAD
#[test]
fn test_telemetry_defaults() {
    let tmp = tempdir().unwrap();
    let config = load_config(tmp.path());

    assert_eq!(config.telemetry.enabled, false);
    assert_eq!(config.telemetry.dir, ".spec-engine/trace");
    assert_eq!(config.telemetry.ttl_secs, 86_400);
    assert_eq!(config.telemetry.max_bytes, 10_485_760);
    assert_eq!(config.telemetry.max_buffer_events, 1000);
    for field in asep::config::settings::MANDATORY_EVIDENCE_FIELDS {
        assert!(config.telemetry.allowlist.iter().any(|name| name == field));
    }
    assert!(config.telemetry.redact_patterns.is_empty());
}

// @test:IN-01-OVERHEAD
#[test]
fn test_telemetry_round_trip_toml() {
    let tmp = tempdir().unwrap();
    let mut config = EngineConfig::default();
    config.telemetry.enabled = true;
    config.telemetry.allowlist = vec!["span".to_string(), "event".to_string()];
    config.telemetry.redact_patterns = vec!["token".to_string()];
    config.telemetry.ttl_secs = 3600;
    config.telemetry.max_bytes = 5242880;
    config.telemetry.max_buffer_events = 5000;
    config.telemetry.dir = ".spec-engine/custom-trace".to_string();

    save_config(tmp.path(), &config).expect("Config save must succeed");

    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.telemetry.enabled, true);
    assert_eq!(reloaded.telemetry.allowlist, vec!["span", "event"]);
    assert_eq!(reloaded.telemetry.redact_patterns, vec!["token"]);
    assert_eq!(reloaded.telemetry.ttl_secs, 3600);
    assert_eq!(reloaded.telemetry.max_bytes, 5242880);
    assert_eq!(reloaded.telemetry.max_buffer_events, 5000);
    assert_eq!(reloaded.telemetry.dir, ".spec-engine/custom-trace");
}

// @test:IN-01-OVERHEAD
#[test]
fn test_telemetry_enabled_round_trip() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join(".spec-engine");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), "[telemetry]\nenabled = true\n").unwrap();

    let config = load_config(tmp.path());
    assert!(config.telemetry.enabled);
    assert_eq!(config.telemetry.dir, ".spec-engine/trace");
}

// @test:FR-LEVEL-02
#[test]
fn test_engineer_level_default_and_round_trip() {
    let profile = CapabilityProfile::default();
    assert_eq!(profile.engineer_level, EngineerLevel::L1);

    let tmp = tempdir().unwrap();
    let mut config = EngineConfig::default();
    config.capabilities.engineer_level = EngineerLevel::L3;
    save_config(tmp.path(), &config).expect("save");
    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.capabilities.engineer_level, EngineerLevel::L3);

    // absent field defaults to L1
    let dir = tmp.path().join(".spec-engine");
    fs::write(
        dir.join("config.toml"),
        "[capabilities]\nfile_read = true\n",
    )
    .unwrap();
    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.capabilities.engineer_level, EngineerLevel::L1);
}
