pub mod lexer;

pub use lexer::{parse_tags_from_line, Priority, TagKind, TagOccurrence};

use crate::error::AsepError;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// @design:FR-CLI-SCAN-01
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedTag {
    pub file_path: PathBuf,
    pub kind: TagKind,
    pub line_number: usize,
    pub raw_line: String,
}

// @impl:FR-CLI-SCAN-01 @impl:FR-CLI-01
/// Recursively scans repository for traceability tags respecting gitignore rules.
///
/// # Errors: Returns `AsepError::Io` if traversal errors occur.
pub fn scan_project_tags(root: &Path) -> Result<Vec<ScannedTag>, AsepError> {
    let mut tags = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .add_custom_ignore_filename(".asepignore")
        .build();

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Calculate relative path for clean reporting
        let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let is_code_file = matches!(
            rel_path.extension().and_then(|s| s.to_str()),
            Some("rs" | "go" | "ts" | "js" | "py" | "cpp" | "c" | "h" | "hpp" | "java" | "cs")
        );

        // Open and read file line-by-line
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                for (idx, line_res) in reader.lines().enumerate() {
                    match line_res {
                        Ok(line) => {
                            let occurrences = parse_tags_from_line(&line, idx + 1);
                            for occ in occurrences {
                                // Code files implement (@impl) and test (@test); they do not declare requirements (@req).
                                if is_code_file && matches!(occ.kind, TagKind::Req { .. }) {
                                    continue;
                                }

                                tags.push(ScannedTag {
                                    file_path: rel_path.clone(),
                                    kind: occ.kind,
                                    line_number: occ.line_number,
                                    raw_line: occ.raw_line,
                                });
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Ok(tags)
}
