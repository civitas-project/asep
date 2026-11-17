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
    Req { id: String, priority: Priority },
    Design { id: String },
    Impl { id: String },
    Test { id: String },
    Token { id: String },
    Gov { category: String, id: String },
    Adr { id: String },
    Limit { id: String },
    Criticality { level: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagOccurrence {
    pub kind: TagKind,
    pub line_number: usize,
    pub raw_line: String,
}

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(req|design|impl|test|token|gov|adr|lim|criticality):([A-Za-z0-9_-]+)(?::([A-Za-z0-9_-]+))?")
        .expect("valid tag regex")
});

// @impl:FR-CLI-SCAN-02
/// Extracts traceability tags from a single line of source code.
///
/// # Invariants: Returned tags preserve exact identifier casing and order of occurrence.
pub fn parse_tags_from_line(line: &str, line_num: usize) -> Vec<TagOccurrence> {
    let mut results = Vec::new();

    for caps in TAG_REGEX.captures_iter(line) {
        if let Some(whole_match) = caps.get(0) {
            if whole_match.start() > 0 && line.as_bytes()[whole_match.start() - 1] == b'\\' {
                continue;
            }
        }

        let tag_type = caps.get(1).map_or("", |m| m.as_str());
        let part1 = caps.get(2).map_or("", |m| m.as_str());
        let part2 = caps.get(3).map(|m| m.as_str());

        let kind = match tag_type {
            "req" => {
                let priority = match part2 {
                    Some(p) if p.eq_ignore_ascii_case("SHOULD") => Priority::Should,
                    Some(p) if p.eq_ignore_ascii_case("DRAFT") => Priority::Draft,
                    _ => Priority::Must,
                };
                TagKind::Req {
                    id: part1.to_string(),
                    priority,
                }
            }
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
                let (category, id) = if let Some(p2) = part2 {
                    (part1.to_string(), p2.to_string())
                } else if let Some((cat, rest)) = part1.split_once('-') {
                    (cat.to_string(), rest.to_string())
                } else {
                    ("GENERAL".to_string(), part1.to_string())
                };
                TagKind::Gov { category, id }
            }
            "adr" => TagKind::Adr { id: part1.to_string() },
            "lim" => TagKind::Limit { id: part1.to_string() },
            "criticality" => TagKind::Criticality { level: part1.to_string() },
            _ => continue,
        };

        results.push(TagOccurrence {
            kind,
            line_number: line_num,
            raw_line: line.to_string(),
        });
    }

    results
}
