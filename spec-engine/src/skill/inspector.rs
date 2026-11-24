//! Status inspector comparing local skills against binary embedded originals.
// @impl:FR-AMB-05

use crate::pack::embedded::{get_embedded_skills, EmbeddedSkill};
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

    let dir_entries = match fs::read_dir(skills_dir) {
        Ok(entries) => entries,
        Err(_) => return drifts,
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
        let content = match fs::read_to_string(&skill_file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let (parsed_name, _, _, _) = extract_skill_full_metadata(&content);
        // Guard-clause mandate: no if/else, match on bool instead.
        #[allow(clippy::match_bool)]
        let skill_id = match parsed_name.is_empty() {
            false => parsed_name,
            true => dir_name.clone(),
        };

        let embedded_match = embedded
            .iter()
            .find(|e| e.id == skill_id || e.directory == dir_name);

        // Guard-clause mandate: `if let` would nest an `if` inside it (depth 2),
        // so keep the explicit match; the empty None arm is intentional.
        #[allow(clippy::single_match)]
        match embedded_match {
            Some(orig) => {
                if content.trim() != orig.content.trim() {
                    drifts.push(SkillDrift {
                        skill_id: orig.id.to_string(),
                        path: skill_file,
                        kind: "modified".to_string(),
                    });
                }
            }
            None => {}
        }
    }

    drifts
}

/// Extracts YAML frontmatter `name` and `description` fields from a `SKILL.md` string.
pub fn extract_skill_metadata(content: &str) -> (String, String) {
    let (name, desc, _, _) = extract_skill_full_metadata(content);
    (name, desc)
}

/// Extracts YAML frontmatter `requires` field: `[a, b]` inline list or
/// `- item` dash list. Returns empty vec when `requires` is absent.
pub fn extract_skill_requires(content: &str) -> Vec<String> {
    let mut requires: Vec<String> = Vec::new();

    // Guard-clause mandate: no if/else and no match guards; explicit matches
    // with empty None arms replace if-let chains here.
    #[allow(clippy::single_match)]
    match content.strip_prefix("---") {
        Some(rest) =>
        {
            #[allow(clippy::single_match)]
            match rest.find("\n---") {
                Some(end_idx) => {
                    let frontmatter = &rest[..end_idx];
                    let mut current_field: Option<&str> = None;

                    for raw_line in frontmatter.lines() {
                        let line = raw_line.trim_end();
                        if line.trim().is_empty() {
                            continue;
                        }

                        let trimmed = line.trim();
                        match (
                            trimmed.starts_with("requires:"),
                            trimmed.starts_with("- ") && current_field == Some("requires"),
                        ) {
                            (true, _) => {
                                let v = trimmed["requires:".len()..].trim();
                                parse_depends_list(v, &mut requires);
                                current_field = Some("requires");
                            }
                            (_, true) => {
                                let item = trimmed[2..]
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .trim();
                                if !item.is_empty() {
                                    requires.push(item.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                None => {}
            }
        }
        None => {}
    }

    requires
}

/// Extracts full frontmatter metadata including version and depends.
pub fn extract_skill_full_metadata(content: &str) -> (String, String, Option<String>, Vec<String>) {
    let mut name = String::new();
    let mut description = String::new();
    let mut version: Option<String> = None;
    let mut depends: Vec<String> = Vec::new();

    // Guard-clause mandate: no if/else and no match guards; explicit matches
    // with empty None arms replace if-let chains here.
    #[allow(clippy::single_match)]
    match content.strip_prefix("---") {
        Some(rest) => {
            #[allow(clippy::single_match)]
            match rest.find("\n---") {
                Some(end_idx) => {
                    let frontmatter = &rest[..end_idx];
                    let mut current_field: Option<&str> = None;

                    for raw_line in frontmatter.lines() {
                        let line = raw_line.trim_end();
                        if line.trim().is_empty() {
                            continue;
                        }

                        let trimmed = line.trim();
                        // Priority-ordered replacement of the if/else-if chain.
                        // Match on a tuple of bools: no match guards, no else.
                        match (
                            trimmed.starts_with("name:"),
                            trimmed.starts_with("description:"),
                            trimmed.starts_with("version:"),
                            trimmed.starts_with("depends:"),
                            trimmed.starts_with("- ") && current_field == Some("depends"),
                            (line.starts_with("  ") || line.starts_with("    "))
                                && current_field == Some("description"),
                        ) {
                            (true, _, _, _, _, _) => {
                                name = trimmed["name:".len()..]
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .to_string();
                                current_field = Some("name");
                            }
                            (_, true, _, _, _, _) => {
                                let clean = trimmed["description:".len()..]
                                    .trim()
                                    .trim_matches('>')
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'');
                                description = clean.to_string();
                                current_field = Some("description");
                            }
                            (_, _, true, _, _, _) => {
                                let v = trimmed["version:".len()..]
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .to_string();
                                if !v.is_empty() {
                                    version = Some(v);
                                }
                                current_field = Some("version");
                            }
                            (_, _, _, true, _, _) => {
                                let v = trimmed["depends:".len()..].trim();
                                parse_depends_list(v, &mut depends);
                                current_field = Some("depends");
                            }
                            (_, _, _, _, true, _) => {
                                let item = trimmed[2..]
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .trim();
                                if !item.is_empty() {
                                    depends.push(item.to_string());
                                }
                            }
                            (_, _, _, _, false, true) => {
                                if !description.is_empty() {
                                    description.push(' ');
                                }
                                description.push_str(line.trim());
                            }
                            _ => {}
                        }
                    }
                }
                None => {}
            }
        }

        None => {}
    }

    (name, description, version, depends)
}

/// Parses a YAML `depends:` value: `[a, b]` or a comma-separated inline list.
fn parse_depends_list(v: &str, depends: &mut Vec<String>) {
    let items = match (v.starts_with('['), v.ends_with(']')) {
        (true, true) => &v[1..v.len() - 1],
        _ => v,
    };
    for item in items.split(',') {
        let item = item.trim().trim_matches('"').trim_matches('\'').trim();
        if !item.is_empty() {
            depends.push(item.to_string());
        }
    }
}

/// Inspects the target skills directory and categorizes each skill as Builtin, Modified, Custom, or Missing.
pub fn inspect_skills(skills_dir: &Path) -> Vec<SkillEntry> {
    let mut entries = Vec::new();
    let mut discovered_ids = HashSet::new();
    let embedded = get_embedded_skills();

    if skills_dir.exists() && skills_dir.is_dir() {
        collect_installed_skills(skills_dir, embedded, &mut entries, &mut discovered_ids);
    }

    // Identify embedded skills that are absent locally -> Missing
    for orig in embedded {
        if !discovered_ids.contains(orig.id) {
            let (orig_name, orig_desc, orig_version, orig_depends) =
                extract_skill_full_metadata(orig.content);
            entries.push(SkillEntry {
                id: orig.id.to_string(),
                // Guard-clause mandate: no if/else, match on bool instead.
                #[allow(clippy::match_bool)]
                name: match orig_name.is_empty() {
                    false => orig_name,
                    true => orig.id.to_string(),
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

/// Scans `skills_dir` for installed skill copies and pushes Builtin/Custom entries.
fn collect_installed_skills(
    skills_dir: &Path,
    embedded: &[EmbeddedSkill],
    entries: &mut Vec<SkillEntry>,
    discovered_ids: &mut HashSet<String>,
) {
    let dir_entries = match fs::read_dir(skills_dir) {
        Ok(entries) => entries,
        Err(_) => return,
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

        let content = match fs::read_to_string(&skill_file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let (parsed_name, parsed_desc, parsed_version, parsed_depends) =
            extract_skill_full_metadata(&content);
        // Guard-clause mandate: no if/else, match on bool instead.
        #[allow(clippy::match_bool)]
        let skill_id = match parsed_name.is_empty() {
            false => parsed_name.clone(),
            true => dir_name.clone(),
        };

        let embedded_match = embedded
            .iter()
            .find(|e| e.id == skill_id || e.directory == dir_name);

        let status = match embedded_match {
            Some(orig) => {
                // Guard-clause mandate: no if/else, match on bool instead.
                #[allow(clippy::match_bool)]
                match content.trim() == orig.content.trim() {
                    true => SkillStatus::Builtin,
                    false => SkillStatus::Modified,
                }
            }
            None => SkillStatus::Custom,
        };

        discovered_ids.insert(skill_id.clone());
        entries.push(SkillEntry {
            id: skill_id,
            // Guard-clause mandate: no if/else, match on bool instead.
            #[allow(clippy::match_bool)]
            name: match parsed_name.is_empty() {
                false => parsed_name,
                true => dir_name,
            },
            description: parsed_desc,
            file_path: skill_file,
            status,
            version: parsed_version,
            depends: parsed_depends,
        });
    }
}
