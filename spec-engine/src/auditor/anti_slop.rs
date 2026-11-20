use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::policy::Severity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiSlopViolationKind {
    DecorativeBanner { banner: String },
    WorkflowNarration { step: String },
    AiBuzzword { word: String },
    ExcessiveNesting { depth: usize },
    Canonical { rule_id: String, matched: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiSlopViolation {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub kind: AntiSlopViolationKind,
    pub message: String,
    pub severity: Severity,
}

/// A compiled canonical anti-slop rule from the policy TOML.
struct CompiledRule {
    id: String,
    domain: String,
    severity: Severity,
    max_nesting: Option<usize>,
    message: String,
    regex: Regex,
}

/// Compiles the nine canonical policy regexes exactly once.
fn antislop_rules_cache() -> &'static [CompiledRule] {
    static RULES: OnceLock<Vec<CompiledRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        crate::policy::load_policy()
            .antislop_rules()
            .iter()
            .map(|r| CompiledRule {
                id: r.id.clone(),
                domain: r.domain.clone(),
                severity: r.severity,
                max_nesting: r.max_nesting,
                message: r.message.clone(),
                regex: Regex::new(&r.pattern).expect("valid antislop rule pattern"),
            })
            .collect()
    })
}

/// Returns the compiled code-domain rule with the given id, if any.
fn code_rule(id: &str) -> Option<&'static CompiledRule> {
    antislop_rules_cache()
        .iter()
        .find(|r| r.domain == "code" && r.id == id)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    SlashStar,
    Hash,
    Markdown,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SegKind {
    DocLine,
    LineComment,
    BlockComment,
}

/// Returns the compiled rule with the given id, in any domain.
fn rule_by_id(id: &str) -> Option<&'static CompiledRule> {
    antislop_rules_cache().iter().find(|r| r.id == id)
}

/// Banner match: policy AS-CODE-BANNER pattern plus canonical AS-CODE-BOXDRAW.
fn banner_match(trimmed: &str) -> Option<String> {
    code_rule("AS-CODE-BANNER")
        .and_then(|r| r.regex.find(trimmed).map(|m| m.as_str().to_string()))
        .or_else(|| {
            code_rule("AS-CODE-BOXDRAW")
                .and_then(|r| r.regex.find(trimmed).map(|m| m.as_str().to_string()))
        })
}

fn lang_of(path: &Path) -> Lang {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs")
        | Some("go")
        | Some("ts")
        | Some("js")
        | Some("java")
        | Some("cpp")
        | Some("c") => Lang::SlashStar,
        Some("py") => Lang::Hash,
        Some("md") => Lang::Markdown,
        _ => Lang::Other,
    }
}

fn message_for(kind: &AntiSlopViolationKind) -> String {
    match kind {
        AntiSlopViolationKind::DecorativeBanner { banner } => {
            format!("Decorative banner comment detected: '{}'", banner)
        }
        AntiSlopViolationKind::WorkflowNarration { step } => {
            format!("Workflow narration comment detected: '{}'", step)
        }
        AntiSlopViolationKind::AiBuzzword { word } => {
            format!("AI buzzword '{}' detected; use plain language", word)
        }
        AntiSlopViolationKind::ExcessiveNesting { depth } => format!(
            "Excessive if nesting depth {}; refactor with guard clauses and early returns",
            depth
        ),
        AntiSlopViolationKind::Canonical { rule_id, matched } => {
            format!("{rule_id}: '{matched}'")
        }
    }
}

fn check_comment(
    violations: &mut Vec<AntiSlopViolation>,
    path: &Path,
    line_number: usize,
    text: &str,
    kind: SegKind,
) {
    let trimmed = text.trim();

    // Inline marker opts a specific line out of anti-slop auditing.
    if trimmed.contains("asep:allow-antislop") {
        return;
    }

    if let Some(banner) = banner_match(trimmed) {
        violations.push(AntiSlopViolation {
            file_path: path.to_path_buf(),
            line_number,
            kind: AntiSlopViolationKind::DecorativeBanner {
                banner: banner.clone(),
            },
            message: message_for(&AntiSlopViolationKind::DecorativeBanner {
                banner: banner.clone(),
            }),
            severity: Severity::Error,
        });
    }

    if let Some(rule) = code_rule("AS-CODE-NARRATION") {
        if let Some(m) = rule.regex.find(trimmed) {
            violations.push(AntiSlopViolation {
                file_path: path.to_path_buf(),
                line_number,
                kind: AntiSlopViolationKind::WorkflowNarration {
                    step: m.as_str().to_string(),
                },
                message: message_for(&AntiSlopViolationKind::WorkflowNarration {
                    step: m.as_str().to_string(),
                }),
                severity: Severity::Error,
            });
        }
    }

    let check_buzzwords =
        matches!(kind, SegKind::DocLine | SegKind::LineComment | SegKind::BlockComment);
    if check_buzzwords {
        if let Some(rule) = code_rule("AS-CODE-BUZZWORD") {
            for m in rule.regex.find_iter(trimmed) {
                violations.push(AntiSlopViolation {
                    file_path: path.to_path_buf(),
                    line_number,
                    kind: AntiSlopViolationKind::AiBuzzword {
                        word: m.as_str().to_string(),
                    },
                    message: message_for(&AntiSlopViolationKind::AiBuzzword {
                        word: m.as_str().to_string(),
                    }),
                    severity: Severity::Error,
                });
            }
        }
    }
}

/// Audits a single source or spec file for anti-slop violations:
/// decorative banners, workflow narration, AI buzzwords, and excessive if nesting.
pub fn audit_anti_slop(path: &Path, content: &str) -> Vec<AntiSlopViolation> {
    let mut violations = Vec::new();
    let lang = lang_of(path);
    let is_markdown = matches!(lang, Lang::Markdown);

    if is_markdown {
        for (idx, line) in content.lines().enumerate() {
            let line_number = idx + 1;
            // Inline marker opts a specific line out of anti-slop auditing.
            if line.contains("asep:allow-antislop") {
                continue;
            }
            // AI buzzwords in Markdown use the copy-domain rule AS-COPY-BUZZWORD
            // but keep the existing AiBuzzword violation kind.
            if let Some(rule) = rule_by_id("AS-COPY-BUZZWORD") {
                for m in rule.regex.find_iter(line) {
                    violations.push(AntiSlopViolation {
                        file_path: path.to_path_buf(),
                        line_number,
                        kind: AntiSlopViolationKind::AiBuzzword {
                            word: m.as_str().to_string(),
                        },
                        message: message_for(&AntiSlopViolationKind::AiBuzzword {
                            word: m.as_str().to_string(),
                        }),
                        severity: rule.severity,
                    });
                }
            }
            // The six non-code canonical rules apply to Markdown lines only.
            for rule in antislop_rules_cache() {
                if rule.domain == "code" {
                    continue;
                }
                for m in rule.regex.find_iter(line) {
                    violations.push(AntiSlopViolation {
                        file_path: path.to_path_buf(),
                        line_number,
                        kind: AntiSlopViolationKind::Canonical {
                            rule_id: rule.id.clone(),
                            matched: m.as_str().to_string(),
                        },
                        message: rule.message.clone(),
                        severity: rule.severity,
                    });
                }
            }
        }
        return violations;
    }

    let mut in_block = false;
    let mut cleaned_lines: Vec<String> = Vec::new();
    let mut allowed_lines: HashSet<usize> = HashSet::new();

    for (idx, mut remaining) in content.lines().enumerate() {
        let line_number = idx + 1;
        let mut cleaned = String::with_capacity(remaining.len());
        if remaining.contains("asep:allow-antislop") {
            allowed_lines.insert(line_number);
            cleaned.push_str(remaining);
            cleaned_lines.push(cleaned);
            continue;
        }

        if in_block {
            match remaining.find("*/") {
                Some(end) => {
                    let text = &remaining[..end];
                    check_comment(
                        &mut violations,
                        path,
                        line_number,
                        text,
                        SegKind::BlockComment,
                    );
                    cleaned.push_str(&" ".repeat(text.len() + 4));
                    remaining = &remaining[end + 2..];
                    in_block = false;
                }
                None => {
                    check_comment(
                        &mut violations,
                        path,
                        line_number,
                        remaining,
                        SegKind::BlockComment,
                    );
                    cleaned.push_str(&" ".repeat(remaining.len()));
                    cleaned_lines.push(cleaned);
                    continue;
                }
            }
        }

        loop {
            let slash_slash = remaining.find("//");
            let slash_star = remaining.find("/*");
            let hash = if lang == Lang::Hash {
                remaining.find('#')
            } else {
                None
            };
            let marker = match (slash_slash, slash_star, hash) {
                (None, None, None) => break,
                (Some(a), None, None) => ("//", a),
                (None, Some(b), None) => ("/*", b),
                (None, None, Some(c)) => ("#", c),
                (Some(a), None, Some(c)) => {
                    if c <= a {
                        ("#", c)
                    } else {
                        ("//", a)
                    }
                }
                (None, Some(b), Some(c)) => {
                    if c <= b {
                        ("#", c)
                    } else {
                        ("/*", b)
                    }
                }
                (Some(a), Some(b), None) => {
                    if b <= a {
                        ("/*", b)
                    } else {
                        ("//", a)
                    }
                }
                (Some(a), Some(b), Some(c)) => {
                    if c <= a.min(b) {
                        ("#", c)
                    } else if b <= a {
                        ("/*", b)
                    } else {
                        ("//", a)
                    }
                }
            };
            let (tag, pos) = marker;
            cleaned.push_str(&remaining[..pos]);

            match tag {
                "//" => {
                    let text = &remaining[pos + 2..];
                    let kind = if text.starts_with('/') {
                        SegKind::DocLine
                    } else {
                        SegKind::LineComment
                    };
                    check_comment(&mut violations, path, line_number, text, kind);
                    cleaned.push_str(&" ".repeat(text.len()));
                    remaining = "";
                    break;
                }
                "#" => {
                    let text = &remaining[pos + 1..];
                    check_comment(
                        &mut violations,
                        path,
                        line_number,
                        text,
                        SegKind::LineComment,
                    );
                    cleaned.push_str(&" ".repeat(text.len()));
                    remaining = "";
                    break;
                }
                "/*" => {
                    let after = &remaining[pos + 2..];
                    if let Some(end) = after.find("*/") {
                        let text = &after[..end];
                        check_comment(
                            &mut violations,
                            path,
                            line_number,
                            text,
                            SegKind::BlockComment,
                        );
                        cleaned.push_str(&" ".repeat(text.len() + 4));
                        remaining = &after[end + 2..];
                        continue;
                    } else {
                        in_block = true;
                        check_comment(
                            &mut violations,
                            path,
                            line_number,
                            after,
                            SegKind::BlockComment,
                        );
                        cleaned.push_str(&" ".repeat(after.len()));
                        remaining = "";
                        break;
                    }
                }
                _ => unreachable!(),
            }
        }
        cleaned.push_str(remaining);
        cleaned_lines.push(cleaned);
    }

    scan_nesting(&mut violations, path, &cleaned_lines, &allowed_lines);
    violations
}

/// Scans comment-stripped code for if blocks nested at or above the policy
/// threshold from canonical rule AS-CODE-NESTING (`max_nesting`, default 3).
/// An if at depth d is nested inside d-1 enclosing if blocks.
/// Lines whose numbers are in `allowed_lines` are skipped (inline marker).
fn scan_nesting(
    violations: &mut Vec<AntiSlopViolation>,
    path: &Path,
    lines: &[String],
    allowed_lines: &std::collections::HashSet<usize>,
) {
    let nesting_rule = code_rule("AS-CODE-NESTING");
    let max_nesting = nesting_rule.and_then(|r| r.max_nesting).unwrap_or(3);
    let severity = nesting_rule.map(|r| r.severity).unwrap_or(Severity::Warning);
    let mut stack: Vec<usize> = Vec::new();
    let mut brace_depth: usize = 0;

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;
        // Inline marker opts this line out of anti-slop auditing.
        if allowed_lines.contains(&line_number) {
            continue;
        }
        let mut i = 0;
        while i < line.len() {
            if line[i..].starts_with("if")
                && (i == 0
                    || line[..i]
                        .chars()
                        .last()
                        .map(|c| !is_ident_char(c))
                        .unwrap_or(true))
                && (i + 2 == line.len()
                    || line[i + 2..]
                        .chars()
                        .next()
                        .map(|c| !is_ident_char(c))
                        .unwrap_or(true))
            {
                let depth = stack.len() + 1;
                stack.push(brace_depth);
                if depth >= max_nesting {
                    violations.push(AntiSlopViolation {
                        file_path: path.to_path_buf(),
                        line_number: idx + 1,
                        kind: AntiSlopViolationKind::ExcessiveNesting { depth },
                        message: message_for(&AntiSlopViolationKind::ExcessiveNesting { depth }),
                        severity,
                    });
                }
                i += 2;
                continue;
            }
            match line[i..].chars().next() {
                Some('{') => brace_depth += 1,
                Some('}') => {
                    brace_depth = brace_depth.saturating_sub(1);
                    while let Some(&top) = stack.last() {
                        if top >= brace_depth {
                            stack.pop();
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
            i += line[i..].chars().next().map_or(0, |c| c.len_utf8());
        }
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Walks a project tree (respecting .gitignore and .asepignore) and audits
/// code files (.rs, .go, .ts, .js, .py, .java, .cpp, .c) and spec docs (.md).
pub fn audit_project_anti_slop(
    root: &Path,
) -> Result<Vec<AntiSlopViolation>, crate::error::AsepError> {
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
        if path.is_file() {
            let is_audit_ext = matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("rs")
                | Some("go")
                | Some("ts")
                | Some("js")
                | Some("py")
                | Some("java")
                | Some("cpp")
                | Some("c")
                | Some("md")
            );
            if is_audit_ext {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
                    let mut file_violations = audit_anti_slop(&rel_path, &content);
                    violations.append(&mut file_violations);
                }
            }
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rs(content: &str) -> Vec<AntiSlopViolation> {
        audit_anti_slop(Path::new("test.rs"), content)
    }

    fn md(content: &str) -> Vec<AntiSlopViolation> {
        audit_anti_slop(Path::new("spec.md"), content)
    }

    #[test]
    fn detects_banner_comment() {
        let violations = rs("// =====================\nfn foo() {}\n"); // asep:allow-antislop
        assert!(violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::DecorativeBanner { .. }
        )));
    }

    #[test]
    fn detects_banner_in_block_comment() {
        let violations = rs("/* ----------------- */\nfn foo() {}\n"); // asep:allow-antislop
        assert!(violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::DecorativeBanner { .. }
        )));
    }

    #[test]
    fn detects_workflow_narration() {
        let violations = rs("// Step 1: validate input\nfn foo() {}\n"); // asep:allow-antislop
        assert!(violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::WorkflowNarration { .. }
        )));
    }

    #[test]
    fn detects_buzzword_in_docstring() {
        let violations = rs("/// Delve into robust solutions\nfn foo() {}\n"); // asep:allow-antislop
        let words: Vec<String> = violations
            .iter()
            .filter_map(|v| match &v.kind {
                AntiSlopViolationKind::AiBuzzword { word } => Some(word.clone()),
                _ => None,
            })
            .collect();
        assert!(words.iter().any(|w| w == "Delve"));
        assert!(words.iter().any(|w| w == "robust"));
    }

    #[test]
    fn detects_buzzword_in_markdown() {
        let violations = md("This is a seamless experience.\n");
        assert!(violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::AiBuzzword { .. }
        )));
    }

    #[test]
    fn detects_excessive_nesting() {
        let violations = rs(
            "fn foo(x: bool) {\n    if x {\n        if true {\n            if false {\n            }\n        }\n    }\n}\n",
        );
        assert!(violations.iter().any(|v| {
            if let AntiSlopViolationKind::ExcessiveNesting { depth } = &v.kind {
                *depth >= 3
            } else {
                false
            }
        }));
    }

    #[test]
    fn clean_idiomatic_code_passes() {
        let violations = rs(
            "fn foo(x: i32) -> i32 {\n    if x < 0 { return 0; }\n    if x > 100 { return 100; }\n    x\n}\n",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn sequential_ifs_are_not_nesting() {
        let violations = rs(
            "fn foo(a: bool, b: bool) {\n    if a {}\n    if b {}\n}\n",
        );
        assert!(violations.iter().all(|v| !matches!(
            v.kind,
            AntiSlopViolationKind::ExcessiveNesting { .. }
        )));
    }
}
