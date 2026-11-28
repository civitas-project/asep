//! Requirement-to-Coverage mapping for test-driven traceability.

use crate::scanner::lexer::TagKind;
use crate::scanner::ScannedTag;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCoverageItem {
    pub req_id: String,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub percentage: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCoverageReport {
    pub items: Vec<RequirementCoverageItem>,
    pub all_passed: bool,
    pub threshold: f64,
}

/// Parses LCOV format into a map of FilePath -> (LineNumber -> HitCount).
pub fn parse_lcov(content: &str) -> HashMap<PathBuf, HashMap<usize, usize>> {
    let mut files = HashMap::new();
    let mut current_file: Option<PathBuf> = None;
    let mut current_lines = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        match trimmed {
            rest if rest.starts_with("SF:") => {
                let rest = rest.strip_prefix("SF:").unwrap().trim();
                flush_current_file(&mut current_file, &mut current_lines, &mut files);
                current_file = Some(PathBuf::from(rest));
            }
            rest if rest.starts_with("DA:") => {
                let rest = rest.strip_prefix("DA:").unwrap().trim();
                apply_da_hit(&mut current_lines, rest);
            }
            _ if trimmed == "end_of_record" => {
                flush_current_file(&mut current_file, &mut current_lines, &mut files);
            }
            _ => {}
        }
    }

    if let Some(f) = current_file {
        files.insert(f, current_lines);
    }

    files
}

/// Flushes the current file's accumulated lines into `files`, if a file is active.
fn flush_current_file(
    current_file: &mut Option<PathBuf>,
    current_lines: &mut HashMap<usize, usize>,
    files: &mut HashMap<PathBuf, HashMap<usize, usize>>,
) {
    if let Some(f) = current_file.take() {
        files.insert(f, std::mem::take(current_lines));
    }
}

/// Parses LCOV file from disk.
pub fn parse_lcov_file(
    path: &Path,
) -> Result<HashMap<PathBuf, HashMap<usize, usize>>, std::io::Error> {
    let content = fs::read_to_string(path)?;
    Ok(parse_lcov(&content))
}

/// Parses a single LCOV `DA:` payload into `(line_number, hit_count)`.
/// Returns `None` when the payload has no comma or either value fails to parse.
fn parse_da_pair(rest: &str) -> Option<(usize, usize)> {
    let (line_s, hit_s) = rest.split_once(',')?;
    let line_num = line_s.parse::<usize>().ok()?;
    let hit_count = hit_s.parse::<usize>().ok()?;
    Some((line_num, hit_count))
}

/// Applies a parsed `DA:` hit count to the current line map, if parsing succeeded.
fn apply_da_hit(current_lines: &mut HashMap<usize, usize>, rest: &str) {
    if let Some((line_num, hit_count)) = parse_da_pair(rest) {
        current_lines.insert(line_num, hit_count);
    }
}

/// Maps coverage execution counts against implementation lines for each requirement.
pub fn map_coverage_to_requirements(
    tags: &[ScannedTag],
    cov_map: &HashMap<PathBuf, HashMap<usize, usize>>,
    threshold: f64,
) -> RequirementCoverageReport {
    let mut req_impls: HashMap<String, Vec<(PathBuf, usize)>> = HashMap::new();

    for tag in tags {
        if let TagKind::Impl { ref id } = tag.kind {
            req_impls
                .entry(id.clone())
                .or_default()
                .push((tag.file_path.clone(), tag.line_number));
        }
    }

    let mut items = Vec::new();
    let mut all_passed = true;

    for (req_id, impl_lines) in req_impls {
        let total = impl_lines.len();
        let mut covered = 0;

        for (path, line_num) in impl_lines {
            // Find match by exact path or matching suffix
            let hit = cov_map.iter().find_map(|(p, lines)| {
                if !(p == &path || path.ends_with(p) || p.ends_with(&path)) {
                    return None;
                }
                lines.get(&line_num).copied()
            });

            if hit.is_some_and(|count| count > 0) {
                covered += 1;
            }
        }

        let percentage = match total {
            0 => 0.0,
            _ => (covered as f64 / total as f64) * 100.0,
        };

        let passed = percentage >= threshold;
        if !passed {
            all_passed = false;
        }

        items.push(RequirementCoverageItem {
            req_id,
            total_lines: total,
            covered_lines: covered,
            percentage,
            passed,
        });
    }

    items.sort_by(|a, b| a.req_id.cmp(&b.req_id));

    RequirementCoverageReport {
        items,
        all_passed,
        threshold,
    }
}
