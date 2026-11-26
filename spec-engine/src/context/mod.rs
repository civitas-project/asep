//! Context bundle assembly from impacted source files.
// @impl:FR-CTX-01
// @impl:FR-CTX-02

use crate::affected::{affected, AffectedReport};
use crate::runtime::budget::{BudgetTracker, ResourceBudget};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use srcdump::config::{Config, OutputFormat, OutputMode};
use srcdump::formatter::Formatter;
use srcidx::parser::ParserRegistry;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextExcerpt {
    pub path: PathBuf,
    pub tokens_est: u64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub changed_paths: Vec<PathBuf>,
    pub objective: String,
    #[serde(skip_deserializing, default = "default_impacted")]
    pub impacted: AffectedReport,
    pub excerpts: Vec<ContextExcerpt>,
    pub truncation: String,
    pub commands: Vec<String>,
}

/// Empty `AffectedReport` used as the serde default for `impacted`.
/// `AffectedReport` (in crate::affected, out of scope for C1) derives only
/// `Serialize`, so the field cannot be derived-deserialized directly.
fn default_impacted() -> AffectedReport {
    AffectedReport {
        impls: Vec::new(),
        requirements: Vec::new(),
        tests: Vec::new(),
        adrs: Vec::new(),
        unmapped: Vec::new(),
        provenance: Default::default(),
        commands: Vec::new(),
    }
}

/// Unique source files impacted by `changed`, sorted and deduplicated.
fn impacted_files(impacted: &AffectedReport) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for node in &impacted.impls {
        files.push(node.file.clone());
    }
    for node in &impacted.requirements {
        files.push(node.file.clone());
    }
    for node in &impacted.tests {
        files.push(node.file.clone());
    }
    for node in &impacted.adrs {
        files.push(node.file.clone());
    }
    files.sort();
    files.dedup();
    files
}

/// Extract `(tokens_est, content)` for one file via `srcdump` NDJSON output.
/// Falls back to raw file contents when no parsable file entry exists.
/// Returns an error only when the file cannot be read.
fn excerpt_for_file(
    formatter: &mut Formatter,
    root: &Path,
    file: &Path,
) -> Result<(u64, String), String> {
    let full = root.join(file);
    let mut output: Vec<u8> = Vec::new();
    if formatter
        .format_files(vec![full.clone()], &mut output)
        .is_err()
    {
        return Err(format!("unreadable file: {}", full.display()));
    }
    let mut tokens_est: u64 = 0;
    let mut content = String::new();
    let mut found = false;
    let text = String::from_utf8_lossy(&output);
    let entry: Option<Value> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("type").and_then(|t| t.as_str()) == Some("file"));
    match entry {
        Some(value) => {
            tokens_est = value
                .get("tokens_est")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            content = value
                .get("content")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            found = true;
        }
        None => {}
    }
    if !found {
        match std::fs::read_to_string(&full) {
            Ok(raw) => {
                tokens_est = 0;
                content = raw;
            }
            Err(_) => {
                return Err(format!("unreadable file: {}", full.display()));
            }
        }
    }
    Ok((tokens_est, content))
}

pub fn compile_context(
    root: &Path,
    paths: &[PathBuf],
    budget: ResourceBudget,
) -> Result<ContextBundle, String> {
    let impacted = affected(root, paths)?;
    let files = impacted_files(&impacted);

    let config = Config {
        root: root.to_path_buf(),
        mode: OutputMode::Signatures,
        format: OutputFormat::Ndjson,
        estimate_tokens: true,
        use_default_includes: false,
        include_ext: vec![],
        ..Default::default()
    };
    let parsers = ParserRegistry::default();
    let mut formatter = Formatter::new(config, parsers).map_err(|e| e.to_string())?;

    let mut tracker = BudgetTracker {
        budget,
        ..Default::default()
    };
    let mut excerpts: Vec<ContextExcerpt> = Vec::new();
    let mut truncation = String::new();
    for file in &files {
        if !truncation.is_empty() {
            break;
        }
        let (tokens_est, content) = excerpt_for_file(&mut formatter, root, file)?;
        tracker.record_tokens(tokens_est);
        match tracker.exceeded() {
            Some(_) => truncation = tracker.blocked_message().unwrap_or_default(),
            None => excerpts.push(ContextExcerpt {
                path: file.clone(),
                tokens_est,
                content,
            }),
        }
    }

    let mut changed_paths: Vec<PathBuf> = paths.to_vec();
    changed_paths.sort();
    changed_paths.dedup();
    let objective = format!("Context for {} changed paths", changed_paths.len());

    let commands = impacted.commands.clone();
    Ok(ContextBundle {
        changed_paths,
        objective,
        impacted,
        excerpts,
        truncation,
        commands,
    })
}
