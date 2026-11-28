use std::fs;
use std::path::Path;

pub mod anti_slop;

pub use anti_slop::{
    audit_anti_slop, audit_project_anti_slop, AntiSlopViolation, AntiSlopViolationKind,
};

pub mod doc_contracts;

pub use doc_contracts::{
    audit_doc_contracts, audit_project_docstrings, DocContractViolation, DocContractViolationKind,
};

// @impl:FR-AMB-06
/// Audits the optional ambient skills gate.
///
/// The gate is opt-in: when `root/skills` does not exist, no violations are
/// returned. When it exists, all of the following are required:
///   * at least one of AGENTS.md/CLAUDE.md/GEMINI.md contains both
///     `<!-- asep:start -->` and `<!-- asep:end -->`;
///   * every `skills/<id>/SKILL.md` frontmatter `description` starts with
///     `Use when`;
///   * `skills/using-asep/SKILL.md` body (frontmatter excluded) is at most
///     200 words, if present.
///
/// Returns one descriptive string per violation.
pub fn audit_ambient_skills(root: &Path) -> Vec<String> {
    let skills_dir = root.join("skills");
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut violations = Vec::new();

    // 1. Pointer block must appear in at least one agent config file.
    let has_markers = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"].iter().any(|name| {
        let path = root.join(name);
        fs::read_to_string(&path).map_or(false, |content| {
            content.contains("<!-- asep:start -->") && content.contains("<!-- asep:end -->")
        })
    });
    if !has_markers {
        violations.push(
            "ambient skills: none of AGENTS.md/CLAUDE.md/GEMINI.md contain both \
             `<!-- asep:start -->` and `<!-- asep:end -->` markers"
                .to_string(),
        );
    }

    // 2. Every skill SKILL.md frontmatter description must start with `Use when`.
    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            audit_ambient_skill_entry(&mut violations, entry);
        }
    }

    // 3. `skills/using-asep/SKILL.md` body must be <= 200 words, if present.
    let using_asep = skills_dir.join("using-asep").join("SKILL.md");
    check_using_asep_body(&mut violations, &using_asep);

    violations
}

/// Audits one `skills/<id>/SKILL.md` entry: skipped when not a file or
/// unreadable; records a violation when its frontmatter `description` is
/// missing or does not start with `Use when`.
fn audit_ambient_skill_entry(violations: &mut Vec<String>, entry: fs::DirEntry) {
    let skill_md = entry.path().join("SKILL.md");
    if !skill_md.is_file() {
        return;
    }
    let content = match fs::read_to_string(&skill_md) {
        Ok(content) => content,
        Err(_) => return,
    };
    let description = frontmatter_description(&content);
    let starts_with_use_when = description.map_or(false, |d| d.starts_with("Use when"));
    if !starts_with_use_when {
        violations.push(format!(
            "ambient skills: {} frontmatter description must start with `Use when`",
            skill_md.display()
        ));
    }
}

/// Checks `skills/using-asep/SKILL.md` body word budget when the file
/// exists and is readable; skips silently when absent or unreadable.
fn check_using_asep_body(violations: &mut Vec<String>, using_asep: &Path) {
    if !using_asep.is_file() {
        return;
    }
    let content = match fs::read_to_string(using_asep) {
        Ok(content) => content,
        Err(_) => return,
    };
    let body = strip_frontmatter(&content);
    let word_count = body.split_whitespace().count();
    if word_count > 200 {
        violations.push(format!(
            "ambient skills: {} body has {word_count} words (max 200)",
            using_asep.display()
        ));
    }
}

/// Returns the frontmatter `description` value, or `None` when no frontmatter
/// or no `description:` field is present. ponytail: single-line YAML only;
/// upgrade path is a proper YAML parser when multi-line values are needed.
fn frontmatter_description(content: &str) -> Option<String> {
    let block = content.strip_prefix("---\n")?;
    let end = block.find("\n---\n")?;
    let fm = &block[..end];
    fm.lines()
        .find_map(|line| line.trim_start().strip_prefix("description:"))
        .map(|value| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string()
        })
}

/// Returns file content with YAML frontmatter stripped, or the content
/// unchanged when no frontmatter block is present.
fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---\n") {
        return content;
    }
    let rest = &content[4..];
    match rest.find("\n---\n") {
        Some(end) => &rest[end + 5..],
        None => content,
    }
}
