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
    Req { id: String, priority: Priority },
    Design { id: String },
    Impl { id: String },
    Test { id: String },
    Token { id: String },
    Gov { category: String, id: String },
    Adr { id: String },
    Limit { id: String },
    Criticality { level: String },
    Rigor { tier: String },
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
            "req" => TagKind::Req {
                id: part1.to_string(),
                priority: req_priority(part2),
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
            "adr" => TagKind::Adr { id: part1.to_string() },
            "lim" => TagKind::Limit { id: part1.to_string() },
            "criticality" => TagKind::Criticality { level: part1.to_string() },
            "rigor" => TagKind::Rigor { tier: part1.to_string() },
            _ => continue,
        };

        results.push((whole_match.start(), kind));
    }

    for caps in REL_REGEX.captures_iter(line) {
        let whole_match = caps.get(0).expect("regex group 0 always participates");
        if whole_match.start() > 0 && line.as_bytes()[whole_match.start() - 1] == b'\\' {
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
            TagKind::Rel { kind, from, to, note },
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

/// Priority for a `@req:ID:PRIORITY` tag, defaulting to `Must`.
fn req_priority(part2: Option<&str>) -> Priority {
    let p = match part2 {
        Some(p) => p,
        None => return Priority::Must,
    };
    if p.eq_ignore_ascii_case("SHOULD") {
        return Priority::Should;
    }
    if p.eq_ignore_ascii_case("DRAFT") {
        return Priority::Draft;
    }
    Priority::Must
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
