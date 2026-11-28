// @impl:FR-AMB-01
use crate::error::AsepError;
use crate::pack::embedded::{get_core_protocol, get_embedded_skills, AMBIENT_SKILL_IDS};
use std::fs;
use std::path::Path;

const AGENT_FILES: [&str; 3] = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"];
const ASEP_START: &str = "<!-- asep:start -->";
const ASEP_END: &str = "<!-- asep:end -->";

// @impl:FR-CLI-INIT-01 @impl:FR-CLI-03 @impl:FR-INIT-01
/// Initializes standard documentation directories and baseline PRD specification.
pub fn init_asep_project(root: &Path) -> Result<(), AsepError> {
    let docs_dir = root.join("docs");
    let adr_dir = docs_dir.join("adr");
    let diagrams_dir = docs_dir.join("diagrams");
    let rendered_dir = diagrams_dir.join("rendered");
    let invariants_dir = docs_dir.join("invariants");
    let prd_path = docs_dir.join("PRD.md");

    fs::create_dir_all(&adr_dir)?;
    fs::create_dir_all(&rendered_dir)?;
    fs::create_dir_all(&invariants_dir)?;

    if !prd_path.exists() {
        let starter_prd = r#"# Product Requirements Document (PRD)

<!-- @req:FR-INIT-01:MUST Baseline project specification -->
## 1. Overview
Project initialized under ASEP SDLC framework.
"#;
        fs::write(&prd_path, starter_prd)?;
    }

    write_ambient_skill_files(root)?;

    Ok(())
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
    if !protocol_path.exists() {
        fs::write(&protocol_path, get_core_protocol())?;
    }

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
        if !skill_md.exists() {
            fs::write(&skill_md, skill.content)?;
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
