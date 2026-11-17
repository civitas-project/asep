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
        if let Some(rest) = trimmed.strip_prefix("SF:") {
            if let Some(f) = current_file.take() {
                files.insert(f, std::mem::take(&mut current_lines));
            }
            let file_str = rest.trim();
            current_file = Some(PathBuf::from(file_str));
        } else if let Some(rest) = trimmed.strip_prefix("DA:") {
            if let Some((line_s, hit_s)) = rest.split_once(',') {
                if let (Ok(line_num), Ok(hit_count)) = (line_s.parse::<usize>(), hit_s.parse::<usize>()) {
                    current_lines.insert(line_num, hit_count);
                }
            }
        } else if trimmed == "end_of_record" {
            if let Some(f) = current_file.take() {
                files.insert(f, std::mem::take(&mut current_lines));
            }
        }
    }

    if let Some(f) = current_file {
        files.insert(f, current_lines);
    }

    files
}

/// Parses LCOV file from disk.
pub fn parse_lcov_file(path: &Path) -> Result<HashMap<PathBuf, HashMap<usize, usize>>, std::io::Error> {
    let content = fs::read_to_string(path)?;
    Ok(parse_lcov(&content))
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
                if p == &path || path.ends_with(p) || p.ends_with(&path) {
                    lines.get(&line_num).copied()
                } else {
                    None
                }
            });

            if let Some(count) = hit {
                if count > 0 {
                    covered += 1;
                }
            }
        }

        let percentage = if total > 0 {
            (covered as f64 / total as f64) * 100.0
        } else {
            0.0
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
