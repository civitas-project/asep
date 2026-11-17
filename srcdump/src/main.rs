//! srcdump: Source code dumper for LLM consumption.
//!
//! Streams source files as token-efficient NDJSON with configurable formatting modes.

#![deny(unsafe_code)]
#![deny(missing_docs)]

use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use tracing::{info, Level};

mod config;
mod formatter;
mod walker;

use config::{Config, OutputFormat, OutputMode};
use formatter::Formatter;
use walker::walk_files;

/// Output format for dumped source.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliOutputFormat {
    Ndjson,
    Json,
    Text,
}

/// Output mode for content formatting.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliOutputMode {
    Full,
    SymbolsOnly,
    Signatures,
    Summary,
}

/// Configuration options.
#[derive(Parser, Debug)]
#[command(name = "srcdump", version, about = "Source code dumper for LLM consumption")]
struct Cli {
    /// Root directory to dump
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = CliOutputFormat::Ndjson)]
    format: CliOutputFormat,

    /// Output mode
    #[arg(short, long, value_enum, default_value_t = CliOutputMode::Full)]
    mode: CliOutputMode,

    /// Include file extensions (comma-separated)
    #[arg(short = 'i', long, value_delimiter = ',')]
    include_ext: Vec<String>,

    /// Exclude file extensions (comma-separated)
    #[arg(short = 'e', long, value_delimiter = ',')]
    exclude_ext: Vec<String>,

    /// Include glob patterns
    #[arg(short = 'I', long, value_delimiter = ',')]
    include: Vec<String>,

    /// Exclude glob patterns
    #[arg(short = 'E', long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Strip all comments
    #[arg(long, default_value_t = true)]
    strip_comments: bool,

    /// Collapse multiple blank lines to single
    #[arg(long, default_value_t = true)]
    strip_blank_lines: bool,

    /// Maximum lines per file (truncate longer files)
    #[arg(long)]
    max_lines: Option<usize>,

    /// Maximum line length (truncate longer lines)
    #[arg(long)]
    max_line_length: Option<usize>,

    /// Threshold for summary mode (files larger than this use summary)
    #[arg(long, default_value_t = 1000)]
    summary_threshold: usize,

    /// Include symbol annotations from srcidx
    #[arg(long)]
    include_symbols: bool,

    /// Output only metadata, no content
    #[arg(long)]
    no_content: bool,

    /// Show token estimates
    #[arg(long)]
    estimate_tokens: bool,

    /// Tokenizer encoding (cl100k_base, o200k_base, p50k_base, r50k_base)
    #[arg(long, default_value = "cl100k_base", value_parser = ["cl100k_base", "o200k_base", "p50k_base", "r50k_base"])]
    tokenizer: String,

    /// Respect .gitignore (default: true)
    #[arg(long, default_value_t = true)]
    gitignore: bool,

    /// Custom ignore file (gitignore syntax)
    #[arg(long)]
    ignore_file: Option<PathBuf>,

    /// Number of parallel workers
    #[arg(short = 'j', long, default_value_t = num_cpus::get())]
    jobs: usize,

    /// Follow symlinks (with safety limits)
    #[arg(long, default_value_t = true)]
    follow_symlinks: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Config file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Don't use default includes (empty include_ext means allow all)
    #[arg(long)]
    no_default_includes: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // Load config from file if provided
    let mut config = if let Some(config_path) = &cli.config {
        Config::from_file(config_path)?
    } else {
        Config::default()
    };

    // Override with CLI args
    config.root = cli.root.canonicalize()?;
    config.output = cli.output;
    config.format = match cli.format {
        CliOutputFormat::Ndjson => OutputFormat::Ndjson,
        CliOutputFormat::Json => OutputFormat::Json,
        CliOutputFormat::Text => OutputFormat::Text,
    };
    config.mode = match cli.mode {
        CliOutputMode::Full => OutputMode::Full,
        CliOutputMode::SymbolsOnly => OutputMode::SymbolsOnly,
        CliOutputMode::Signatures => OutputMode::Signatures,
        CliOutputMode::Summary => OutputMode::Summary,
    };
    if !cli.include_ext.is_empty() {
        config.include_ext = cli.include_ext;
    }
    if !cli.exclude_ext.is_empty() {
        config.exclude_ext = cli.exclude_ext;
    }
    if !cli.include.is_empty() {
        config.include_patterns = cli.include;
    }
    if !cli.exclude.is_empty() {
        config.exclude_patterns = cli.exclude;
    }
    config.strip_comments = cli.strip_comments;
    config.strip_blank_lines = cli.strip_blank_lines;
    config.max_lines = cli.max_lines;
    config.max_line_length = cli.max_line_length;
    config.summary_threshold = cli.summary_threshold;
    config.include_symbols = cli.include_symbols;
    config.no_content = cli.no_content;
    config.estimate_tokens = cli.estimate_tokens;
    config.tokenizer = cli.tokenizer;
    config.gitignore = cli.gitignore;
    config.ignore_file = cli.ignore_file;
    config.jobs = cli.jobs;
    config.follow_symlinks = cli.follow_symlinks;
    config.use_default_includes = !cli.no_default_includes;

    info!("Dumping: {}", config.root.display());
    info!("Mode: {:?}, Format: {:?}", config.mode, config.format);

    // Initialize srcidx parser registry
    let parsers = srcidx::parser::ParserRegistry::default();

    // Create formatter
    let mut formatter = Formatter::new(config.clone(), parsers)?;

    // Walk and dump files
    let files = walk_files(&config)?;
    info!("Found {} files", files.len());

    // Write output
    if let Some(output_path) = &config.output {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);
        formatter.format_files(files, &mut writer)?;
        info!("Written to: {}", output_path.display());
    } else {
        let mut writer = std::io::stdout();
        formatter.format_files(files, &mut writer)?;
    }

    Ok(())
}