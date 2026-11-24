//! Configuration for srcdump.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Ndjson,
    Json,
    Text,
}

/// Output mode for content.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Full,
    SymbolsOnly,
    Signatures,
    Summary,
}

/// Main configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Root directory to dump
    #[serde(skip)]
    pub root: PathBuf,

    /// Output file (stdout if None)
    #[serde(skip)]
    pub output: Option<PathBuf>,

    /// Output format
    #[serde(default = "default_format")]
    pub format: OutputFormat,

    /// Output mode
    #[serde(default = "default_mode")]
    pub mode: OutputMode,

    /// Include file extensions
    #[serde(default = "default_include_ext")]
    pub include_ext: Vec<String>,

    /// Exclude file extensions
    #[serde(default = "default_exclude_ext")]
    pub exclude_ext: Vec<String>,

    /// Include glob patterns
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// Exclude glob patterns
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// Strip all comments
    #[serde(default = "default_true")]
    pub strip_comments: bool,

    /// Collapse multiple blank lines
    #[serde(default = "default_true")]
    pub strip_blank_lines: bool,

    /// Maximum lines per file
    #[serde(default)]
    pub max_lines: Option<usize>,

    /// Maximum line length
    #[serde(default)]
    pub max_line_length: Option<usize>,

    /// Threshold for summary mode
    #[serde(default = "default_summary_threshold")]
    pub summary_threshold: usize,

    /// Include symbol annotations
    #[serde(default)]
    pub include_symbols: bool,

    /// Output only metadata, no content
    #[serde(default)]
    pub no_content: bool,

    /// Show token estimates
    #[serde(default)]
    pub estimate_tokens: bool,

    /// Tokenizer encoding name
    #[serde(default = "default_tokenizer")]
    pub tokenizer: String,

    /// Respect .gitignore
    #[serde(default = "default_true")]
    pub gitignore: bool,

    /// Custom ignore file
    #[serde(default)]
    pub ignore_file: Option<PathBuf>,

    /// Number of parallel workers
    #[serde(default = "default_jobs")]
    pub jobs: usize,

    /// Follow symlinks with safety
    #[serde(default = "default_true")]
    pub follow_symlinks: bool,

    /// Verbose logging
    #[serde(default)]
    pub verbose: bool,

    /// Config file path
    #[serde(skip)]
    #[allow(dead_code)]
    pub config_file: Option<PathBuf>,

    /// Use default includes (default: true). If false, empty include_ext means "allow all".
    #[serde(default = "default_true")]
    pub use_default_includes: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            output: None,
            format: OutputFormat::Ndjson,
            mode: OutputMode::Full,
            include_ext: default_include_ext(),
            exclude_ext: default_exclude_ext(),
            include_patterns: Vec::new(),
            exclude_patterns: default_exclude_patterns(),
            strip_comments: true,
            strip_blank_lines: true,
            max_lines: None,
            max_line_length: None,
            summary_threshold: 1000,
            include_symbols: false,
            no_content: false,
            estimate_tokens: false,
            tokenizer: default_tokenizer(),
            gitignore: true,
            ignore_file: None,
            jobs: num_cpus::get(),
            follow_symlinks: true,
            verbose: false,
            config_file: None,
            use_default_includes: true,
        }
    }
}

impl Config {
    /// Load config from TOML file.
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Check if a file extension is included.
    pub fn is_extension_included(&self, ext: &str) -> bool {
        if !self.include_ext.is_empty() && !self.include_ext.iter().any(|e| e == ext) {
            return false;
        }
        !self.exclude_ext.iter().any(|e| e == ext)
    }
}

fn default_format() -> OutputFormat {
    OutputFormat::Ndjson
}

fn default_mode() -> OutputMode {
    OutputMode::Full
}

fn default_true() -> bool {
    true
}

fn default_jobs() -> usize {
    num_cpus::get()
}

fn default_summary_threshold() -> usize {
    1000
}

fn default_include_ext() -> Vec<String> {
    vec![
        // Rust
        "rs".into(),
        // TypeScript/JavaScript
        "ts".into(),
        "tsx".into(),
        "js".into(),
        "jsx".into(),
        "mjs".into(),
        "cjs".into(),
        // Python
        "py".into(),
        "pyi".into(),
        "pyx".into(),
        "pyw".into(),
        // Dart
        "dart".into(),
        // Go
        "go".into(),
        // PHP
        "php".into(),
        "php8".into(),
        "phtml".into(),
        // Java
        "java".into(),
        // Kotlin
        "kt".into(),
        "kts".into(),
        // C#
        "cs".into(),
        // C/C++
        "cpp".into(),
        "cc".into(),
        "cxx".into(),
        "hpp".into(),
        "h".into(),
        "c".into(),
        // Ruby
        "rb".into(),
        // Perl
        "pl".into(),
        "pm".into(),
        // Shell
        "sh".into(),
        "bash".into(),
        "zsh".into(),
        "fish".into(),
        // PowerShell
        "ps1".into(),
        // Lua
        "lua".into(),
        // R
        "r".into(),
        // Julia
        "jl".into(),
        // Swift
        "swift".into(),
        // Objective-C
        "m".into(),
        "mm".into(),
        // Scala
        "scala".into(),
        // Clojure
        "clj".into(),
        "cljs".into(),
        // Haskell
        "hs".into(),
        // OCaml/F#
        "ml".into(),
        "fs".into(),
        "fsx".into(),
        // V
        "v".into(),
        // Zig
        "zig".into(),
        // Nim
        "nim".into(),
        // D
        "d".into(),
        // Elixir
        "ex".into(),
        "exs".into(),
        // Erlang
        "erl".into(),
        "hrl".into(),
        // Elm
        "elm".into(),
        // PureScript
        "purs".into(),
        // Reason/Rescript
        "res".into(),
        "rei".into(),
        // Config/Data
        "toml".into(),
        "json".into(),
        "yaml".into(),
        "yml".into(),
        "xml".into(),
        // Web
        "html".into(),
        "css".into(),
        "scss".into(),
        "sass".into(),
        "less".into(),
        "svg".into(),
        // TeX/LaTeX
        "tex".into(),
        "latex".into(),
        // Docs
        "md".into(),
        "mdx".into(),
        "txt".into(),
        "rst".into(),
        "adoc".into(),
    ]
}

fn default_exclude_ext() -> Vec<String> {
    vec![
        // Lock files
        "lock".into(),
        "sum".into(),
        "mod".into(),
        // Package manager
        "cargo".into(),
        "npm".into(),
        "pnpm".into(),
        "yarn".into(),
        "bun".into(),
        "gem".into(),
        "pip".into(),
        "poetry".into(),
        "pipenv".into(),
        "gradle".into(),
        "maven".into(),
        // Compiled
        "class".into(),
        "jar".into(),
        "war".into(),
        "ear".into(),
        "dll".into(),
        "so".into(),
        "dylib".into(),
        "exe".into(),
        "bin".into(),
        "obj".into(),
        "o".into(),
        "a".into(),
        "lib".into(),
        "pyc".into(),
        "pyo".into(),
        "pyd".into(),
        // Minified
        "min.js".into(),
        "min.css".into(),
        // Source maps
        "map".into(),
        // Archives
        "gz".into(),
        "zip".into(),
        "tar".into(),
        "tgz".into(),
        "xz".into(),
        "bz2".into(),
        "7z".into(),
        "rar".into(),
        // Images
        "png".into(),
        "jpg".into(),
        "jpeg".into(),
        "gif".into(),
        "bmp".into(),
        "ico".into(),
        "webp".into(),
        "avif".into(),
        // Fonts
        "ttf".into(),
        "woff".into(),
        "woff2".into(),
        "eot".into(),
        "otf".into(),
        // Audio/Video
        "mp3".into(),
        "mp4".into(),
        "wav".into(),
        "ogg".into(),
        "flac".into(),
        "mov".into(),
        "avi".into(),
        "mkv".into(),
        "webm".into(),
        // Documents
        "pdf".into(),
        "doc".into(),
        "docx".into(),
        "xls".into(),
        "xlsx".into(),
        "ppt".into(),
        "pptx".into(),
        // Databases
        "db".into(),
        "sqlite".into(),
        "sqlite3".into(),
        "realm".into(),
        "mdb".into(),
        "accdb".into(),
        // Logs/Cache
        "log".into(),
        "tmp".into(),
        "temp".into(),
        "cache".into(),
        "swp".into(),
        "swo".into(),
        "bak".into(),
        "backup".into(),
        "orig".into(),
        "old".into(),
        // OS
        "DS_Store".into(),
        "Thumbs.db".into(),
        "ehthumbs.db".into(),
        "desktop.ini".into(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        // Build directories (glob patterns with recursive wildcards)
        "target/**".into(),
        "node_modules/**".into(),
        ".git/**".into(),
        "dist/**".into(),
        "build/**".into(),
        "out/**".into(),
        ".next/**".into(),
        ".nuxt/**".into(),
        "vendor/**".into(),
        // IDE
        ".idea/**".into(),
        ".vscode/**".into(),
        "*.swp".into(),
        "*.swo".into(),
        // Test outputs
        "coverage/**".into(),
        ".nyc_output/**".into(),
    ]
}

fn default_tokenizer() -> String {
    "cl100k_base".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tokenizer() {
        let config = Config::default();
        assert_eq!(config.tokenizer, "cl100k_base");
    }
}
