// @impl:FR-ACC-05
//! Persistent TOML configuration manager for spec-engine.
// @impl:FR-CAP-01 // @impl:FR-CAP-03 // @impl:FR-CAP-04 // @impl:FR-CAP-08
// @impl:FR-LEVEL-02

use crate::runtime::EngineerLevel;
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

/// Agent capability profile: fail-closed ceiling applied to scaffolded
/// OpenCode agent permissions. Default is all capabilities enabled with a
/// 200k context window, preserving existing scaffold behavior.
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
    pub context_window: usize,
    pub supported_media: Vec<String>,
    pub tool_domains: Vec<String>,
    pub delegation_threshold: usize,
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
            context_window: 200_000,
            supported_media: vec!["text".to_string(), "code".to_string()],
            tool_domains: vec!["code".to_string()],
            delegation_threshold: 3,
            engineer_level: EngineerLevel::default(),
        }
    }
}

impl CapabilityProfile {
    /// Returns the names from `required` that are disabled on this profile,
    /// preserving the order of `required`. Unknown names are ignored.
    pub fn missing(&self, required: &[&str]) -> Vec<String> {
        required
            .iter()
            .filter_map(|name| {
                let disabled = match *name {
                    "file_read" => !self.file_read,
                    "file_write" => !self.file_write,
                    "terminal" => !self.terminal,
                    "mcp" => !self.mcp,
                    "subagents" => !self.subagents,
                    "web" => !self.web,
                    _ => return None,
                };
                disabled.then_some(name.to_string())
            })
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

/// Loads the persistent configuration from disk or returns defaults if missing/corrupt.
pub fn load_config(target_dir: &Path) -> EngineConfig {
    let path = get_config_path(target_dir);
    if !path.exists() {
        return EngineConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|_| EngineConfig::default()),
        Err(_) => EngineConfig::default(),
    }
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
    fn valid_capability_profile_parses() {
        let profile = validate_capabilities("file_read = false\ncontext_window = 100\n").unwrap();
        assert!(!profile.file_read);
        assert_eq!(profile.context_window, 100);
    }
}
