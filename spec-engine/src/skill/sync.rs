//! Skill sync: propagate skills to all detected agentic AI tools.

use crate::pack::targets::{resolve_target_paths, InstallScope, TargetAgent};
use crate::skill::inspector::inspect_skills;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct SyncReport {
    pub synced: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Sync all skills from skills_dir to the given agents.
pub fn sync_skills(
    skills_dir: &Path,
    scope: &InstallScope,
    agents: &[TargetAgent],
) -> SyncReport {
    let entries = inspect_skills(skills_dir);
    let mut report = SyncReport {
        synced: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    for agent in agents {
        let target = resolve_target_paths(scope, *agent);
        // Ensure target dir exists
        if let Err(e) = fs::create_dir_all(&target) {
            report.errors.push(format!("{}: {}", agent.display_name(), e));
            continue;
        }

        for entry in &entries {
            if entry.status == crate::skill::model::SkillStatus::Missing {
                report.skipped += 1;
                continue;
            }
            if !entry.file_path.exists() {
                report.skipped += 1;
                continue;
            }
            let content = match fs::read_to_string(&entry.file_path) {
                Ok(c) => c,
                Err(e) => {
                    report.errors.push(format!("read {}: {}", entry.id, e));
                    continue;
                }
            };

            // Determine target file path based on agent format
            let res = match agent {
                TargetAgent::OpenCode
                | TargetAgent::Antigravity
                | TargetAgent::Goose
                | TargetAgent::ClaudeCode => {
                    let s_dir = target.join(&entry.id);
                    match fs::create_dir_all(&s_dir) {
                        Ok(()) => fs::write(s_dir.join("SKILL.md"), &content).map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    }
                }
                TargetAgent::Cursor => {
                    fs::write(target.join(format!("{}.mdc", entry.id)), &content)
                        .map_err(|e| e.to_string())
                }
                _ => {
                    fs::write(target.join(format!("{}.md", entry.id)), &content)
                        .map_err(|e| e.to_string())
                }
            };

            match res {
                Ok(_) => {
                    report.synced += 1;
                    let _ = crate::skill::state::record_agent_install(skills_dir, &entry.id, agent);
                }
                Err(e) => report.errors.push(format!("{} -> {}: {}", entry.id, agent.display_name(), e)),
            }
        }
    }

    report
}