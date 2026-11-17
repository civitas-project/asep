//! srcidx: Semantic source indexer for Rust.
//!
//! Extracts symbols (functions, structs, enums, traits, etc.) from Rust source code
//! and emits a self-describing NDJSON stream suitable for LLM consumption.
//!
//! # Traceability
//! - Infrastructure: `@impl-infra:UTIL-SRCIDX`

#![deny(unsafe_code)]
#![deny(missing_docs)]

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use tracing::{info, warn, Level};
use srcwalk::build_walker;

use srcidx::index::SymbolIndex;
use srcidx::parser::ParserRegistry;
use srcidx::emitter::{EmitterRegistry, IndexEmitter};

/// Output format.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Ndjson,
}

/// Configuration options.
#[derive(Parser, Debug)]
#[command(name = "srcidx", version, about = "Semantic source indexer for Rust")]
struct Cli {
    /// Root directory to index
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Ndjson)]
    format: OutputFormat,

    /// Include full JSON schema in header (larger output)
    #[arg(long, default_value_t = true)]
    include_schema: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Show JSON schema and exit
    #[arg(long)]
    show_schema: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    if cli.show_schema {
        println!("{}", srcidx::schema::schema_json_string());
        return Ok(());
    }

    let root = cli.root.canonicalize()?;
    info!("Indexing: {}", root.display());

    // Initialize registries
    let parsers = ParserRegistry::default();
    let _emitters = EmitterRegistry::default();

    let mut index = SymbolIndex::new(root.display().to_string());
    let mut files_processed = 0;
    let mut files_skipped = 0;

    // Walk the directory tree (respects .gitignore)
    let walker = build_walker(&srcwalk::WalkConfig::new(&root));

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("Walk error: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Check if we have a parser for this file
        if !parsers.can_parse(path) {
            continue;
        }

        // Read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                files_skipped += 1;
                continue;
            }
        };

        // Parse
        let result = parsers.parse_file(path, &content);
        for err in &result.errors {
            warn!("Parse error in {}: {}", path.display(), err);
        }

        if !result.errors.is_empty() && result.symbols.is_empty() {
            files_skipped += 1;
            continue;
        }

        for symbol in result.symbols {
            index.add_symbol(symbol);
        }

        files_processed += 1;
        if files_processed % 100 == 0 {
            info!("Processed {} files, {} symbols", files_processed, index.symbols.len());
        }
    }

    // Update header
    index.header.files = files_processed;
    index.header.skipped = files_skipped;

    info!("Indexing complete: {} files, {} symbols", files_processed, index.symbols.len());

    // Emit output
    let emitter = srcidx::emitter::ndjson::NdjsonEmitter::new();

    match cli.output {
        Some(path) => {
            let mut file = std::fs::File::create(&path)?;
            emitter.emit(&index, &mut file)?;
            info!("Output written to: {}", path.display());
        }
        None => {
            let mut stdout = std::io::stdout();
            emitter.emit(&index, &mut stdout)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use srcidx::index::SymbolIndex;
    use srcidx::parser::ParserRegistry;
    use std::path::Path;

    #[test]
    fn test_cli_integration() {
        // Test that the full pipeline works
        let code = r#"
/// @impl:FR-TEST-01
pub fn hello() -> &'static str {
    "world"
}
"#;
        let parsers = ParserRegistry::default();
        let result = parsers.parse_file(Path::new("test.rs"), code);
        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "hello");
        assert_eq!(result.symbols[0].tags, vec!["@impl:FR-TEST-01"]);

        let mut index = SymbolIndex::new("/test".to_string());
        index.header.files = 1;
        for sym in result.symbols {
            index.add_symbol(sym);
        }

        let emitter = srcidx::emitter::ndjson::NdjsonEmitter::new();
        let mut output = Vec::new();
        emitter.emit(&index, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.lines().count() == 2); // header + symbol
    }

    #[test]
    fn test_parse_failure_results() {
        let parsers = ParserRegistry::default();
        let result = parsers.parse_file(Path::new("broken.rs"), "fn broken {");
        assert!(!result.errors.is_empty());
        assert!(result.symbols.is_empty());
    }
}
