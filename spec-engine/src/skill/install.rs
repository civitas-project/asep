//! Dependency resolution and skill installation.

use crate::pack::embedded::get_embedded_skills;
use crate::skill::inspector::{extract_skill_full_metadata, inspect_skills};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves transitive dependencies for a skill id.
pub fn resolve_dependencies(skill_id: &str) -> Vec<String> {
    let embedded = get_embedded_skills();
    let mut resolved = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(skill_id.to_string());
    visited.insert(skill_id.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(orig) = embedded.iter().find(|e| e.id == current || e.directory == current) {
            let (_, _, _, deps) = extract_skill_full_metadata(orig.content);
            for dep in deps {
                if !visited.contains(&dep) {
                    visited.insert(dep.clone());
                    resolved.push(dep.clone());
                    queue.push_back(dep);
                }
            }
        }
    }

    resolved
}

/// Installs a skill and optionally its transitive dependencies into the skills directory.
pub fn install_skill(
    skills_dir: &Path,
    skill_id: &str,
    with_deps: bool,
) -> Result<Vec<PathBuf>, String> {
    let embedded = get_embedded_skills();
    let mut to_install = vec![skill_id.to_string()];

    if with_deps {
        let deps = resolve_dependencies(skill_id);
        to_install.extend(deps);
    }

    let mut installed_paths = Vec::new();
    let local_entries = inspect_skills(skills_dir);

    for id in to_install {
        // Skip if already present and not missing
        if local_entries
            .iter()
            .any(|e| e.id == id && e.status != crate::skill::model::SkillStatus::Missing)
        {
            continue;
        }

        let orig = embedded
            .iter()
            .find(|e| e.id == id || e.directory == id)
            .ok_or_else(|| format!("skill '{}' not found in embedded catalog", id))?;

        let skill_folder = skills_dir.join(orig.directory);
        fs::create_dir_all(&skill_folder).map_err(|e| e.to_string())?;
        let skill_file = skill_folder.join("SKILL.md");
        fs::write(&skill_file, orig.content).map_err(|e| e.to_string())?;
        installed_paths.push(skill_file);
    }

    Ok(installed_paths)
}
