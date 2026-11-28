//! Dependency resolution and skill installation.

use crate::skill::distribution::embedded::get_embedded_skills;
use crate::skill::inspector::{extract_skill_full_metadata, inspect_skills};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::Write;
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
        let orig = match embedded
            .iter()
            .find(|e| e.id == current || e.directory == current)
        {
            Some(orig) => orig,
            None => continue,
        };

        let (_, _, _, deps) = extract_skill_full_metadata(orig.content);
        for dep in deps {
            if visited.contains(&dep) {
                continue;
            }
            visited.insert(dep.clone());
            resolved.push(dep.clone());
            queue.push_back(dep);
        }
    }

    resolved
}

use sha2::{Digest, Sha256};

/// Stable location of recorded install digests.
fn managed_state_path(skills_dir: &Path, id: &str) -> PathBuf {
    skills_dir.join(".asep-managed").join(format!("{id}.json"))
}

fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

/// Reject ids/directories that could escape the skills directory.
fn safe_skill_name(name: &str) -> Result<&str, String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || name.starts_with('.')
    {
        return Err(format!("unsafe skill name: {name}"));
    }
    Ok(name)
}

/// Writes through a uniquely named sibling temp file and renames it into
/// place, so a failed write never leaves a partially replaced artifact.
fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("file has no parent directory")?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("file name is not UTF-8")?;
    for attempt in 0..10_000u32 {
        let temp = parent.join(format!(
            ".{name}.asep-{}-{attempt}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(content.as_bytes())
                    .and_then(|_| file.flush())
                    .map_err(|e| e.to_string())
                    .and_then(|_| fs::rename(&temp, path).map_err(|e| e.to_string()));
                if result.is_err() {
                    let _ = fs::remove_file(&temp);
                }
                return result;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("cannot create temp file: {e}")),
        }
    }
    Err("cannot reserve temp file".to_string())
}

/// Refreshes a skill only when the file still matches the digest recorded at
/// the last ASEP install. A managed marker is not proof of ownership: user
/// edits, missing state, unreadable files and conflicts are preserved.
pub fn refresh_managed_skill(
    skills_dir: &Path,
    skill: &crate::skill::distribution::embedded::EmbeddedSkill,
) -> Result<bool, String> {
    safe_skill_name(skill.id)?;
    safe_skill_name(skill.directory)?;
    let skill_file = skills_dir.join(skill.directory).join("SKILL.md");
    let existing = match fs::read_to_string(&skill_file) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(skill_file.parent().ok_or("invalid skill path")?)
                .map_err(|e| e.to_string())?;
            write_atomic(&skill_file, skill.content)?;
            let state = managed_state_path(skills_dir, skill.id);
            fs::create_dir_all(state.parent().ok_or("invalid state path")?)
                .map_err(|e| e.to_string())?;
            write_atomic(&state, &digest(skill.content))?;
            return Ok(true);
        }
        Err(e) => return Err(format!("unreadable skill {}: {e}", skill_file.display())),
    };

    let state = managed_state_path(skills_dir, skill.id);
    let recorded = fs::read_to_string(&state).ok();
    let marker = existing.contains("<!-- asep-managed-skill -->");
    let unchanged = recorded.as_deref() == Some(digest(&existing).as_str());
    if !marker || !unchanged {
        return Ok(false);
    }
    if existing == skill.content {
        return Ok(false);
    }
    write_atomic(&skill_file, skill.content)?;
    write_atomic(&state, &digest(skill.content))?;
    Ok(true)
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
        let orig = embedded
            .iter()
            .find(|e| e.id == id || e.directory == id)
            .ok_or_else(|| format!("skill '{}' not found in embedded catalog", id))?;
        if refresh_managed_skill(skills_dir, orig)? {
            installed_paths.push(skills_dir.join(orig.directory).join("SKILL.md"));
        }
    }

    Ok(installed_paths)
}
