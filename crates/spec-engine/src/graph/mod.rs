pub mod builder;
pub mod model;

pub use crate::scanner::lexer::{Priority, TagKind};
pub use builder::{build_relation_graph, build_trace_graph};
pub use model::{
    BrokenTrace, CoverageViolation, GraphReport, NodeKind, RequirementEntry, TraceEdge, TraceNode,
    TraceabilityReport,
};
