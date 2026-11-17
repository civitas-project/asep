//! Remote skill retrieval (Git repositories, HTTP endpoints, Registry).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Git {
        url: String,
        tag_or_branch: Option<String>,
    },
    Http {
        url: String,
    },
    Registry {
        id: String,
        version_req: Option<String>,
    },
}

/// Parses a skill identifier or location into a SkillSource.
pub fn parse_skill_specifier(spec: &str) -> SkillSource {
    let s = spec.trim();

    if s.starts_with("git@") || (s.starts_with("http") && s.contains(".git")) {
        if let Some((url, tag)) = s.split_once('#') {
            SkillSource::Git {
                url: url.to_string(),
                tag_or_branch: Some(tag.to_string()),
            }
        } else {
            SkillSource::Git {
                url: s.to_string(),
                tag_or_branch: None,
            }
        }
    } else if s.starts_with("http://") || s.starts_with("https://") {
        SkillSource::Http { url: s.to_string() }
    } else if let Some((id, req)) = s.split_once('@') {
        SkillSource::Registry {
            id: id.to_string(),
            version_req: Some(req.to_string()),
        }
    } else {
        SkillSource::Registry {
            id: s.to_string(),
            version_req: None,
        }
    }
}

/// Fetches a remote skill and writes it into skills_dir/<skill_id>/SKILL.md.
pub fn fetch_remote_skill(
    source: &SkillSource,
    skills_dir: &Path,
) -> Result<PathBuf, String> {
    match source {
        SkillSource::Git { url, tag_or_branch } => {
            let tmp_dir = std::env::temp_dir().join(format!(
                "asep_git_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let mut cmd = Command::new("git");
            cmd.arg("clone").arg("--depth").arg("1");

            if let Some(tag) = tag_or_branch {
                cmd.arg("--branch").arg(tag);
            }
            cmd.arg(url).arg(&tmp_dir);

            let output = cmd.output().map_err(|e| format!("git clone failed: {e}"))?;
            if !output.status.success() {
                let _ = fs::remove_dir_all(&tmp_dir);
                return Err(format!(
                    "git clone failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            // Look for SKILL.md in cloned repo
            let candidate_paths = [
                tmp_dir.join("SKILL.md"),
                tmp_dir.join("skill").join("SKILL.md"),
            ];

            let found = candidate_paths
                .iter()
                .find(|p| p.exists())
                .cloned();

            let res = if let Some(found_path) = found {
                let content = fs::read_to_string(&found_path).map_err(|e| e.to_string())?;
                let (name, _, _, _) = crate::skill::inspector::extract_skill_full_metadata(&content);
                let skill_id = if !name.is_empty() {
                    name.to_lowercase().replace(' ', "-")
                } else {
                    url.split('/').next_back().unwrap_or("remote-skill").trim_end_matches(".git").to_string()
                };

                let dest_folder = skills_dir.join(&skill_id);
                fs::create_dir_all(&dest_folder).map_err(|e| e.to_string())?;
                let dest_file = dest_folder.join("SKILL.md");
                fs::write(&dest_file, content).map_err(|e| e.to_string())?;
                Ok(dest_file)
            } else {
                Err("No SKILL.md found in cloned repository".to_string())
            };

            let _ = fs::remove_dir_all(&tmp_dir);
            res
        }

        SkillSource::Http { url } => {
            let resp = reqwest::blocking::get(url).map_err(|e| format!("HTTP fetch failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("HTTP request returned status {}", resp.status()));
            }
            let content = resp.text().map_err(|e| e.to_string())?;
            let (name, _, _, _) = crate::skill::inspector::extract_skill_full_metadata(&content);
            let skill_id = if !name.is_empty() {
                name.to_lowercase().replace(' ', "-")
            } else {
                "remote-http-skill".to_string()
            };

            let dest_folder = skills_dir.join(&skill_id);
            fs::create_dir_all(&dest_folder).map_err(|e| e.to_string())?;
            let dest_file = dest_folder.join("SKILL.md");
            fs::write(&dest_file, content).map_err(|e| e.to_string())?;
            Ok(dest_file)
        }

        SkillSource::Registry { id, version_req: _ } => {
            // Local embedded fallback if not an external registry URL
            let installed = crate::skill::install::install_skill(skills_dir, id, true)?;
            installed
                .into_iter()
                .next()
                .ok_or_else(|| format!("Skill '{id}' already installed or not found"))
        }
    }
}
