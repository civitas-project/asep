use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::policy::Severity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiSlopViolationKind {
    DecorativeBanner { banner: String },
    WorkflowNarration { step: String },
    AiBuzzword { word: String },
    ExcessiveNesting { depth: usize },
    ElseBranch { token: String },
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
        AntiSlopViolationKind::ElseBranch { token: _ } => {
            "Else branch forbidden; use early return (guard clause)".to_string()
        }
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

    for (idx, mut remaining) in content.lines().enumerate() {
        let line_number = idx + 1;
        let mut cleaned = String::with_capacity(remaining.len());
        if remaining.contains("asep:allow-antislop") {
            // Strict rules (nesting, else) still run on this line; only
            // comment-based rules opt out via check_comment's own marker check.
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

    scan_nesting(&mut violations, path, &cleaned_lines);
    scan_else(&mut violations, path, &cleaned_lines);
    violations
}

// @impl:FR-ANTISLOP-06 @impl:FR-ANTISLOP-09
/// Scans comment-stripped code for if blocks nested deeper than the policy
/// threshold from canonical rule AS-CODE-NESTING. `max_nesting` is the number
/// of allowed enclosing `if` levels (default 0). An `if` is flagged when the
/// number of enclosing `if` blocks exceeds `max_nesting`; `depth` is reported
/// as enclosing count + 1. This rule is not suppressible via inline markers.
fn scan_nesting(
    violations: &mut Vec<AntiSlopViolation>,
    path: &Path,
    lines: &[String],
) {
    let nesting_rule = code_rule("AS-CODE-NESTING");
    let max_nesting = nesting_rule.and_then(|r| r.max_nesting).unwrap_or(0);
    let severity = nesting_rule.map(|r| r.severity).unwrap_or(Severity::Warning);
    let mut stack: Vec<usize> = Vec::new();
    let mut brace_depth: usize = 0;

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;
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
                let enclosing = stack.len();
                let depth = enclosing + 1;
                stack.push(brace_depth);
                if enclosing > max_nesting {
                    violations.push(AntiSlopViolation {
                        file_path: path.to_path_buf(),
                        line_number,
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

// @impl:FR-ANTISLOP-05 @impl:FR-ANTISLOP-09
/// Scans comment-stripped code for `else` tokens (whole word, per
/// canonical rule AS-CODE-ELSE). String literals are tracked so an `else`
/// inside `"..."`, `r"..."`, or `r#"..."#` is ignored. This rule is not
/// suppressible via `asep:allow-antislop`.
fn scan_else(
    violations: &mut Vec<AntiSlopViolation>,
    path: &Path,
    lines: &[String],
) {
    let else_rule = code_rule("AS-CODE-ELSE");
    let severity = else_rule.map(|r| r.severity).unwrap_or(Severity::Error);
    let message = else_rule
        .map(|r| r.message.clone())
        .unwrap_or_else(|| "Else branch forbidden; use early return (guard clause)".to_string());

    for (idx, line) in lines.iter().enumerate() {
        let sanitized = strip_string_literals(line);
        let chars: Vec<char> = sanitized.chars().collect();
        let mut i = 0;
        while i + 4 <= chars.len() {
            if chars[i..i + 4] == ['e', 'l', 's', 'e']
                && (i == 0 || !is_ident_char(chars[i - 1]))
                && (i + 4 == chars.len() || !is_ident_char(chars[i + 4]))
            {
                violations.push(AntiSlopViolation {
                    file_path: path.to_path_buf(),
                    line_number: idx + 1,
                    kind: AntiSlopViolationKind::ElseBranch {
                        token: "else".to_string(),
                    },
                    message: message.clone(),
                    severity,
                });
                i += 4;
            } else {
                i += 1;
            }
        }
    }
}

/// Replaces the contents of string literals with spaces so their text is
/// never matched by token scanners. Handles `"..."` with `\` escapes and
/// raw strings `r"..."` / `r#"..."#`. Comments are assumed pre-blanked.
fn strip_string_literals(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' && raw_string_hash_count(&chars, i).is_some() {
            let n = raw_string_hash_count(&chars, i).unwrap();
            out.push(' ');
            i += 1;
            // Scan for closing `"` followed by exactly n `#`.
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '"' {
                    let mut hashes = 0;
                    while i + hashes + 1 < chars.len() && chars[i + hashes + 1] == '#' {
                        hashes += 1;
                    }
                    if hashes == n {
                        // push the closing quote and hashes as spaces
                        out.extend(std::iter::repeat_n(' ', 1 + hashes));
                        i += 1 + hashes;
                        closed = true;
                        break;
                    }
                }
                out.push(' ');
                i += 1;
            }
            if !closed {
                // unterminated raw string: rest already blanked
            }
        } else if chars[i] == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    out.extend(std::iter::repeat_n(' ', 2));
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                }
                out.push(' ');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// If `chars[idx]` is `"` opening a raw string `r#^n"` (n >= 1), returns n.
/// `r"..."` (n=0) is handled as a regular string and returns None.
fn raw_string_hash_count(chars: &[char], idx: usize) -> Option<usize> {
    if chars[idx] != '"' {
        return None;
    }
    let mut n = 0;
    let mut j = idx;
    while j > 0 && chars[j - 1] == '#' {
        n += 1;
        j -= 1;
    }
    if n == 0 || j == 0 || chars[j - 1] != 'r' {
        return None;
    }
    if j > 1 && chars[j - 2] == 'b' {
        // `br"..."`: `b` must not be part of an identifier.
        if j > 2 && is_ident_char(chars[j - 3]) {
            return None;
        }
    } else if j > 1 && is_ident_char(chars[j - 2]) {
        return None;
    }
    Some(n)
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

    #[test]
    fn detects_else_and_else_if() {
        let violations = rs(
            "fn foo(a: bool, d: bool) {\n    if a { return; } else { }\n    if d { return; } else if true { return; } else { }\n}\n",
        );
        let count = violations
            .iter()
            .filter(|v| matches!(v.kind, AntiSlopViolationKind::ElseBranch { .. }))
            .count();
        assert_eq!(count, 3, "expected 3 ElseBranch violations, got {violations:?}");
        assert!(violations.iter().all(|v| !matches!(
            v.kind,
            AntiSlopViolationKind::ExcessiveNesting { .. }
        )));
    }

    #[test]
    fn nested_if_forbidden_but_top_level_allowed() {
        let violations = rs(
            "fn foo() {\n    if a {\n        if b {\n        }\n    }\n    if c {\n    }\n    if d {\n    }\n}\n",
        );
        let nesting: Vec<_> = violations
            .iter()
            .filter(|v| matches!(v.kind, AntiSlopViolationKind::ExcessiveNesting { .. }))
            .collect();
        assert_eq!(nesting.len(), 1, "expected exactly one nesting violation");
        assert_eq!(nesting[0].line_number, 3);
        if let AntiSlopViolationKind::ExcessiveNesting { depth } = &nesting[0].kind {
            assert_eq!(*depth, 2);
        } else {
            panic!("expected ExcessiveNesting");
        }
        assert!(!violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::ElseBranch { .. }
        )));
    }

    #[test]
    fn else_in_string_literal_or_comment_is_ignored() {
        let violations = rs("let s = \"else\"; // else branch\n");
        assert!(!violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::ElseBranch { .. }
        )));
    }

    #[test]
    fn antislop_allow_does_not_suppress_strict_rules() {
        let violations = rs(
            "fn foo(x: bool) {\n    if x { return; } else { } // asep:allow-antislop\n    /// Delve into robust solutions // asep:allow-antislop\n}\n",
        );
        assert!(violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::ElseBranch { .. }
        )));
        assert!(!violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::AiBuzzword { .. }
        )));
    }
}
