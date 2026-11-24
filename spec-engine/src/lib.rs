pub mod acceptance_scoped;
pub mod adr;
pub mod affected;
pub mod agent_protocol;
pub mod auditor;
pub mod ci;
pub mod clean;
pub mod cli;
pub mod config;
pub mod coverage;
pub mod diagram;
pub mod error;
pub mod formal;
pub mod graph;
pub mod harness;
pub mod init;
pub mod mcp;
pub mod observability;
pub mod pack;
pub mod policy;
pub mod runner;
pub mod scanner;
pub mod self_update;
pub mod skill;
pub mod snapshot;
pub mod verification;
pub mod watch;

// @impl:IN-01-COMPAT
// The observability module is additive: existing modules, config fields and
// MCP tools keep their names and behavior.

#[cfg(test)]
mod traceability_tests;
