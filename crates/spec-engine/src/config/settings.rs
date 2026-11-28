// @impl:FR-ACC-05
//! Persistent TOML configuration manager for spec-engine.
// @impl:FR-CAP-01 // @impl:FR-CAP-03 // @impl:FR-CAP-04 // @impl:FR-CAP-08
// @impl:FR-LEVEL-02

use crate::policy::EngineerLevel;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// @impl:FR-CONFIG-LOAD-01 @impl:FR-CONFIG-01
// @impl:FR-CONFIG-LOAD-02
// @impl:FR-CONFIG-SAVE-01

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorPreference {
    #[default]
    Ask,
    Terminal,
    Gui,
}

impl std::fmt::Display for EditorPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorPreference::Ask => write!(f, "Always Ask (dialog prompt)"),
            EditorPreference::Terminal => write!(f, "Terminal Editor ($EDITOR / nano / vim)"),
            EditorPreference::Gui => {
                write!(f, "System Default / GUI (VS Code / Cursor / OS default)")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    #[serde(default)]
    pub preference: EditorPreference,
    #[serde(default)]
    pub custom_command: String,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            preference: EditorPreference::Ask,
            custom_command: String::new(),
        }
    }
}

// @impl:IN-01-OVERHEAD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    pub allowlist: Vec<String>,
    pub redact_patterns: Vec<String>,
    pub ttl_secs: u64,
    pub max_bytes: u64,
    #[serde(default)]
    pub max_buffer_events: usize,
    pub dir: String,
}

pub const MANDATORY_EVIDENCE_FIELDS: [&str; 6] = [
    "schema_version",
    "trace_id",
    "task_id",
    "event",
    "seq",
    "detail",
];

// @impl:FR-AUD-17
impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowlist: MANDATORY_EVIDENCE_FIELDS.map(ToString::to_string).to_vec(),
            redact_patterns: Vec::new(),
            ttl_secs: 86_400,
            max_bytes: 10_485_760,
            max_buffer_events: 1000,
            dir: ".spec-engine/trace".to_string(),
        }
    }
}

/// Canonical capability names. A required capability outside this registry is
/// blocked, never silently ignored.
pub const CAPABILITIES: [&str; 6] = [
    "file_read",
    "file_write",
    "terminal",
    "mcp",
    "subagents",
    "web",
];

/// Agent capability profile: fail-closed ceiling applied to scaffolded
/// OpenCode agent permissions. The all-enabled default preserves fresh-install
/// scaffold behavior; it is a declared policy, not an inference about native
/// runtime limits. No fixed window or delegation threshold is assumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct CapabilityProfile {
    pub file_read: bool,
    pub file_write: bool,
    pub terminal: bool,
    pub mcp: bool,
    pub subagents: bool,
    pub web: bool,
    pub supported_media: Vec<String>,
    pub tool_domains: Vec<String>,
    #[serde(default)]
    pub engineer_level: EngineerLevel,
}

impl Default for CapabilityProfile {
    fn default() -> Self {
        Self {
            file_read: true,
            file_write: true,
            terminal: true,
            mcp: true,
            subagents: true,
            web: true,
            supported_media: vec!["text".to_string(), "code".to_string()],
            tool_domains: vec!["code".to_string()],
            engineer_level: EngineerLevel::default(),
        }
    }
}

impl CapabilityProfile {
    /// Fail-closed profile used when a persistent config exists but cannot be
    /// read or parsed. No capability is granted; callers must opt in explicitly
    /// with a valid config.
    pub fn restrictive() -> Self {
        Self {
            file_read: false,
            file_write: false,
            terminal: false,
            mcp: false,
            subagents: false,
            web: false,
            supported_media: Vec::new(),
            tool_domains: Vec::new(),
            engineer_level: EngineerLevel::default(),
        }
    }

    /// Returns the names from `required` that are disabled on this profile,
    /// preserving the order of `required`. Unknown capability names are
    /// blocking: an unrecognised requirement can never be treated as satisfied.
    pub fn missing(&self, required: &[&str]) -> Vec<String> {
        required
            .iter()
            .filter(|name| {
                if !CAPABILITIES.contains(name) {
                    return true;
                }
                match **name {
                    "file_read" => !self.file_read,
                    "file_write" => !self.file_write,
                    "terminal" => !self.terminal,
                    "mcp" => !self.mcp,
                    "subagents" => !self.subagents,
                    "web" => !self.web,
                    _ => true,
                }
            })
            .map(|name| name.to_string())
            .collect()
    }

    /// Returns true if `media` is empty or present in `supported_media`
    /// (case-sensitive match).
    pub fn supports_media(&self, media: &str) -> bool {
        media.is_empty() || self.supported_media.iter().any(|m| m == media)
    }

    /// Checks required capabilities via `missing()`, then optional media.
    /// Returns Err describing the first blocking requirement, else Ok(()).
    pub fn covers(&self, required: &[&str], media: Option<&str>) -> Result<(), String> {
        if let Some(first) = self.missing(required).into_iter().next() {
            return Err(format!("BLOCKED: requires {first}"));
        }
        let media = match media.filter(|m| !m.is_empty()) {
            Some(media) => media,
            None => return Ok(()),
        };
        if !self.supports_media(media) {
            return Err(format!("BLOCKED: requires media:{media}"));
        }
        Ok(())
    }
}

// @impl:FR-AUD-21

/// Validates declared capability configuration. Unknown keys or malformed
/// TOML are rejected (fail closed). A valid profile is returned unchanged.
pub fn validate_capabilities(text: &str) -> Result<CapabilityProfile, String> {
    toml::from_str(text).map_err(|err| err.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    #[serde(default)]
    pub editor: EditorSettings,

    #[serde(default)]
    pub rigor: Option<String>,

    #[serde(default)]
    pub capabilities: CapabilityProfile,

    #[serde(default)]
    pub evidence_dir: Option<String>,

    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

/// Resolves the canonical path to `config.toml` in target directory (`.spec-engine/config.toml`).
pub fn get_config_path(target_dir: &Path) -> PathBuf {
    if !target_dir.ends_with(".spec-engine") {
        return target_dir.join(".spec-engine").join("config.toml");
    }
    target_dir.join("config.toml")
}

/// Loads the persistent configuration from disk.
///
/// Missing config returns engine defaults (fresh-install behavior). A config
/// that exists but is unreadable or malformed returns a fail-closed engine
/// config with no capabilities granted, so invalid policy can never widen
/// permissions. Use `validate_capabilities` to surface a parse error explicitly.
pub fn load_config(target_dir: &Path) -> EngineConfig {
    load_config_checked(target_dir).unwrap_or_else(|_| EngineConfig {
        capabilities: CapabilityProfile::restrictive(),
        ..EngineConfig::default()
    })
}

/// Loads persistent configuration with explicit errors. Missing config returns
/// engine defaults for fresh installs. An inaccessible path, a non-file, an
/// unreadable file, malformed TOML or an unknown key is an error, never a
/// silent widening or narrowing of policy. Enforcement call sites should use
/// this variant and surface the error instead of relying on the restricted
/// fallback in `load_config`.
pub fn load_config_checked(target_dir: &Path) -> Result<EngineConfig, String> {
    let path = get_config_path(target_dir);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(format!("config path is not a file: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(EngineConfig::default()),
        Err(e) => return Err(format!("cannot access config {}: {e}", path.display())),
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("invalid config {}: {e}", path.display()))
}

/// Saves the persistent configuration to `config.toml`.
pub fn save_config(target_dir: &Path, config: &EngineConfig) -> Result<(), std::io::Error> {
    let path = get_config_path(target_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml_str = toml::to_string_pretty(config).map_err(std::io::Error::other)?;

    fs::write(path, toml_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // @test:FR-AUD-17
    fn default_allowlist_contains_mandatory_fields() {
        let config = TelemetryConfig::default();
        for field in MANDATORY_EVIDENCE_FIELDS {
            assert!(
                config.allowlist.iter().any(|name| name == field),
                "missing {field}"
            );
        }
        assert_eq!(config.allowlist.len(), MANDATORY_EVIDENCE_FIELDS.len());
    }

    #[test]
    // @test:FR-AUD-21
    fn unknown_capability_is_rejected() {
        let err = validate_capabilities("unknown_capability = true\n").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn malformed_toml_is_rejected() {
        assert!(validate_capabilities("[[unclosed\n").is_err());
    }

    #[test]
    // @test:R02
    fn unknown_required_capability_is_blocking() {
        let profile = CapabilityProfile::default();
        assert_eq!(
            profile.missing(&["file_read", "not_a_capability"]),
            vec!["not_a_capability".to_string()]
        );
        assert!(profile
            .covers(&["not_a_capability"], None)
            .unwrap_err()
            .contains("not_a_capability"));
    }

    #[test]
    // @test:R02
    fn checked_loader_distinguishes_missing_malformed_and_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_config_checked(dir.path()).is_ok(), "missing config is allowed");

        let path = get_config_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "[[unclosed\n").unwrap();
        assert!(load_config_checked(dir.path()).is_err(), "malformed config must error");

        fs::write(&path, "unknown_section = 1\n").unwrap();
        assert!(load_config_checked(dir.path()).is_err(), "unknown key must error");

        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(load_config_checked(dir.path()).is_err(), "non-file config must error");
    }

    #[test]
    fn valid_capability_profile_parses() {
        let profile = validate_capabilities("file_read = false\n").unwrap();
        assert!(!profile.file_read);
    }

    // Regression: a malformed persistent config must not widen permissions.
    #[test]
    fn malformed_config_file_does_not_grant_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_config_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "[[unclosed\n").unwrap();
        let config = load_config(dir.path());
        assert!(
            config.capabilities.covers(&["file_write", "web"], None).is_err(),
            "malformed config silently granted permissions"
        );
    }

    // Regression: an unreadable persistent config must not widen permissions.
    #[test]
    fn unreadable_config_file_does_not_grant_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_config_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir(&path).unwrap();
        let config = load_config(dir.path());
        assert!(
            config.capabilities.covers(&["file_write", "web"], None).is_err(),
            "unreadable config silently granted permissions"
        );
    }
}
