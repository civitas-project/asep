use crate::policy::EdgeKind;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Must,
    Should,
    Draft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagKind {
    Req {
        id: String,
        priority: Priority,
    },
    Design {
        id: String,
    },
    Impl {
        id: String,
    },
    Test {
        id: String,
    },
    Token {
        id: String,
    },
    Gov {
        category: String,
        id: String,
    },
    Adr {
        id: String,
    },
    Limit {
        id: String,
    },
    Criticality {
        level: String,
    },
    Rigor {
        tier: String,
    },
    Rel {
        kind: EdgeKind,
        from: String,
        to: Vec<String>,
        note: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagOccurrence {
    pub kind: TagKind,
    pub line_number: usize,
    pub raw_line: String,
}

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(req|design|impl|test|token|gov|adr|lim|criticality|rigor):([A-Za-z0-9_./-]+)(?::([A-Za-z0-9_./-]+))?")
        .expect("valid tag regex")
});

static REL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"@rel:(satisfies|verifies|informs|constrains):([A-Za-z0-9_./-]+)->([A-Za-z0-9_./-]+(?:,[A-Za-z0-9_./-]+)*)(?:\s+"([^"]*)")?"#)
        .expect("valid rel tag regex")
});

// @impl:FR-CLI-SCAN-02
/// Extracts traceability tags from a single line of source code.
///
/// # Invariants: Returned tags preserve exact identifier casing and order of occurrence.
pub fn parse_tags_from_line(line: &str, line_num: usize) -> Vec<TagOccurrence> {
    let mut results: Vec<(usize, TagKind)> = Vec::new();

    for caps in TAG_REGEX.captures_iter(line) {
        let whole_match = caps.get(0).expect("regex group 0 always participates");
        if whole_match.start() > 0 && line.as_bytes()[whole_match.start() - 1] == b'\\' {
            continue;
        }

        let tag_type = caps.get(1).map_or("", |m| m.as_str());
        let part1 = caps.get(2).map_or("", |m| m.as_str());
        let part2 = caps.get(3).map(|m| m.as_str());

        let kind = match tag_type {
            "req" => match req_priority(part2) {
                Some(priority) => TagKind::Req {
                    id: part1.to_string(),
                    priority,
                },
                None => continue,
            },
            "design" => TagKind::Design {
                id: part1.to_string(),
            },
            "impl" => TagKind::Impl {
                id: part1.to_string(),
            },
            "test" => TagKind::Test {
                id: part1.to_string(),
            },
            "token" => TagKind::Token {
                id: part1.to_string(),
            },
            "gov" => {
                let (category, id) = gov_parts(part1, part2);
                TagKind::Gov { category, id }
            }
            "adr" => TagKind::Adr {
                id: part1.to_string(),
            },
            "lim" => TagKind::Limit {
                id: part1.to_string(),
            },
            "criticality" => TagKind::Criticality {
                level: part1.to_string(),
            },
            "rigor" => TagKind::Rigor {
                tier: part1.to_string(),
            },
            _ => continue,
        };

        results.push((whole_match.start(), kind));
    }

    for caps in REL_REGEX.captures_iter(line) {
        let whole_match = caps.get(0).expect("regex group 0 always participates");
        if whole_match.start() > 0 && line.as_bytes()[whole_match.start() - 1] == b'\\' {
            continue;
        }
        let end = whole_match.end();
        let closed = end == line.len()
            || matches!(line.as_bytes()[end], b' ' | b'\t' | b'\r' | b'\n' | b'\0');
        if !closed {
            continue;
        }

        let kind = match caps.get(1).map_or("", |m| m.as_str()) {
            "satisfies" => EdgeKind::Satisfies,
            "verifies" => EdgeKind::Verifies,
            "informs" => EdgeKind::Informs,
            "constrains" => EdgeKind::Constrains,
            _ => continue,
        };
        let from = caps.get(2).map_or("", |m| m.as_str()).to_string();
        let to: Vec<String> = caps
            .get(3)
            .map_or("", |m| m.as_str())
            .split(',')
            .map(|s| s.to_string())
            .collect();
        let note = caps.get(4).map(|m| m.as_str().to_string());

        results.push((
            whole_match.start(),
            TagKind::Rel {
                kind,
                from,
                to,
                note,
            },
        ));
    }

    results.sort_by_key(|(start, _)| *start);

    results
        .into_iter()
        .map(|(_, kind)| TagOccurrence {
            kind,
            line_number: line_num,
            raw_line: line.to_string(),
        })
        .collect()
}

// @impl:FR-AUD-25
/// Priority for a `@req:ID:PRIORITY` tag.
///
/// Only `MUST`, `SHOULD`, and `DRAFT` (case-insensitive) are recognized.
/// Returns `None` when a priority token is present but unrecognized; the
/// caller skips the tag instead of silently defaulting it to `Must`. A missing
/// priority still defaults to `Must` for backward compatibility.
fn req_priority(part2: Option<&str>) -> Option<Priority> {
    match part2 {
        None => Some(Priority::Must),
        Some(p) => priority_from_token(p),
    }
}

fn priority_from_token(token: &str) -> Option<Priority> {
    match token.to_ascii_uppercase().as_str() {
        "MUST" => Some(Priority::Must),
        "SHOULD" => Some(Priority::Should),
        "DRAFT" => Some(Priority::Draft),
        _ => None,
    }
}

/// Diagnostic for a `@req` tag whose priority token is not `MUST`, `SHOULD`,
/// or `DRAFT`. Returns `Some(message)` when the tag on `line` must be skipped,
/// `None` when the line carries no diagnosable priority problem.
/// `parse_tags_from_line` skips such tags instead of silently defaulting them
/// to `Must`; callers record the returned string as a parse diagnostic.
pub fn parse_tag_diagnostic(line: &str, line_no: usize) -> Option<String> {
    for caps in TAG_REGEX.captures_iter(line) {
        let whole_match = caps.get(0).expect("regex group 0 always participates");
        if whole_match.start() > 0 && line.as_bytes()[whole_match.start() - 1] == b'\\' {
            continue;
        }
        let tag_type = caps.get(1).map_or("", |m| m.as_str());
        let part2 = caps.get(3).map(|m| m.as_str());
        let unknown = match tag_type {
            "req" => match part2 {
                None => false,
                Some(p) => priority_from_token(p).is_none(),
            },
            _ => false,
        };
        if unknown {
            let id = caps.get(2).map_or("", |m| m.as_str());
            let token = part2.unwrap_or("<missing>");
            return Some(format!(
                "line {line_no}: unknown priority '{token}' in @req:{id}; tag skipped"
            ));
        }
    }
    None
}

/// Splits a `@gov` tag into `(category, id)`.
fn gov_parts(part1: &str, part2: Option<&str>) -> (String, String) {
    if let Some(p2) = part2 {
        return (part1.to_string(), p2.to_string());
    }
    if !part1.contains('-') {
        return ("GENERAL".to_string(), part1.to_string());
    }
    let (cat, rest) = part1.split_once('-').expect("part1 contains '-'");
    (cat.to_string(), rest.to_string())
}

// @test:FR-AUD-25
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_priority_is_skipped_not_must() {
        let tags = parse_tags_from_line("// @req:FR-AUD-25-A:UNKNOWN", 1);
        assert!(tags.is_empty(), "unknown priority must not default to Must");

        let diagnostic = parse_tag_diagnostic("// @req:FR-AUD-25-A:UNKNOWN", 1);
        assert!(
            diagnostic.is_some(),
            "unknown priority must record a diagnostic"
        );
        let message = diagnostic.expect("diagnostic present");
        assert!(
            message.contains("UNKNOWN"),
            "diagnostic names the bad token: {message}"
        );
        assert!(
            message.contains("line 1"),
            "diagnostic names the line: {message}"
        );
    }

    #[test]
    fn test_malformed_rel_suffix_is_rejected() {
        let rejected = parse_tags_from_line(
            &format!("// {}satisfies:auth/session->AUTH-01;bogus", "@rel:"),
            1,
        );
        assert!(
            rejected.is_empty(),
            "trailing garbage after targets must not match"
        );

        let valid =
            parse_tags_from_line(&format!("// {}satisfies:auth/session->AUTH-01", "@rel:"), 1);
        assert_eq!(valid.len(), 1, "plain rel tag still parses");

        let noted = parse_tags_from_line(
            &format!("// {}satisfies:auth/session->AUTH-01 \"ok\"", "@rel:"),
            1,
        );
        assert_eq!(noted.len(), 1, "rel tag with quoted note still parses");
    }

    #[test]
    fn test_valid_priorities_still_parse() {
        let cases = [
            ("// @req:FR-P-1:MUST", Priority::Must),
            ("// @req:FR-P-2:SHOULD", Priority::Should),
            ("// @req:FR-P-3:DRAFT", Priority::Draft),
            ("// @req:FR-P-4", Priority::Must),
        ];
        for (line, expected) in cases {
            let tags = parse_tags_from_line(line, 1);
            assert_eq!(tags.len(), 1, "line {line} should yield exactly one tag");
            match &tags[0].kind {
                TagKind::Req { priority, .. } => assert_eq!(*priority, expected),
                other => panic!("expected Req tag, got {other:?}"),
            }
        }
    }
}
