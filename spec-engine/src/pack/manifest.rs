//! Manifest generation and configuration merging for Antigravity and OpenCode.

use std::fs;
use std::path::Path;

// @impl:FR-PACK-MANIFEST-01
// @impl:FR-PACK-MANIFEST-02

/// Generates the canonical `plugin.json` manifest for Antigravity plugins.
pub fn generate_plugin_json() -> String {
    serde_json::json!({
        "name": "asep",
        "description": "Axiomatic Systems Engineering Protocol (ASEP): Deterministic SDLC validator, traceability graph checker, and living spec engine"
    })
    .to_string()
}

/// Generates the canonical `mcp_config.json` manifest.
pub fn generate_mcp_config_json(command: &str) -> String {
    serde_json::json!({
        "mcpServers": {
            "asep": {
                "command": command,
                "args": ["mcp"]
            }
        }
    })
    .to_string()
}

/// Merges `asep` MCP configuration into an existing or new `opencode.json`.
///
/// Safety: reads and parses the existing file first; if parsing fails, the
/// original content is preserved in a `.bak` file and an error is returned
/// instead of clobbering the user's configuration. A backup is also written
/// before any successful write.
pub fn update_opencode_json(config_path: &Path, command: &str) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        String::new()
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = if existing_raw.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => v,
            Err(e) => {
                // Preserve the user's file verbatim in a backup.
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, &existing_raw);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "existing opencode.json is not valid JSON ({}); backup written to {}",
                        e,
                        backup_path.display()
                    ),
                ));
            }
        }
    };

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Preserve any pre-existing mcp.servers (and other keys) by only touching
    // the "asep" entry under mcp.
    if config.get("mcp").is_none() || !config["mcp"].is_object() {
        config["mcp"] = serde_json::json!({});
    }

    config["mcp"]["asep"] = serde_json::json!({
        "type": "local",
        "command": command,
        "args": ["mcp"],
        "enabled": true
    });

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, &existing_raw);
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;

    // Sanity check: the merged result must still contain any previously
    // present top-level keys (e.g. "provider", "model", "plugins").
    if !existing_raw.trim().is_empty() {
        if let Ok(prev) = serde_json::from_str::<serde_json::Value>(&existing_raw) {
            if let Some(prev_obj) = prev.as_object() {
                if let Some(new_obj) = config.as_object() {
                    for key in prev_obj.keys() {
                        if !new_obj.contains_key(key) {
                            // Should not happen, but guard against silent loss.
                            let backup_path = config_path.with_extension("json.bak");
                            let _ = fs::write(&backup_path, &existing_raw);
                            return Err(std::io::Error::other(format!(
                                "merge would drop top-level key '{}'; aborted, backup at {}",
                                key,
                                backup_path.display()
                            )));
                        }
                    }
                }
            }
        }
    }

    fs::write(config_path, formatted)?;
    Ok(())
}

/// Installs default Lead-Judge-Executor agent presets into `opencode.json`
/// without clobbering pre-existing top-level keys or pre-existing agents.
pub fn scaffold_opencode_agents(config_path: &Path) -> Result<(), std::io::Error> {
    // Read existing content (if any) without losing it.
    let existing_raw = if config_path.exists() {
        fs::read_to_string(config_path)?
    } else {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        String::new()
    };

    // Parse existing JSON. If the file is non-empty but unparseable, back up
    // the raw content and return an error instead of overwriting.
    let mut config: serde_json::Value = if existing_raw.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&existing_raw) {
            Ok(v) => v,
            Err(e) => {
                let backup_path = config_path.with_extension("json.bak");
                let _ = fs::write(&backup_path, &existing_raw);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "existing opencode.json is not valid JSON ({}); backup written to {}",
                        e,
                        backup_path.display()
                    ),
                ));
            }
        }
    };

    if !config.is_object() {
        config = serde_json::json!({});
    }

    // Ensure the "agents" object exists, then insert only missing defaults.
    if !config.get("agents").is_some_and(|v| v.is_object()) {
        config["agents"] = serde_json::json!({});
    }

    let default_agents = serde_json::json!({
        "lead": {
            "description": "Orchestrator for the bounded executor-judge workflow.",
            "mode": "primary"
        },
        "executor": {
            "description": "Fast coding executor in a bounded executor-judge pipeline.",
            "mode": "subagent"
        },
        "judge": {
            "description": "Read-only reviewer verifying executor output against task, diff, and test evidence.",
            "mode": "subagent"
        },
        "explorer": {
            "description": "Fast explorer subagent for codebase exploration.",
            "mode": "subagent"
        }
    });

    let agents_obj = config
        .get_mut("agents")
        .expect("agents object was just ensured")
        .as_object_mut()
        .expect("agents value is an object");
    for (key, value) in default_agents.as_object().expect("default_agents is an object") {
        agents_obj.entry(key.clone()).or_insert(value.clone());
    }

    // Back up the original content before overwriting (best effort).
    if !existing_raw.is_empty() {
        let backup_path = config_path.with_extension("json.bak");
        let _ = fs::write(&backup_path, &existing_raw);
    }

    let formatted = serde_json::to_string_pretty(&config).map_err(std::io::Error::other)?;
    fs::write(config_path, formatted)?;
    Ok(())
}
