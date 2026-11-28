//! Skill search and discovery.

use crate::skill::distribution::embedded::get_embedded_skills;
use crate::skill::inspector::inspect_skills;
use crate::skill::model::SkillEntry;
use std::collections::HashSet;
use std::path::Path;

/// Searches for skills matching a query across workspace and embedded catalogs.
pub fn search_skills(skills_dir: &Path, query: &str) -> Vec<SkillEntry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Search local workspace skills
    let local = inspect_skills(skills_dir);
    for entry in local {
        let id_match = entry.id.to_lowercase().contains(&q);
        let name_match = entry.name.to_lowercase().contains(&q);
        let desc_match = entry.description.to_lowercase().contains(&q);
        let deps_match = entry.depends.iter().any(|d| d.to_lowercase().contains(&q));

        if id_match || name_match || desc_match || deps_match {
            seen_ids.insert(entry.id.clone());
            matches.push(entry);
        }
    }

    // 2. Search embedded catalog for any that were not seen
    let embedded = get_embedded_skills();
    for orig in embedded {
        if seen_ids.contains(orig.id) {
            continue;
        }
        let (name, desc, version, depends) =
            crate::skill::inspector::extract_skill_full_metadata(orig.content);
        let id_match = orig.id.to_lowercase().contains(&q);
        let name_match = name.to_lowercase().contains(&q);
        let desc_match = desc.to_lowercase().contains(&q);
        let deps_match = depends.iter().any(|d| d.to_lowercase().contains(&q));

        if !(id_match || name_match || desc_match || deps_match) {
            continue;
        }
        let mut skill_name = name;
        if skill_name.is_empty() {
            skill_name = orig.id.to_string();
        }
        matches.push(SkillEntry {
            id: orig.id.to_string(),
            name: skill_name,
            description: desc,
            file_path: skills_dir.join(orig.directory).join("SKILL.md"),
            status: crate::skill::model::SkillStatus::Missing,
            version,
            depends,
        });
    }

    matches.sort_by(|a, b| a.id.cmp(&b.id));
    matches
}
