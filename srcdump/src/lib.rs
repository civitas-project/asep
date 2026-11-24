//! srcdump library: Source code dumper for LLM consumption.
//!
//! Streams source files as token-efficient NDJSON with configurable formatting modes.

pub mod config;
pub mod formatter;
pub mod walker;

pub use config::{Config, OutputFormat, OutputMode};
pub use formatter::Formatter;
pub use walker::walk_files;
