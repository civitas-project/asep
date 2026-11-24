//! Skill rollback: backup before edit, restore from backup or embedded original.

use crate::pack::embedded::get_embedded_skills;
use crate::skill::inspector::inspect_skills;
use std::fs;
use std::path::{Path, PathBuf};

const BACKUP_ROOT: &str = ".asep/backups";

fn backup_dir(skills_dir: &Path, id: &str) -> PathBuf {
    skills_dir.join(BACKUP_ROOT).join(id)
}

pub fn list_backups(skills_dir: &Path, id: &str) -> Vec<String> {
    let dir = backup_dir(skills_dir, id);
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for name in entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        {
            out.push(name);
        }
    }
    out.sort();
    out.reverse();
    out
}

/// Create a backup of the current skill file before editing.
pub fn backup_skill(skills_dir: &Path, id: &str) -> Result<String, String> {
    let entries = inspect_skills(skills_dir);
    let entry = entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("skill '{}' not found", id))?;
    if entry.status == crate::skill::model::SkillStatus::Missing {
        return Err(format!("skill '{}' missing on disk", id));
    }
    let ts = timestamp_now();
    let dest_dir = backup_dir(skills_dir, id).join(&ts);
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    fs::copy(&entry.file_path, dest_dir.join("SKILL.md")).map_err(|e| e.to_string())?;
    Ok(ts)
}

#[allow(clippy::single_match)]
pub fn rollback_skill(skills_dir: &Path, id: &str, to: Option<&str>) -> Result<String, String> {
    let backups = list_backups(skills_dir, id);

    // If no backups and no explicit target, try embedded original
    if backups.is_empty() && to.is_none() {
        let embedded = get_embedded_skills();
        match embedded.iter().find(|e| e.id == id) {
            Some(orig) => {
                let entries = inspect_skills(skills_dir);
                let entry = entries.iter().find(|e| e.id == id);
                let file_path = entry
                    .map(|e| e.file_path.clone())
                    .unwrap_or_else(|| skills_dir.join(id).join("SKILL.md"));
                match file_path.parent() {
                    Some(parent) => fs::create_dir_all(parent).map_err(|e| e.to_string())?,
                    None => {}
                }
                fs::write(&file_path, orig.content).map_err(|e| e.to_string())?;
                return Ok(format!("restored '{}' from embedded original", id));
            }
            None => return Err(format!("no backups and no embedded original for '{}'", id)),
        }
    }

    let target_ts = match to {
        Some(t) => t.to_string(),
        None => backups
            .first()
            .cloned()
            .ok_or_else(|| "no backup available".to_string())?,
    };

    if target_ts.is_empty() {
        return Err("no backup timestamp specified and none available".to_string());
    }

    if target_ts.is_empty() {
        return Err("no backup timestamp specified and none available".to_string());
    }

    let backup_file = backup_dir(skills_dir, id).join(&target_ts).join("SKILL.md");
    if !backup_file.exists() {
        return Err(format!("backup '{}' not found", target_ts));
    }
    let content = fs::read_to_string(&backup_file).map_err(|e| e.to_string())?;

    let entries = inspect_skills(skills_dir);
    let entry = entries.iter().find(|e| e.id == id);
    let file_path = entry
        .map(|e| e.file_path.clone())
        .unwrap_or_else(|| skills_dir.join(id).join("SKILL.md"));
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&file_path, content).map_err(|e| e.to_string())?;
    Ok(format!("restored '{}' from backup {}", id, target_ts))
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
