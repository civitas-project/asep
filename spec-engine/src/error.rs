use std::process::ExitCode;
use thiserror::Error;

// @design:FR-CLI-GRAPH-01
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AsepError {
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization failure: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Regex compilation failure: {0}")]
    Regex(#[from] regex::Error),

    #[error("Traceability gate broken: {0} requirements lack implementation or passing tests")]
    BrokenTraceability(usize),

    #[error("Docstring contract violations detected: {0}")]
    DocContractViolations(usize),

    #[error("Invalid project state: {0}")]
    InvalidState(String),

    #[error("Diagram compilation failure: {0}")]
    Diagram(String),
}

impl AsepError {
    // @impl:FR-CLI-GRAPH-02
    #[allow(dead_code)]
    /// Maps domain error to standardized process exit codes.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::BrokenTraceability(_) | Self::DocContractViolations(_) => ExitCode::from(1),
            Self::Io(_)
            | Self::Serialization(_)
            | Self::Regex(_)
            | Self::InvalidState(_)
            | Self::Diagram(_) => ExitCode::from(2),
        }
    }
}
