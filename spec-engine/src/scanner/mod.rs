pub mod lexer;

pub use lexer::{parse_tags_from_line, Priority, TagKind, TagOccurrence};

use crate::error::AsepError;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

// @design:FR-CLI-SCAN-01
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedTag {
    pub file_path: PathBuf,
    pub kind: TagKind,
    pub line_number: usize,
    pub raw_line: String,
}

#[derive(Debug, Clone, Default)]
pub struct ScanCompleteness {
    pub files_seen: usize,
    pub files_read: usize,
    pub skipped_binary: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct ScanResult {
    pub tags: Vec<ScannedTag>,
    pub completeness: ScanCompleteness,
}

// @impl:FR-AUD-08
/// Recursively scans repository for traceability tags respecting gitignore rules.
/// Walker, open, and line-read errors are recorded in `completeness.errors`
/// instead of being silently swallowed.
///
/// # Errors: Returns `AsepError::Io` if `root` does not exist or is not a directory.
pub fn scan_project_tags_detailed(root: &Path) -> Result<ScanResult, AsepError> {
    if !root.exists() || !root.is_dir() {
        let message = format!("scan root is not a directory: {}", root.display());
        return Err(AsepError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            message,
        )));
    }

    let mut tags = Vec::new();
    let mut completeness = ScanCompleteness::default();

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
            Err(error) => {
                completeness.errors.push(format!("walker error: {error}"));
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        completeness.files_seen += 1;

        // Calculate relative path for clean reporting
        let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let is_code_file = matches!(
            rel_path.extension().and_then(|s| s.to_str()),
            Some("rs" | "go" | "ts" | "js" | "py" | "cpp" | "c" | "h" | "hpp" | "java" | "cs")
        );

        match File::open(path) {
            Ok(mut file) => {
                let mut buffer = [0u8; 4096];
                match file.read(&mut buffer) {
                    Ok(n) => {
                        if buffer[..n].contains(&0) {
                            completeness.skipped_binary += 1;
                            continue;
                        }
                    }
                    Err(error) => {
                        completeness
                            .errors
                            .push(format!("sniff error at {}: {error}", path.display()));
                        continue;
                    }
                }
                if file.seek(SeekFrom::Start(0)).is_err() {
                    completeness
                        .errors
                        .push(format!("rewind error at {}", path.display()));
                    continue;
                }
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
                        Err(error) => {
                            completeness.errors.push(format!(
                                "line read error at {}:{idx}: {error}",
                                path.display()
                            ));
                        }
                    }
                }
                completeness.files_read += 1;
            }
            Err(error) => {
                completeness
                    .errors
                    .push(format!("open error at {}: {error}", path.display()));
            }
        }
    }

    Ok(ScanResult { tags, completeness })
}

// @impl:FR-CLI-SCAN-01 @impl:FR-CLI-01
/// Recursively scans repository for traceability tags respecting gitignore rules.
///
/// # Errors: Returns `AsepError::Io` if `root` is invalid or traversal errors occur.
pub fn scan_project_tags(root: &Path) -> Result<Vec<ScannedTag>, AsepError> {
    scan_project_tags_detailed(root).map(|result| result.tags)
}
