use crate::policy::EdgeKind;
use crate::scanner::lexer::Priority;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// @design:FR-CLI-GRAPH-02
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Req,
    Design,
    Impl,
    Test,
    Adr,
}

// @impl:FR-CLI-GRAPH-02-B
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNode {
    pub kind: NodeKind,
    pub id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
}

// @impl:FR-CLI-GRAPH-02-C
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEdge {
    pub kind: EdgeKind,
    pub from: String,
    pub to: Vec<String>,
    pub note: Option<String>,
}

// @impl:FR-CLI-GRAPH-02-D
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageViolation {
    pub rule_id: String,
    pub subject: String,
    /// Observed count for the rule's subject (e.g. incoming edges, declarations).
    pub found: usize,
    /// Required threshold: `min` from the policy rule, or the structural
    /// maximum allowed (1 declaration per ID, 0 dangling references).
    pub required: usize,
}

// @impl:FR-CLI-GRAPH-02-E
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphReport {
    pub nodes: Vec<TraceNode>,
    pub edges: Vec<TraceEdge>,
    pub violations: Vec<CoverageViolation>,
    pub is_closed: bool,
}
