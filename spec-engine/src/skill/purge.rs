//! Skill purge and cascade uninstallation across agent directories.

use crate::pack::targets::{resolve_target_paths, InstallScope, TargetAgent};
use crate::skill::state::{load_state, save_state};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PurgeReport {
    pub id: String,
    pub workspace_removed: bool,
    pub agents_cleaned: Vec<String>,
}

/// Purges a skill from local workspace and cascades deletion to tracked agent targets.
pub fn purge_skill(
    skills_dir: &Path,
    id: &str,
    specific_agents: Option<&[TargetAgent]>,
    scope: &InstallScope,
) -> Result<PurgeReport, std::io::Error> {
    let mut report = PurgeReport {
        id: id.to_string(),
        workspace_removed: false,
        agents_cleaned: Vec::new(),
    };

    // 1. Remove workspace skill folder
    let workspace_folder = skills_dir.join(id);
    if workspace_folder.exists() {
        fs::remove_dir_all(&workspace_folder)?;
        report.workspace_removed = true;
    }

    // 2. Cascade remove from agent targets
    let state = load_state(skills_dir);
    let tracked_agents: Vec<String> = state
        .as_ref()
        .and_then(|s| s.skills.iter().find(|e| e.id == id).map(|e| e.agents.clone()))
        .unwrap_or_default();

    let all_agents = [
        TargetAgent::Antigravity,
        TargetAgent::OpenCode,
        TargetAgent::ClaudeCode,
        TargetAgent::Cursor,
        TargetAgent::Windsurf,
        TargetAgent::Continue,
        TargetAgent::RooCode,
        TargetAgent::Copilot,
        TargetAgent::Goose,
        TargetAgent::Zed,
        TargetAgent::Aider,
    ];

    let mut agents_to_clean: Vec<TargetAgent> = all_agents
        .iter()
        .copied()
        .filter(|a| tracked_agents.iter().any(|i| i.as_str() == a.id()))
        .collect();
    if let Some(spec) = specific_agents {
        agents_to_clean = spec.to_vec();
    }

    for agent in agents_to_clean {
        let base_target = resolve_target_paths(scope, agent);
        let agent_skill_folder = base_target.join(id);
        if agent_skill_folder.exists() {
            let _ = fs::remove_dir_all(&agent_skill_folder);
            report.agents_cleaned.push(agent.id().to_string());
        }
    }

    // 3. Remove entry from state.json
    if let Some(mut s) = state {
        s.skills.retain(|e| e.id != id);
        let _ = save_state(skills_dir, &s);
    }

    // 4. Update skills.lock if present
    let lock_file = skills_dir.join("skills.lock");
    if lock_file.exists() {
        let _ = crate::skill::lock::generate_lock(skills_dir);
    }

    Ok(report)
}
