use crate::scanner::lexer::Priority;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// @design:FR-CLI-GRAPH-01
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenTrace {
    pub id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub missing_design: bool,
    pub missing_impl: bool,
    pub missing_test: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementEntry {
    pub id: String,
    pub priority: Priority,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub has_design: bool,
    pub has_impl: bool,
    pub has_test: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceabilityReport {
    pub total_requirements: usize,
    pub must_satisfied: usize,
    pub must_broken: Vec<BrokenTrace>,
    pub should_pending: Vec<BrokenTrace>,
    pub draft_total: usize,
    pub draft_pending: Vec<BrokenTrace>,
    pub is_closed: bool,
}
