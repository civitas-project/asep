use crate::pack::embedded::get_embedded_skills;
use std::fs;
use std::io;
use std::path::Path;

/// Restores specified built-in skills to their factory default content from binary assets.
pub fn restore_skills(skills_dir: &Path, skill_ids: &[&str]) -> Result<Vec<String>, io::Error> {
    let embedded = get_embedded_skills();
    let mut restored = Vec::new();

    for id in skill_ids {
        if let Some(skill) = embedded.iter().find(|s| s.id == *id) {
            let target_dir = skills_dir.join(skill.directory);
            fs::create_dir_all(&target_dir)?;
            let target_file = target_dir.join("SKILL.md");
            fs::write(&target_file, skill.content)?;
            restored.push(skill.id.to_string());
        }
    }

    Ok(restored)
}

/// Restores all built-in skills to factory defaults.
pub fn restore_all_builtin_skills(skills_dir: &Path) -> Result<Vec<String>, io::Error> {
    let embedded = get_embedded_skills();
    let ids: Vec<&str> = embedded.iter().map(|s| s.id).collect();
    restore_skills(skills_dir, &ids)
}
