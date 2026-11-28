//! Fork an existing skill into a new custom skill.

use crate::skill::distribution::embedded::get_embedded_skills;
use crate::skill::inspector::inspect_skills;
use std::fs;
use std::path::{Path, PathBuf};

/// Forks an existing skill (workspace or embedded) into a new custom skill directory.
pub fn fork_skill(
    skills_dir: &Path,
    src_id: &str,
    new_id: &str,
    new_name: &str,
    new_desc: &str,
) -> Result<PathBuf, String> {
    let clean_new_id = new_id.trim().to_lowercase().replace(' ', "-");
    let target_folder = skills_dir.join(&clean_new_id);
    if target_folder.exists() {
        return Err(format!("target skill '{}' already exists", clean_new_id));
    }

    // Locate source content
    let entries = inspect_skills(skills_dir);
    let mut source_content = entries
        .iter()
        .find(|e| e.id == src_id)
        .filter(|e| e.file_path.exists())
        .and_then(|e| fs::read_to_string(&e.file_path).ok());

    if source_content.is_none() {
        source_content = get_embedded_skills()
            .iter()
            .find(|e| e.id == src_id || e.directory == src_id)
            .map(|e| e.content.to_string());
    }

    let source = source_content.ok_or_else(|| format!("source skill '{}' not found", src_id))?;

    // Extract body (after frontmatter)
    let body = source
        .strip_prefix("---")
        .and_then(|rest| rest.find("\n---").map(|end_idx| rest[end_idx + 4..].trim()))
        .unwrap_or_else(|| source.trim());

    let new_content = format!(
        "---\nname: {}\ndescription: {}\nversion: 0.1.0\ndepends: []\n---\n\n{}\n",
        new_name.trim(),
        new_desc.trim(),
        body
    );

    fs::create_dir_all(&target_folder).map_err(|e| e.to_string())?;
    let new_skill_file = target_folder.join("SKILL.md");
    fs::write(&new_skill_file, new_content).map_err(|e| e.to_string())?;

    Ok(new_skill_file)
}
