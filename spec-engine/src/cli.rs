// @impl:FR-ACC-03 @impl:FR-ACC-04
// Clap derives generate `from_arg_matches`, which builds every enum variant on the stack in debug builds.
#![allow(clippy::large_stack_frames)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;

// @design:FR-CLI-SCAN-01-B
#[derive(Debug, Parser)]
#[command(
    name = "asep",
    alias = "spec-engine",
    author = "da578 & AI Assistant",
    version = env!("CARGO_PKG_VERSION"),
    about = "Axiomatic Systems Engineering Protocol (ASEP): Deterministic SDLC validator, traceability graph checker, and living spec engine"
)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Emit verbose diagnostic trace output"
    )]
    pub verbose: bool,

    #[arg(long, global = true, help = "Emit machine-parseable JSON summary")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Verify 1:1 traceability, docstring contracts, and governance maps
    Check {
        #[arg(help = "Target repository path to scan (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target repository path to scan")]
        path: Option<PathBuf>,

        #[arg(long, help = "Treat non-critical warnings as errors (Exit Code 1)")]
        strict: bool,

        #[arg(
            long,
            help = "Run directory containing a messages/ subdirectory with agent envelopes to validate"
        )]
        messages: Option<PathBuf>,

        #[arg(
            long,
            help = "Rigor tier override: 'T0'..'T4' (default: T1 for code, T0 for docs-only)"
        )]
        rigor: Option<String>,

        #[arg(
            long,
            help = "Path to write `.spec-engine/behavioral.json` (Gate 3 behavioral report)"
        )]
        behavioral_report: Option<PathBuf>,

        #[arg(
            long,
            help = "Deny all @req:*:DRAFT tags (enforced on main production branch)"
        )]
        deny_drafts: bool,

        #[arg(
            long,
            help = "Live continuous monitoring mode (re-evaluates on file changes)"
        )]
        watch: bool,

        #[arg(
            long,
            help = "Path to LCOV or coverage file to correlate with requirements"
        )]
        coverage: Option<PathBuf>,

        #[arg(
            long,
            help = "Print only the detailed skill drift report for installed skill copies"
        )]
        skills: bool,

        #[arg(
            long,
            help = "Directory containing semantic evidence files (default: <target>/evidence)"
        )]
        evidence_dir: Option<PathBuf>,
    },

    /// Manage Architecture Decision Records (ADRs)
    Adr {
        #[command(subcommand)]
        sub: AdrCommands,
    },

    /// Execute the full 5-gate local CI verification pipeline
    Ci {
        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,

        #[arg(
            long,
            help = "Run single gate only: 0 (Formal), 1 (Traceability), 2 (Clippy), 3 (Test), 4 (Bench)"
        )]
        gate: Option<u8>,

        #[arg(long, help = "Comma-separated gates to skip (e.g. '0,4')")]
        skip: Option<String>,

        #[arg(
            long,
            default_value = "T1",
            help = "Rigor tier: 'T0'..'T4' (default: T1)"
        )]
        rigor: Option<String>,
    },

    /// Compile PlantUML diagrams into SVG assets
    Render {
        #[arg(help = "Path to scan for diagram files (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Path to scan for diagram files")]
        path: Option<PathBuf>,
    },

    /// Scaffold canonical ASEP docs directory structure
    Init {
        #[arg(help = "Target repository path (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target repository path")]
        path: Option<PathBuf>,
    },

    /// Install plugins and skills into agentic AI environments (Antigravity, OpenCode, Claude Code, Cursor, Windsurf, Continue, Roo Code, Copilot, Goose, Zed, Aider)
    Pack {
        #[arg(long, help = "Install locally to current project")]
        local: bool,

        #[arg(long, help = "Install globally to user profile")]
        global: bool,

        #[arg(
            long,
            help = "Target agent: antigravity, opencode, claudecode, cursor, windsurf, continue, roo, copilot, goose, zed, aider, detected, all"
        )]
        agent: Option<String>,

        #[arg(
            short = 'y',
            long = "yes",
            help = "Skip interactive prompts and use defaults"
        )]
        yes: bool,

        #[arg(long, help = "Specific skills to unpack")]
        skills: Option<Vec<String>>,

        #[arg(long, help = "Target project directory")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,

        #[arg(help = "Target project directory (positional)")]
        pos_dir: Option<PathBuf>,
    },

    /// Run Model Context Protocol (MCP) server over standard I/O
    Mcp {
        #[arg(long, help = "Target repository path")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target repository path")]
        path: Option<PathBuf>,

        #[arg(help = "Target repository path (positional)")]
        pos_dir: Option<PathBuf>,
    },

    /// Self-update the asep binary from the latest Codeberg release
    Update {},

    /// Run scoped acceptance for a single commit (FR-ACC-02 through FR-ACC-04).
    AcceptScoped {
        /// Full commit hash to accept
        commit: String,
    },

    /// Run empirical complexity and memory harness commands (ARCH-10)
    Harness {
        #[command(subcommand)]
        sub: HarnessCommands,
    },

    /// Manage skills (list, inspect, add, edit, delete, restore, export, import)
    Skill {
        #[command(subcommand)]
        sub: Option<SkillCommands>,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// List tests impacted by changed files using the traceability graph
    Affected {
        /// Changed file or directory paths
        paths: Vec<PathBuf>,

        /// Read changed paths from stdin, one per line
        #[arg(long)]
        stdin: bool,

        /// Project root (default: current directory)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },

    /// Compile deterministic context bundle for changed paths
    Context {
        /// Changed file or directory paths
        paths: Vec<PathBuf>,

        /// Project root (default: current directory)
        #[arg(short, long)]
        root: Option<PathBuf>,

        /// Maximum token budget for the context bundle
        #[arg(long)]
        max_tokens: Option<u64>,
    },

    /// Measure or clean Rust target artifacts by size threshold
    Clean {
        /// Project root (default: current directory)
        #[arg(short, long)]
        root: Option<PathBuf>,

        /// Explicit threshold in GiB (overrides measured baseline)
        #[arg(long)]
        max_gb: Option<f64>,

        /// Target directory to measure/clean (default: <root>/target)
        #[arg(long)]
        target: Option<PathBuf>,

        /// Write the current target size as baseline and exit
        #[arg(long)]
        measure: bool,

        /// Report the plan without deleting anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Inspect and manage telemetry trace segments
    Trace {
        /// Project root (default: current directory)
        dir: Option<PathBuf>,

        #[command(subcommand)]
        action: TraceAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum TraceAction {
    /// Show whether telemetry is enabled and summarize trace segments
    Status,
    /// Print a summary of a trace grouped by task_id and event
    Report {
        /// Trace identifier to report on
        trace_id: String,
    },
    /// Remove all segment files for a trace
    Purge {
        /// Trace identifier to purge
        trace_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum AdrCommands {
    /// Scaffold a newly numbered Rich ADR
    New {
        #[arg(help = "Title of the architecture decision record")]
        title: String,

        #[arg(help = "Target repository path (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target repository path")]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SkillCommands {
    /// List all skills and their status (Builtin, Modified, Custom, Missing)
    List {
        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Create a new custom skill
    Add {
        #[arg(help = "Unique identifier / directory name")]
        id: String,

        #[arg(short, long, help = "Skill display name")]
        name: Option<String>,

        #[arg(short, long, help = "Skill description")]
        description: Option<String>,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Edit an existing skill in your configured or chosen editor
    Edit {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(long, help = "Auto-sync to all detected agents after editing")]
        sync: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Delete a skill from the local skills directory
    Delete {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(short = 'y', long = "yes", help = "Confirm deletion without prompt")]
        yes: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Restore built-in skills to their factory default content
    Restore {
        #[arg(short, long, help = "Skill identifiers to restore")]
        skills: Vec<String>,

        #[arg(long, help = "Restore all built-in skills to factory defaults")]
        all: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Export a custom or modified skill to a pure Markdown (.md) file
    Export {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(help = "Destination .md file path")]
        out: PathBuf,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Import a skill from a pure Markdown (.md) file
    Import {
        #[arg(help = "Source .md file to import")]
        file: PathBuf,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Configure editor preference and default settings
    Config {
        #[arg(long, help = "Set editor preference: ask, terminal, gui")]
        preference: Option<String>,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Validate a skill's format and metadata
    Validate {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Show diff between local skill and embedded original
    Diff {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Sync skills to all detected agentic AI tools
    Sync {
        #[arg(long, help = "Target agent (comma-separated) or 'all'/'detected'")]
        agent: Option<String>,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Rollback a skill to a previous version or embedded original
    Rollback {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(long, help = "Restore to specific backup timestamp")]
        to: Option<String>,

        #[arg(long, help = "List available backups")]
        list: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Show skill state and drift against last snapshot
    Status {
        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Rebuild skill state snapshot
    State {
        #[arg(long, help = "Rebuild state from current filesystem")]
        rebuild: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Generate or verify skills.lock file
    Lock {
        #[arg(long, help = "Verify lock file against current state")]
        verify: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Search skills across workspace and embedded catalogs
    Search {
        #[arg(help = "Search query keyword")]
        query: String,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Fork an existing skill into a new custom skill
    Fork {
        #[arg(help = "Source skill identifier to fork from")]
        src: String,

        #[arg(long, help = "New skill identifier")]
        id: String,

        #[arg(long, help = "New skill display name")]
        name: Option<String>,

        #[arg(long, help = "New skill description")]
        description: Option<String>,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Install a skill and its dependencies from embedded catalog
    Install {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(long, help = "Do not install dependencies automatically")]
        no_deps: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// 3-way merge conflict resolution for modified skills
    Merge {
        #[arg(help = "Skill identifier")]
        id: String,

        #[arg(long, help = "Remote skill file or content to merge against")]
        remote: Option<PathBuf>,

        #[arg(long, help = "Interactive conflict chunk resolution TUI")]
        interactive: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Purge a skill from workspace and cascade delete across agent tools
    Purge {
        #[arg(help = "Skill identifier to purge")]
        id: String,

        #[arg(long, help = "Comma-separated target agents to purge from")]
        agents: Option<String>,

        #[arg(short = 'y', long, help = "Skip interactive confirmation")]
        yes: bool,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Export skills and state into a portable bundle file
    BundleExport {
        #[arg(help = "Output bundle JSON file path")]
        out: PathBuf,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },

    /// Import skills and state from a portable bundle file
    BundleImport {
        #[arg(help = "Source bundle JSON file path to import")]
        file: PathBuf,

        #[arg(help = "Target project directory (positional)")]
        dir: Option<PathBuf>,

        #[arg(short, long, help = "Target project directory")]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum HarnessCommands {
    /// Measure empirical asymptotic complexity of an external command
    Complexity {
        #[arg(long, help = "Command template with '{n}' placeholder")]
        cmd: String,
        #[arg(
            long,
            default_value = "50,100,200,400,800",
            help = "Comma-separated input sizes"
        )]
        sizes: String,
        #[arg(long, default_value_t = 1.2, help = "Maximum acceptable log-log slope")]
        max_slope: f64,
        #[arg(long, default_value = "10s", help = "Timeout per run")]
        timeout: String,
    },
    /// Measure peak process memory (RSS) and enforce budget
    Memory {
        #[arg(long, help = "Command to run")]
        cmd: String,
        #[arg(long, help = "Peak memory budget (e.g. '64MB', '512KB')")]
        peak: String,
        #[arg(long, default_value = "30s", help = "Timeout")]
        timeout: String,
    },
    /// Create a new E2E run directory and manifest
    NewRun {
        /// Run identifier
        run_id: String,
        /// Run root (default: <temp-dir>/opencode/asep-e2e, env RUN_ROOT overrides)
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Vendor loom = "=0.7.3" into an E2E run directory and prove it works offline
    VendorLoom {
        /// E2E run directory
        run_dir: PathBuf,
        /// Path to a vendored loom tree (default: <repo_root>/harness/e2e/vendor-loom)
        #[arg(long)]
        vendor_path: Option<PathBuf>,
        /// Accepted for compatibility; cargo is always invoked --offline
        #[arg(long)]
        offline: bool,
    },
    /// Verify an ASEP E2E run directory (artifacts, hash lock, re-runs)
    Verify {
        /// E2E run directory
        run_dir: PathBuf,
    },
}
