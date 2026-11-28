pub mod builder;
pub mod model;

pub use crate::scanner::lexer::{Priority, TagKind};
pub use builder::{build_relation_graph, admissible_edge};
pub use model::{
    CoverageViolation, GraphReport, NodeKind, TraceEdge, TraceNode,
};
