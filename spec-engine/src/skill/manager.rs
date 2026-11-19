//! Skill CRUD operations and pure Markdown export/import.

use crate::skill::inspector::extract_skill_full_metadata;
use std::fs;
use std::path::{Path, PathBuf};

// @impl:FR-SKILL-CRUD-01 @impl:FR-SKILL-02
// @impl:FR-SKILL-PORT-01

/// Adds a new skill directory with frontmatter and body.
pub fn add_skill(
    skills_dir: &Path,
    id: &str,
    name: &str,
    description: &str,
    body: &str,
) -> Result<PathBuf, std::io::Error> {
    add_skill_full(skills_dir, id, name, description, body, None, &[])
}

/// Adds a new skill directory with full metadata (version, depends).
pub fn add_skill_full(
    skills_dir: &Path,
    id: &str,
    name: &str,
    description: &str,
    body: &str,
    version: Option<&str>,
    depends: &[&str],
) -> Result<PathBuf, std::io::Error> {
    let clean_id = id.trim().to_lowercase().replace(' ', "-");
    let skill_folder = skills_dir.join(&clean_id);
    fs::create_dir_all(&skill_folder)?;

    let version_line = version
        .map(|v| format!("version: {}\n", v.trim()))
        .unwrap_or_default();
    let depends_line = if depends.is_empty() {
        String::new()
    } else {
        format!(
            "depends: [{}]\n",
            depends
                .iter()
                .map(|d| d.trim())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let content = format!(
        "---\nname: {}\ndescription: {}\n{}{}---\n\n{}\n",
        name.trim(),
        description.trim(),
        version_line,
        depends_line,
        body.trim()
    );

    let skill_file = skill_folder.join("SKILL.md");
    fs::write(&skill_file, content)?;
    Ok(skill_file)
}

/// Deletes a skill directory completely.
pub fn delete_skill(skills_dir: &Path, id: &str) -> Result<(), std::io::Error> {
    let skill_folder = skills_dir.join(id);
    if skill_folder.exists() {
        fs::remove_dir_all(skill_folder)?;
    }
    Ok(())
}

/// Exports a skill's SKILL.md to a standalone destination markdown file.
pub fn export_skill(skills_dir: &Path, id: &str, dest_file: &Path) -> Result<(), std::io::Error> {
    let skill_file = skills_dir.join(id).join("SKILL.md");
    if !skill_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Skill '{id}' not found in {}", skills_dir.display()),
        ));
    }

    if let Some(parent) = dest_file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&skill_file, dest_file)?;
    Ok(())
}

/// Imports an external markdown file with valid frontmatter into the target skills directory.
pub fn import_skill(
    skills_dir: &Path,
    source_file: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if !source_file.exists() {
        return Err(format!("Source file '{}' not found", source_file.display()).into());
    }

    let content = fs::read_to_string(source_file)?;
    let (name, _desc, _version, _depends) = extract_skill_full_metadata(&content);

    if name.is_empty() {
        return Err(
            "Imported markdown file must declare a valid 'name:' in YAML frontmatter".into(),
        );
    }

    let id = name.trim().to_lowercase().replace(' ', "-");
    let skill_dir = skills_dir.join(&id);
    fs::create_dir_all(&skill_dir)?;

    fs::write(skill_dir.join("SKILL.md"), &content)?;
    Ok(id)
}
