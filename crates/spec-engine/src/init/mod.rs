// @impl:FR-AMB-01
use crate::error::AsepError;
use crate::pack::embedded::{get_core_protocol, get_embedded_skills, AMBIENT_SKILL_IDS};
use std::fs;
use std::path::Path;

const AGENT_FILES: [&str; 3] = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"];
const ASEP_START: &str = "<!-- asep:start -->";
const ASEP_END: &str = "<!-- asep:end -->";

// @impl:FR-CLI-INIT-01 @impl:FR-CLI-03 @impl:FR-INIT-01
/// Minimal, idempotent, non-destructive adoption.
///
/// Creates only the native instruction entry points, the canonical Core
/// Protocol and the five ambient domain skills. Documentation artifacts are
/// on demand: init never fabricates a MUST requirement or empty doc tree.
pub fn init_asep_project(root: &Path) -> Result<(), AsepError> {
    write_ambient_skill_files(root)
}

/// Writes the canonical core protocol, then idempotent ambient blocks into
/// AGENTS.md / CLAUDE.md / GEMINI.md and installs only the ambient domain skills.
/// Retired compatibility skills are not installed as ambient by default. Existing
/// files are never deleted, so upgrades remain non-destructive; rewriting the block
/// removes stale mandatory router/bootstrap references from the entry points.
fn write_ambient_skill_files(root: &Path) -> Result<(), AsepError> {
    let protocol_path = root.join("protocol").join("CORE.md");
    if let Some(parent) = protocol_path.parent() {
        fs::create_dir_all(parent)?;
    }
    // The protocol is a managed generated artifact, not user-authored content.
    fs::write(&protocol_path, get_core_protocol())?;

    let ambient: Vec<_> = get_embedded_skills()
        .iter()
        .filter(|skill| AMBIENT_SKILL_IDS.contains(&skill.id))
        .collect();
    let block = build_asep_block(&ambient);

    for name in AGENT_FILES {
        let path = root.join(name);
        let existing = match path.exists() {
            true => fs::read_to_string(&path)?,
            false => String::new(),
        };
        fs::write(&path, replace_or_append_block(&existing, &block))?;
    }

    let skills_dir = root.join("skills");
    fs::create_dir_all(&skills_dir)?;
    for skill in ambient {
        let skill_md = skills_dir.join(skill.id).join("SKILL.md");
        if let Some(parent) = skill_md.parent() {
            fs::create_dir_all(parent)?;
        }
        let managed = skill
            .content
            .contains("<!-- asep-managed-skill -->");
        match fs::read_to_string(&skill_md) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::write(&skill_md, skill.content)?,
            Err(e) => return Err(AsepError::Io(e)),
            Ok(existing) if existing.contains("<!-- asep-managed-skill -->") && managed => {
                // Refresh only ASEP-managed copies; never overwrite custom text.
                fs::write(&skill_md, skill.content)?;
            }
            Ok(_) => {
                eprintln!(
                    "asep: preserved customized skill {}; managed update skipped",
                    skill.id
                );
            }
        }
    }

    Ok(())
}

fn build_asep_block(skills: &[&crate::pack::embedded::EmbeddedSkill]) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(ASEP_START.to_string());
    lines.push("mode: ask".to_string());
    lines.push("protocol: protocol/CORE.md".to_string());
    lines.push(
        "activation: Native tools first; load a domain skill only when its scope matches. \
         Run ASEP checks, impact, audits or full CI only when the activation contract applies; \
         never fabricate evidence or ACCEPTED."
            .to_string(),
    );
    lines.push("skills:".to_string());
    for skill in skills {
        lines.push(format!("- {}", skill.id));
    }
    lines.push(ASEP_END.to_string());
    lines.join("\n")
}

fn replace_or_append_block(existing: &str, block: &str) -> String {
    let start = existing.find(ASEP_START);
    let end = existing.find(ASEP_END);
    match (start, end) {
        (Some(s), Some(e)) if e > s => {
            let end_after = e + ASEP_END.len();
            format!("{}{}{}", &existing[..s], block, &existing[end_after..])
        }
        _ => {
            let mut result = existing.to_string();
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(block);
            result
        }
    }
}
