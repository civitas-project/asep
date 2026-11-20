pub mod adr;
pub mod auditor;
pub mod cli;
pub mod config;
pub mod diagram;
pub mod error;
pub mod graph;
pub mod harness;
pub mod init;
pub mod formal;
pub mod ci;
pub mod coverage;
pub mod watch;
pub mod mcp;
pub mod pack;
pub mod policy;
pub mod runner;
pub mod verification;
pub mod self_update;
pub mod scanner;
pub mod skill;
pub mod snapshot;
pub mod agent_protocol;

#[cfg(test)]
mod traceability_tests;
