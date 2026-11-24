//! Skill diff: compare local skill against embedded original.

use crate::pack::embedded::get_embedded_skills;
use crate::skill::inspector::inspect_skills;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

pub fn diff_skill(skills_dir: &Path, id: &str) -> Result<String, String> {
    let entries = inspect_skills(skills_dir);
    let entry = entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("skill '{}' not found", id))?;

    if entry.status == crate::skill::model::SkillStatus::Missing {
        return Err(format!("skill '{}' is missing on disk", id));
    }

    let local_content = fs::read_to_string(&entry.file_path)
        .map_err(|e| format!("failed to read local skill: {}", e))?;

    // Find embedded original
    let embedded = get_embedded_skills();
    let orig = embedded.iter().find(|e| e.id == id || e.directory == id);

    match orig {
        Some(o) => {
            let diff = TextDiff::from_lines(o.content, &local_content);
            let mut out = String::new();
            out.push_str(&format!(
                "--- embedded:{} (original)\n+++ local:{} ({})\n",
                id, id, entry.status
            ));
            for change in diff.iter_all_changes() {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                out.push_str(&format!("{}{}", prefix, change));
            }
            Ok(out)
        }
        None => {
            // Custom skill: no original to compare
            Ok(format!(
                "Skill '{}' is custom (no embedded original to compare against).\n---\n{}",
                id, local_content
            ))
        }
    }
}
