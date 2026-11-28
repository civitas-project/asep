use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocContractViolationKind {
    Parroting { identifier: String },
    PhantomSection { section: String },
    InfallibleErrorSection { identifier: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocContractViolation {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub kind: DocContractViolationKind,
    pub message: String,
}

static PHANTOM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)#\s*(Panics|Errors|Invariants)\s*[:\-]?\s*(None|N/A|Normal operation|0|Nothing)\b",
    )
    .expect("valid phantom regex")
});

static FN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\(").expect("valid fn regex")
});

// @impl:FR-CLI-DOC-01-A @impl:FR-CLI-04
// @impl:FR-CLI-DOC-02
/// Audits inline docstrings against Docstring Contract Anti-Parroting and Strict Conditionality standards.
///
/// # Invariants: Returned violations contain 1-indexed source line numbers.
pub fn audit_doc_contracts(path: &Path, content: &str) -> Vec<DocContractViolation> {
    let mut violations = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut current_doc: Vec<(usize, String)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        // Accumulate Rust docstrings
        if trimmed.starts_with("///") {
            let doc_text = trimmed.trim_start_matches("///").trim().to_string();
            current_doc.push((line_num, doc_text));
            continue;
        }

        // Detect function signature (capture group 1 = fn name)
        let caps = FN_REGEX.captures(trimmed);
        let is_fn = caps.is_some();
        let fn_name = caps
            .as_ref()
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .unwrap_or("");

        // Non-comment, non-function line resets pending docstring
        if !is_fn && !trimmed.starts_with("//") && !trimmed.is_empty() {
            current_doc.clear();
            continue;
        }

        if !is_fn {
            continue;
        }

        // --- function signature reached with optional pending docstring ---

        // 1. Anti-Parroting check on first docstring line (guard via is_some_and, no if-block)
        let parrots = current_doc.first().is_some_and(|(_, first_line_text)| {
            let first_clean = first_line_text.to_lowercase();
            let fn_clean = fn_name.to_lowercase();
            first_clean.starts_with(&fn_clean)
                || first_clean.starts_with(&format!("{}:", fn_clean))
                || first_clean.starts_with(&format!("{} ", fn_clean))
        });
        if parrots {
            let (first_line_num, _) = current_doc[0];
            violations.push(DocContractViolation {
                file_path: path.to_path_buf(),
                line_number: first_line_num,
                kind: DocContractViolationKind::Parroting {
                    identifier: fn_name.to_string(),
                },
                message: format!(
                    "Parroting detected: docstring begins with identifier name '{}'",
                    fn_name
                ),
            });
        }

        // 2. Phantom sections check
        for (doc_line_num, doc_text) in &current_doc {
            let phantom_caps = PHANTOM_REGEX.captures(doc_text);
            if phantom_caps.is_some() {
                let section = phantom_caps
                    .as_ref()
                    .and_then(|c| c.get(1).map(|m| m.as_str()))
                    .unwrap_or("");
                violations.push(DocContractViolation {
                    file_path: path.to_path_buf(),
                    line_number: *doc_line_num,
                    kind: DocContractViolationKind::PhantomSection {
                        section: section.to_string(),
                    },
                    message: format!(
                        "Phantom section detected: '# {}: None' violates Strict Conditionality",
                        section
                    ),
                });
            }
        }

        current_doc.clear();
    }

    violations
}

// @impl:FR-CLI-DOC-01-B
/// Audits all source code files in a project tree for docstring contract violations.
pub fn audit_project_docstrings(
    root: &Path,
) -> Result<Vec<DocContractViolation>, crate::error::AsepError> {
    use ignore::WalkBuilder;
    let mut violations = Vec::new();

    let mut builder = WalkBuilder::new(root);
    builder.require_git(false);
    builder.hidden(false);
    builder.add_custom_ignore_filename(".asepignore");

    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(ext) => ext,
            None => continue,
        };
        if ext != "rs" {
            continue;
        }

        let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let rel_str = rel_path.to_string_lossy();

        // Docstring contracts apply to production source code, not test suites, benches, or templates.
        if rel_str.starts_with("tests/")
            || rel_str.contains("/tests/")
            || rel_str.starts_with("benches/")
            || rel_str.contains("/benches/")
            || rel_str.starts_with("examples/")
            || rel_str.contains("/examples/")
            || rel_str.contains("/templates/")
            || rel_str.contains("/skeletons/")
        {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let file_violations = audit_doc_contracts(&rel_path, &content);
        violations.extend(file_violations);
    }

    Ok(violations)
}
