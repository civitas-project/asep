//! Status inspector comparing local skills against binary embedded originals.

use crate::pack::embedded::get_embedded_skills;
use crate::skill::model::{SkillEntry, SkillStatus};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// @impl:FR-SKILL-INSPECT-01-A @impl:FR-SKILL-01
// @impl:FR-SKILL-DRIFT-01

/// Drift between an installed skill copy and its embedded canonical original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDrift {
    pub skill_id: String,
    pub path: PathBuf,
    pub kind: String,
}

/// Compares every installed copy in `skills_dir` with its embedded canonical
/// original and returns drifts for copies whose content differs.
/// Only installed copies are compared; missing skills are not drift.
pub fn detect_skill_drift(skills_dir: &Path) -> Vec<SkillDrift> {
    let mut drifts = Vec::new();
    let embedded = get_embedded_skills();

    if !skills_dir.exists() || !skills_dir.is_dir() {
        return drifts;
    }

    let Ok(dir_entries) = fs::read_dir(skills_dir) else {
        return drifts;
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let Ok(content) = fs::read_to_string(&skill_file) else {
            continue;
        };
        let (parsed_name, _, _, _) = extract_skill_full_metadata(&content);
        let skill_id = if !parsed_name.is_empty() {
            parsed_name
        } else {
            dir_name.clone()
        };

        let embedded_match = embedded
            .iter()
            .find(|e| e.id == skill_id || e.directory == dir_name);

        if let Some(orig) = embedded_match {
            if content.trim() != orig.content.trim() {
                drifts.push(SkillDrift {
                    skill_id: orig.id.to_string(),
                    path: skill_file,
                    kind: "modified".to_string(),
                });
            }
        }
    }

    drifts
}

/// Extracts YAML frontmatter `name` and `description` fields from a `SKILL.md` string.
pub fn extract_skill_metadata(content: &str) -> (String, String) {
    let (name, desc, _, _) = extract_skill_full_metadata(content);
    (name, desc)
}

/// Extracts full frontmatter metadata including version and depends.
pub fn extract_skill_full_metadata(content: &str) -> (String, String, Option<String>, Vec<String>) {
    let mut name = String::new();
    let mut description = String::new();
    let mut version: Option<String> = None;
    let mut depends: Vec<String> = Vec::new();

    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let frontmatter = &rest[..end_idx];
            let mut current_field: Option<&str> = None;

            for raw_line in frontmatter.lines() {
                let line = raw_line.trim_end();
                if line.trim().is_empty() {
                    continue;
                }

                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("name:") {
                    name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                    current_field = Some("name");
                } else if let Some(val) = trimmed.strip_prefix("description:") {
                    let clean = val
                        .trim()
                        .trim_matches('>')
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'');
                    description = clean.to_string();
                    current_field = Some("description");
                } else if let Some(val) = trimmed.strip_prefix("version:") {
                    let v = val.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !v.is_empty() {
                        version = Some(v);
                    }
                    current_field = Some("version");
                } else if let Some(val) = trimmed.strip_prefix("depends:") {
                    let v = val.trim();
                    // Support YAML list: [a, b] or inline comma
                    if v.starts_with('[') && v.ends_with(']') {
                        let inner = &v[1..v.len() - 1];
                        for item in inner.split(',') {
                            let item = item.trim().trim_matches('"').trim_matches('\'').trim();
                            if !item.is_empty() {
                                depends.push(item.to_string());
                            }
                        }
                    } else if !v.is_empty() {
                        for item in v.split(',') {
                            let item = item.trim().trim_matches('"').trim_matches('\'').trim();
                            if !item.is_empty() {
                                depends.push(item.to_string());
                            }
                        }
                    }
                    current_field = Some("depends");
                } else if trimmed.starts_with("- ") && current_field == Some("depends") {
                    // YAML list item form
                    let item = trimmed[2..].trim().trim_matches('"').trim_matches('\'').trim();
                    if !item.is_empty() {
                        depends.push(item.to_string());
                    }
                } else if line.starts_with("  ") || line.starts_with("    ") {
                    // Multi-line continuation
                    if current_field == Some("description") {
                        if !description.is_empty() {
                            description.push(' ');
                        }
                        description.push_str(line.trim());
                    }
                }
            }
        }
    }

    (name, description, version, depends)
}

/// Inspects the target skills directory and categorizes each skill as Builtin, Modified, Custom, or Missing.
pub fn inspect_skills(skills_dir: &Path) -> Vec<SkillEntry> {
    let mut entries = Vec::new();
    let mut discovered_ids = HashSet::new();
    let embedded = get_embedded_skills();

    if skills_dir.exists() && skills_dir.is_dir() {
        if let Ok(dir_entries) = fs::read_dir(skills_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_file = path.join("SKILL.md");
                    if skill_file.exists() && skill_file.is_file() {
                        let dir_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if let Ok(content) = fs::read_to_string(&skill_file) {
                            let (parsed_name, parsed_desc, parsed_version, parsed_depends) =
                                extract_skill_full_metadata(&content);
                            let skill_id = if !parsed_name.is_empty() {
                                parsed_name.clone()
                            } else {
                                dir_name.clone()
                            };

                            let embedded_match = embedded
                                .iter()
                                .find(|e| e.id == skill_id || e.directory == dir_name);

                            let status = match embedded_match {
                                Some(orig) => {
                                    if content.trim() == orig.content.trim() {
                                        SkillStatus::Builtin
                                    } else {
                                        SkillStatus::Modified
                                    }
                                }
                                None => SkillStatus::Custom,
                            };

                            discovered_ids.insert(skill_id.clone());
                            entries.push(SkillEntry {
                                id: skill_id,
                                name: if !parsed_name.is_empty() {
                                    parsed_name
                                } else {
                                    dir_name
                                },
                                description: parsed_desc,
                                file_path: skill_file,
                                status,
                                version: parsed_version,
                                depends: parsed_depends,
                            });
                        }
                    }
                }
            }
        }
    }

    // Identify embedded skills that are absent locally -> Missing
    for orig in embedded {
        if !discovered_ids.contains(orig.id) {
            let (orig_name, orig_desc, orig_version, orig_depends) =
                extract_skill_full_metadata(orig.content);
            entries.push(SkillEntry {
                id: orig.id.to_string(),
                name: if !orig_name.is_empty() {
                    orig_name
                } else {
                    orig.id.to_string()
                },
                description: orig_desc,
                file_path: skills_dir.join(orig.directory).join("SKILL.md"),
                status: SkillStatus::Missing,
                version: orig_version,
                depends: orig_depends,
            });
        }
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}
