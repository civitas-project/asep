//! Skill state tracking: records installed skills across agents.

use crate::skill::distribution::targets::TargetAgent;
use crate::skill::inspector::inspect_skills;
use crate::skill::lock::sha256_file;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStateEntry {
    pub id: String,
    pub version: Option<String>,
    pub hash: String,
    pub status: String,
    pub installed_at: String,
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillState {
    pub generated_at: String,
    pub asep_version: String,
    pub skills: Vec<SkillStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBundle {
    pub exported_at: String,
    pub asep_version: String,
    pub state: SkillState,
    pub skills: Vec<SkillBundleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBundleEntry {
    pub id: String,
    pub content: String,
}

const STATE_FILENAME: &str = ".asep/state.json";

fn state_path(skills_dir: &Path) -> PathBuf {
    skills_dir.join(STATE_FILENAME)
}

pub fn load_state(skills_dir: &Path) -> Option<SkillState> {
    let path = state_path(skills_dir);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_state(skills_dir: &Path, state: &SkillState) -> Result<(), std::io::Error> {
    let path = state_path(skills_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json =
        serde_json::to_string_pretty(state).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, json)?;
    Ok(())
}

pub fn rebuild_state(skills_dir: &Path) -> SkillState {
    let entries = inspect_skills(skills_dir);
    let mut skills = Vec::new();
    for e in &entries {
        if e.status == crate::skill::model::SkillStatus::Missing {
            continue;
        }
        let mut hash = "missing".to_string();
        if e.file_path.exists() {
            hash = sha256_file(&e.file_path).unwrap_or_else(|_| "missing".to_string());
        }
        skills.push(SkillStateEntry {
            id: e.id.clone(),
            version: e.version.clone(),
            hash,
            status: e.status.to_string(),
            installed_at: timestamp_now(),
            agents: Vec::new(),
        });
    }
    SkillState {
        generated_at: timestamp_now(),
        asep_version: "2.1.0".to_string(),
        skills,
    }
}

/// Update agent tracking for a skill after sync/install.
pub fn record_agent_install(
    skills_dir: &Path,
    skill_id: &str,
    agent: &TargetAgent,
) -> Result<(), std::io::Error> {
    let mut state = load_state(skills_dir).unwrap_or_else(|| rebuild_state(skills_dir));
    let agent_name = agent.id().to_string();

    let tracked_index = state.skills.iter().position(|e| e.id == skill_id);
    let already_has_agent = tracked_index
        .map(|i| state.skills[i].agents.contains(&agent_name))
        .unwrap_or(false);

    // Skill already tracked and agent recorded: refresh timestamp only.
    if tracked_index.is_some() && already_has_agent {
        state.generated_at = timestamp_now();
        return save_state(skills_dir, &state);
    }
    // Skill already tracked but agent not recorded: add the agent.
    if let Some(i) = tracked_index {
        state.skills[i].agents.push(agent_name);
        state.generated_at = timestamp_now();
        return save_state(skills_dir, &state);
    }

    // Skill exists on disk but not yet in state
    let entries = inspect_skills(skills_dir);
    let disk_entry = entries.iter().find(|e| e.id == skill_id);
    let disk_hash = disk_entry.map_or_else(
        || "missing".to_string(),
        |e| sha256_file(&e.file_path).unwrap_or_else(|_| "missing".to_string()),
    );
    if let Some(e) = disk_entry {
        let version = e.version.clone();
        let status = e.status.to_string();
        state.skills.push(SkillStateEntry {
            id: skill_id.to_string(),
            version,
            hash: disk_hash,
            status,
            installed_at: timestamp_now(),
            agents: vec![agent_name],
        });
    }
    state.generated_at = timestamp_now();
    save_state(skills_dir, &state)
}

/// Get agents that have a skill installed.
pub fn get_skill_agents(skills_dir: &Path, skill_id: &str) -> Vec<String> {
    load_state(skills_dir)
        .and_then(|s| {
            s.skills
                .iter()
                .find(|e| e.id == skill_id)
                .map(|e| e.agents.clone())
        })
        .unwrap_or_default()
}

#[derive(Debug)]
pub struct StateDrift {
    pub id: String,
    pub kind: String,
    pub detail: String,
}

pub fn check_drift(skills_dir: &Path) -> Result<Vec<StateDrift>, std::io::Error> {
    let state = match load_state(skills_dir) {
        Some(s) => s,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no state file found",
            ))
        }
    };
    let entries = inspect_skills(skills_dir);
    let mut drifts = Vec::new();

    for se in &state.skills {
        let disk_entry = entries.iter().find(|e| e.id == se.id);
        match disk_entry {
            None => {
                drifts.push(StateDrift {
                    id: se.id.clone(),
                    kind: "removed".to_string(),
                    detail: "skill no longer present".to_string(),
                });
                continue;
            }
            Some(e) => {
                if e.file_path.exists() && sha256_file(&e.file_path).unwrap_or_default() != se.hash
                {
                    drifts.push(StateDrift {
                        id: se.id.clone(),
                        kind: "modified".to_string(),
                        detail: "content changed since last state snapshot".to_string(),
                    });
                }
                if !e.file_path.exists() {
                    drifts.push(StateDrift {
                        id: se.id.clone(),
                        kind: "missing".to_string(),
                        detail: "file removed".to_string(),
                    });
                }
                if e.version != se.version {
                    drifts.push(StateDrift {
                        id: se.id.clone(),
                        kind: "version_changed".to_string(),
                        detail: format!("{:?} -> {:?}", se.version, e.version),
                    });
                }
            }
        }
    }

    for e in &entries {
        if !state.skills.iter().any(|se| se.id == e.id) {
            drifts.push(StateDrift {
                id: e.id.clone(),
                kind: "new".to_string(),
                detail: "skill not tracked in state".to_string(),
            });
        }
    }

    Ok(drifts)
}

/// Export skills and state to a bundle file for cross-device sync.
pub fn export_bundle(skills_dir: &Path, out_path: &Path) -> Result<SkillBundle, std::io::Error> {
    let state = load_state(skills_dir).unwrap_or_else(|| rebuild_state(skills_dir));
    let entries = inspect_skills(skills_dir);
    let mut skills = Vec::new();

    for e in &entries {
        if e.status == crate::skill::model::SkillStatus::Missing {
            continue;
        }
        let content = fs::read_to_string(&e.file_path).unwrap_or_default();
        skills.push(SkillBundleEntry {
            id: e.id.clone(),
            content,
        });
    }

    let bundle = SkillBundle {
        exported_at: timestamp_now(),
        asep_version: "2.1.0".to_string(),
        state,
        skills,
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json =
        serde_json::to_string_pretty(&bundle).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(out_path, json)?;
    Ok(bundle)
}

/// Import skills and state from a bundle file.
pub fn import_bundle(skills_dir: &Path, bundle_path: &Path) -> Result<SkillBundle, std::io::Error> {
    let content = fs::read_to_string(bundle_path)?;
    let bundle: SkillBundle = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    // Write skill files
    for skill in &bundle.skills {
        let skill_dir = skills_dir.join(&skill.id);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), &skill.content)?;
    }

    // Save state
    save_state(skills_dir, &bundle.state)?;

    Ok(bundle)
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}

// Re-export for convenience
pub use SkillState as State;
