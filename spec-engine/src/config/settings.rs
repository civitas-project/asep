//! Persistent TOML configuration manager for spec-engine.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// @impl:FR-CONFIG-LOAD-01
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineConfig {
    #[serde(default)]
    pub editor: EditorSettings,
}

/// Resolves the canonical path to `config.toml` in target directory (`.spec-engine/config.toml`).
pub fn get_config_path(target_dir: &Path) -> PathBuf {
    if target_dir.ends_with(".spec-engine") {
        target_dir.join("config.toml")
    } else {
        target_dir.join(".spec-engine").join("config.toml")
    }
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
