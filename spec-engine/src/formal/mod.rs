//! Gate 0: Formal specifications validator (Alloy, TLA+, Spectral, DBML).

use crate::pack::targets::is_binary_in_path;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalCheckResult {
    pub tool: String,
    pub file: String,
    pub passed: bool,
    pub skipped: bool,
    pub message: String,
}

/// Scans directory for formal specifications and runs validators if available.
pub fn validate_formal_specs(dir: &Path) -> Vec<FormalCheckResult> {
    let mut results = Vec::new();

    let walker = WalkBuilder::new(dir)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let file_str = path.display().to_string();

        if ext == "als" {
            results.push(validate_alloy(path, &file_str));
        } else if ext == "tla" {
            results.push(validate_tla(path, &file_str));
        } else if file_name == "api.yaml" || file_name == "api.json" || file_name == "openapi.yaml" || file_name == "openapi.json" {
            results.push(validate_openapi(path, &file_str));
        } else if ext == "dbml" || file_name == "schema.dbml" {
            results.push(validate_dbml(path, &file_str));
        }
    }

    results.sort_by(|a, b| a.file.cmp(&b.file));
    results
}

fn validate_alloy(path: &Path, file_str: &str) -> FormalCheckResult {
    if !is_binary_in_path("alloy") {
        return FormalCheckResult {
            tool: "alloy".to_string(),
            file: file_str.to_string(),
            passed: true,
            skipped: true,
            message: "alloy CLI not found in PATH; skipped formal model solving".to_string(),
        };
    }

    match Command::new("alloy").arg("--solve").arg(path).output() {
        Ok(out) => {
            let passed = out.status.success();
            let msg = if passed {
                "Alloy model solved cleanly".to_string()
            } else {
                String::from_utf8_lossy(&out.stderr).trim().to_string()
            };
            FormalCheckResult {
                tool: "alloy".to_string(),
                file: file_str.to_string(),
                passed,
                skipped: false,
                message: msg,
            }
        }
        Err(e) => FormalCheckResult {
            tool: "alloy".to_string(),
            file: file_str.to_string(),
            passed: false,
            skipped: false,
            message: e.to_string(),
        },
    }
}

fn validate_tla(path: &Path, file_str: &str) -> FormalCheckResult {
    if !is_binary_in_path("tlc") {
        return FormalCheckResult {
            tool: "tlc".to_string(),
            file: file_str.to_string(),
            passed: true,
            skipped: true,
            message: "tlc CLI not found in PATH; skipped model checking".to_string(),
        };
    }

    match Command::new("tlc")
        .arg("-tool")
        .arg("-deadlock")
        .arg(path)
        .output()
    {
        Ok(out) => {
            let passed = out.status.success();
            let msg = if passed {
                "TLA+ model checked cleanly".to_string()
            } else {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            };
            FormalCheckResult {
                tool: "tlc".to_string(),
                file: file_str.to_string(),
                passed,
                skipped: false,
                message: msg,
            }
        }
        Err(e) => FormalCheckResult {
            tool: "tlc".to_string(),
            file: file_str.to_string(),
            passed: false,
            skipped: false,
            message: e.to_string(),
        },
    }
}

fn validate_openapi(path: &Path, file_str: &str) -> FormalCheckResult {
    let has_spectral = is_binary_in_path("spectral");
    if !has_spectral {
        return FormalCheckResult {
            tool: "spectral".to_string(),
            file: file_str.to_string(),
            passed: true,
            skipped: true,
            message: "spectral CLI not found in PATH; skipped OpenAPI linting".to_string(),
        };
    }

    match Command::new("spectral").arg("lint").arg(path).output() {
        Ok(out) => {
            let passed = out.status.success();
            let msg = if passed {
                "OpenAPI spec validated cleanly".to_string()
            } else {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            };
            FormalCheckResult {
                tool: "spectral".to_string(),
                file: file_str.to_string(),
                passed,
                skipped: false,
                message: msg,
            }
        }
        Err(e) => FormalCheckResult {
            tool: "spectral".to_string(),
            file: file_str.to_string(),
            passed: false,
            skipped: false,
            message: e.to_string(),
        },
    }
}

fn validate_dbml(path: &Path, file_str: &str) -> FormalCheckResult {
    if !is_binary_in_path("dbml2sql") {
        return FormalCheckResult {
            tool: "dbml2sql".to_string(),
            file: file_str.to_string(),
            passed: true,
            skipped: true,
            message: "dbml2sql CLI not found in PATH; skipped DBML validation".to_string(),
        };
    }

    match Command::new("dbml2sql")
        .arg("--postgres")
        .arg(path)
        .output()
    {
        Ok(out) => {
            let passed = out.status.success();
            let msg = if passed {
                "DBML schema converted cleanly".to_string()
            } else {
                String::from_utf8_lossy(&out.stderr).trim().to_string()
            };
            FormalCheckResult {
                tool: "dbml2sql".to_string(),
                file: file_str.to_string(),
                passed,
                skipped: false,
                message: msg,
            }
        }
        Err(e) => FormalCheckResult {
            tool: "dbml2sql".to_string(),
            file: file_str.to_string(),
            passed: false,
            skipped: false,
            message: e.to_string(),
        },
    }
}
