//! Formatting logic for srcdump.

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tiktoken::{get_encoding, CoreBpe};

use crate::config::{Config, OutputFormat, OutputMode};

use tree_sitter::Query;

/// Header for NDJSON output.
#[derive(Debug, Serialize, Deserialize)]
struct DumpHeader {
    version: u32,
    schema: String,
    root: String,
    files: usize,
    total_tokens_est: usize,
    mode: String,
    generated: String,
}

/// File entry in NDJSON.
#[derive(Debug, Serialize, Deserialize)]
struct FileEntry {
    #[serde(rename = "type")]
    entry_type: String,
    path: String,
    lang: String,
    lines: usize,
    tokens_est: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<Vec<SymbolInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

/// Outcome of processing a single file: either an emitted entry with its
/// original token estimate, or a skip with a reason.
enum ProcessedFile {
    Emitted(FileEntry, usize),
    Skipped { path: String, reason: String },
}

/// Token accounting for a batch of files.
#[derive(Debug, Clone, Default)]
pub struct TokenCounts {
    /// Tokens estimated from original file content.
    pub original_tokens: usize,
    /// Tokens estimated from the exact emitted serialized entry.
    pub emitted_tokens: usize,
    /// Number of skipped files.
    pub skipped: usize,
    /// Human-readable skip reasons.
    pub skip_reasons: Vec<String>,
}

/// Symbol info from srcidx.
#[derive(Debug, Serialize, Deserialize)]
struct SymbolInfo {
    kind: String,
    name: String,
    line: usize,
    signature: String,
}

/// Source formatter.
pub struct Formatter {
    config: Config,
    parsers: srcidx::parser::ParserRegistry,
    tokenizer: &'static CoreBpe,
    comment_queries: HashMap<String, Query>,
}

/// End index (exclusive) of a raw string literal starting at `i`, e.g. `r#"..."#` or `br#"..."#`.
/// Returns `None` when `i` does not start a raw string literal.
fn raw_string_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'r' && !(bytes[i] == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'r') {
        return None;
    }

    let start = i + match bytes[i] {
        b'r' => 1,
        _ => 2,
    };

    let mut hashes = 0;
    while start + hashes < bytes.len() && bytes[start + hashes] == b'#' {
        hashes += 1;
    }
    if start + hashes >= bytes.len() || bytes[start + hashes] != b'"' {
        return None;
    }

    let mut cursor = start + hashes + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && cursor + hashes < bytes.len()
            && bytes[cursor + 1..=cursor + hashes]
                .iter()
                .all(|&b| b == b'#')
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(cursor)
}

/// End index (exclusive) of a string literal starting at `i`, e.g. `"..."` or `b"..."`.
/// Returns `None` when `i` does not start a string literal.
fn string_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'"' && !(bytes[i] == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'"') {
        return None;
    }

    let mut cursor = i;
    if bytes[cursor] == b'b' {
        cursor += 1;
    }
    cursor += 1; // opening quote

    while cursor < bytes.len() {
        let b = bytes[cursor];
        cursor += 1;
        if b == b'\\' && cursor < bytes.len() {
            cursor += 1;
            continue;
        }
        if b == b'"' {
            return Some(cursor);
        }
    }
    Some(cursor)
}

/// End index (inclusive) of a char literal starting at `i`, e.g. `'x'` or `'\n'`.
/// Lifetimes like `'static` or `'a` are ignored and return `None`.
fn char_lit_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'\'' {
        return None;
    }
    if i + 2 < bytes.len() && bytes[i + 1] != b'\\' && bytes[i + 2] == b'\'' {
        return Some(i + 2);
    }
    if i + 3 < bytes.len() && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
        return Some(i + 3);
    }
    None
}

impl Formatter {
    /// Create a new formatter.
    pub fn new(config: Config, parsers: srcidx::parser::ParserRegistry) -> Result<Self> {
        let tokenizer = get_encoding(&config.tokenizer)
            .ok_or_else(|| anyhow::anyhow!("Failed to load {} encoding", config.tokenizer))?;

        // Load comment queries for each language
        let mut comment_queries = HashMap::new();
        let languages = [
            ("rust", include_str!("queries/comments/rust.scm")),
            (
                "typescript",
                include_str!("queries/comments/typescript.scm"),
            ),
            (
                "javascript",
                include_str!("queries/comments/typescript.scm"),
            ),
            ("python", include_str!("queries/comments/python.scm")),
            ("dart", include_str!("queries/comments/dart.scm")),
            ("go", include_str!("queries/comments/go.scm")),
            ("php", include_str!("queries/comments/php.scm")),
            ("java", include_str!("queries/comments/java.scm")),
            ("typst", include_str!("queries/comments/typst.scm")),
        ];

        for (lang, query_str) in languages {
            let language = match srcidx::parser::ParserRegistry::get_language_for_id(lang) {
                Some(language) => language,
                None => continue,
            };
            if let Ok(query) = Query::new(&language, query_str) {
                comment_queries.insert(lang.to_string(), query);
            }
        }

        Ok(Self {
            config,
            parsers,
            tokenizer,
            comment_queries,
        })
    }

    /// Get the formatter's configuration.
    #[allow(dead_code)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Format multiple files and write to output (streaming).
    /// Delegates to `format_dispatch`; existing callers are unchanged.
    pub fn format_files<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        self.format_dispatch(files, writer)
    }

    /// Format files and write to output, branching on `config.format`.
    /// `Json` writes one JSON document with `files` as an array of entries;
    /// `Ndjson` writes one JSON object per line (header line + entry lines);
    /// `Text` writes plain text lines, not NDJSON. Unknown formats error.
    // @impl:FR-AUD-27
    pub fn format_dispatch<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        if self.config.format == OutputFormat::Json {
            return self.format_json(files, writer);
        }
        if self.config.format == OutputFormat::Ndjson {
            return self.format_ndjson(files, writer);
        }
        if self.config.format == OutputFormat::Text {
            return self.format_text(files, writer);
        }
        Err(anyhow::anyhow!(
            "unsupported output format: {:?}",
            self.config.format
        ))
    }

    /// Token accounting for `files`: tokens estimated from original content
    /// and from the exact emitted serialized entry, plus skip reasons for
    /// files that could not be emitted instead of silently dropping them.
    // @impl:FR-AUD-28
    pub fn token_counts(&self, files: &[PathBuf]) -> TokenCounts {
        let (entries, mut counts) = self.prepare_files(files);
        for entry in &entries {
            match serde_json::to_string(entry) {
                Ok(entry_json) => counts.emitted_tokens += self.tokenizer.count(&entry_json),
                Err(_) => {}
            }
        }
        counts
    }

    /// Process every file, returning emitted entries and partial counts
    /// (original tokens and skips). Emitted tokens are added by the caller
    /// at serialization time.
    fn prepare_files(&self, files: &[PathBuf]) -> (Vec<FileEntry>, TokenCounts) {
        let mut entries = Vec::new();
        let mut counts = TokenCounts::default();
        for chunk in files.chunks(256) {
            let results: Vec<_> = chunk
                .par_iter()
                .map(|path| self.process_file(path.as_path()))
                .collect();
            for result in results {
                match result {
                    ProcessedFile::Emitted(entry, original_tokens) => {
                        counts.original_tokens += original_tokens;
                        entries.push(entry);
                    }
                    ProcessedFile::Skipped { path, reason } => {
                        counts.skipped += 1;
                        counts.skip_reasons.push(format!("{path}: {reason}"));
                    }
                }
            }
        }
        (entries, counts)
    }

    /// Write `Json` mode: one JSON document; `files` is an array of entries.
    fn format_json<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        let (entries, mut counts) = self.prepare_files(&files);
        let mut entry_values = Vec::new();
        for entry in &entries {
            let value = serde_json::to_value(entry)?;
            counts.emitted_tokens += self.tokenizer.count(&value.to_string());
            entry_values.push(value);
        }
        let root = self.config.root.to_string_lossy().to_string();
        let mode = format!("{:?}", self.config.mode).to_lowercase();
        let generated = chrono::Utc::now().to_rfc3339();
        let doc = serde_json::json!({
            "version": 1u32,
            "schema": "srcdump.v1",
            "root": root,
            "files": entry_values,
            "total_tokens_est": counts.emitted_tokens,
            "mode": mode,
            "generated": generated,
        });
        let doc_json = serde_json::to_string(&doc)?;
        writer.write_all(doc_json.as_bytes())?;
        writer.write_all(b"\n")?;
        Ok(())
    }

    /// Write `Ndjson` mode: one JSON object per line (header line + entry lines).
    fn format_ndjson<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        let (entries, mut counts) = self.prepare_files(&files);
        let mut lines = Vec::new();
        for entry in &entries {
            let line = serde_json::to_string(entry)?;
            counts.emitted_tokens += self.tokenizer.count(&line);
            lines.push(line);
        }
        let header = DumpHeader {
            version: 1,
            schema: "srcdump.v1".to_string(),
            root: self.config.root.to_string_lossy().to_string(),
            files: entries.len(),
            total_tokens_est: counts.emitted_tokens,
            mode: format!("{:?}", self.config.mode).to_lowercase(),
            generated: chrono::Utc::now().to_rfc3339(),
        };
        let header_json = serde_json::to_string(&header)?;
        writer.write_all(header_json.as_bytes())?;
        writer.write_all(b"\n")?;
        for line in lines {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Write `Text` mode: plain text lines, not NDJSON.
    fn format_text<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        let (entries, mut counts) = self.prepare_files(&files);
        let mut blocks = Vec::new();
        for entry in &entries {
            let mut block = String::new();
            block.push_str(&format!("== {}\n", entry.path));
            block.push_str(&format!("lang: {}\n", entry.lang));
            block.push_str(&format!("tokens: {}\n", entry.tokens_est));
            let content = entry.content.as_deref().unwrap_or("");
            block.push_str(content);
            if !content.ends_with('\n') {
                block.push('\n');
            }
            block.push_str("\n");
            counts.emitted_tokens += self.tokenizer.count(&block);
            blocks.push(block);
        }
        let root = self.config.root.to_string_lossy().to_string();
        let mode = format!("{:?}", self.config.mode).to_lowercase();
        let generated = chrono::Utc::now().to_rfc3339();
        let mut out = String::new();
        out.push_str("srcdump text\n");
        out.push_str(&format!("root: {root}\n"));
        out.push_str(&format!("files: {}\n", blocks.len()));
        out.push_str(&format!("total_tokens_est: {}\n", counts.emitted_tokens));
        out.push_str(&format!("mode: {mode}\n"));
        out.push_str(&format!("generated: {generated}\n"));
        out.push_str("\n");
        for block in blocks {
            out.push_str(&block);
        }
        writer.write_all(out.as_bytes())?;
        Ok(())
    }

    /// Process a single file. Skipped files carry a reason instead of
    /// vanishing silently.
    fn process_file(&self, path: &Path) -> ProcessedFile {
        let rel_path = match path.strip_prefix(&self.config.root) {
            Ok(rel_path) => rel_path,
            Err(_) => {
                return ProcessedFile::Skipped {
                    path: path.to_string_lossy().to_string(),
                    reason: "outside root".to_string(),
                }
            }
        };
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                return ProcessedFile::Skipped {
                    path: path.to_string_lossy().to_string(),
                    reason: format!("unreadable: {err}"),
                }
            }
        };
        let lang = match self.detect_language(path) {
            Some(lang) => lang,
            None => {
                return ProcessedFile::Skipped {
                    path: path.to_string_lossy().to_string(),
                    reason: "undetectable language".to_string(),
                }
            }
        };

        let tokens_est = self.estimate_tokens(&content);
        let symbols = self.symbols_for(path, &content);
        let (processed_content, lines) = self.content_for_mode(&content, &lang);

        let entry = FileEntry {
            entry_type: "file".to_string(),
            path: rel_path.to_string_lossy().to_string(),
            lang,
            lines,
            tokens_est,
            symbols,
            content: (!self.config.no_content).then_some(processed_content),
        };

        ProcessedFile::Emitted(entry, tokens_est)
    }

    /// Symbol extraction; skipped unless symbols are enabled.
    fn symbols_for(&self, path: &Path, content: &str) -> Option<Vec<SymbolInfo>> {
        if self.config.include_symbols {
            return self.extract_symbols(path, content);
        }
        None
    }

    /// Content processing; skipped for symbol-only output modes.
    fn content_for_mode(&self, content: &str, lang: &str) -> (String, usize) {
        if self.config.mode == OutputMode::SymbolsOnly || self.config.mode == OutputMode::Signatures
        {
            // For symbol-only modes, don't process full content
            return (String::new(), 0);
        }
        self.format_content(content, lang)
    }

    /// Detect language from file extension.
    fn detect_language(&self, path: &Path) -> Option<String> {
        if let Some(parser_arc) = self.parsers.find_parser(path) {
            let parser = parser_arc.read().unwrap();
            return Some(parser.language_id().to_string());
        }
        path.extension()?.to_str().map(|s| s.to_string())
    }

    /// Format content based on mode.
    fn format_content(&self, content: &str, lang: &str) -> (String, usize) {
        let mut result = content.to_string();

        // Strip comments
        if self.config.strip_comments {
            result = self.strip_comments(&result, lang);
        }

        // Collapse blank lines
        if self.config.strip_blank_lines {
            result = self.collapse_blank_lines(&result);
        }

        // Truncate long lines
        if let Some(max_len) = self.config.max_line_length {
            result = self.truncate_lines(&result, max_len);
        }

        // Truncate total lines
        if let Some(max_lines) = self.config.max_lines {
            result = self.truncate_total_lines(&result, max_lines);
        }

        // Summary mode
        if self.config.mode == OutputMode::Summary {
            result = self.summarize(&result);
        }

        let lines = result.lines().count();
        (result, lines)
    }

    /// Strip comments using tree-sitter or lexical scanner.
    fn strip_comments(&self, content: &str, lang: &str) -> String {
        // For Rust, use lexical scanner that handles raw strings, char literals, and comments
        if lang == "rust" {
            return self.strip_comments_rust(content);
        }

        // Try to use tree-sitter query for accurate comment removal
        let query = match self.comment_queries.get(lang) {
            Some(query) => query,
            None => return self.strip_comments_simple(content, lang),
        };
        let language = match srcidx::parser::ParserRegistry::get_language_for_id(lang) {
            Some(language) => language,
            None => return self.strip_comments_simple(content, lang),
        };
        self.strip_comments_with_query(content, query, language)
    }

    /// Strip comments using tree-sitter query for accurate removal.
    #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
    fn strip_comments_with_query(
        &self,
        content: &str,
        query: &Query,
        language: tree_sitter::Language,
    ) -> String {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();
        let tree = parser.parse(content, None).unwrap();

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(query, tree.root_node(), content.as_bytes());

        // Collect comment ranges
        let mut comment_ranges = Vec::new();
        use tree_sitter::StreamingIterator;
        while let Some(match_) = matches.next() {
            for capture in match_.captures() {
                if capture.index == 0 {
                    // @comment capture
                    let range = capture.node.range();
                    comment_ranges.push((range.start_byte, range.end_byte));
                }
            }
        }

        // Sort and merge overlapping ranges
        comment_ranges.sort_by_key(|&(start, _)| start);
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in comment_ranges {
            let extend = match merged.last() {
                Some(&(_, last_end)) => start <= last_end,
                None => false,
            };
            match extend {
                true => {
                    if let Some(last) = merged.last_mut() {
                        last.1 = last.1.max(end);
                    }
                }
                false => merged.push((start, end)),
            }
        }

        // Build result by copying non-comment parts
        let mut result = String::with_capacity(content.len());
        let mut last_end = 0;
        for (start, end) in merged {
            result.push_str(&content[last_end..start]);
            last_end = end;
        }
        result.push_str(&content[last_end..]);
        result
    }

    /// Fallback: simple but safer approach for languages without comment queries.
    fn strip_comments_simple(&self, content: &str, _lang: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        for line in lines {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("--") {
                continue;
            }
            result.push(line);
        }

        result.join("\n")
    }

    /// Strip comments from Rust code while preserving string and char literals.
    fn strip_comments_rust(&self, content: &str) -> String {
        let bytes = content.as_bytes();
        let len = bytes.len();
        let mut result = String::with_capacity(len);
        let mut i = 0;

        while i < len {
            if let Some(end) = raw_string_end(bytes, i) {
                result.push_str(&content[i..end]);
                i = end;
                continue;
            }
            if let Some(end) = string_end(bytes, i) {
                result.push_str(&content[i..end]);
                i = end;
                continue;
            }
            if let Some(end) = char_lit_end(bytes, i) {
                result.push_str(&content[i..=end]);
                i = end + 1;
                continue;
            }

            // Line comment //
            if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Block comment /*
            if let Some(next_i) = Self::skip_block_comment(bytes, i, len) {
                i = next_i;
                continue;
            }

            // Normal character (handling UTF-8)
            let ch = content[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }

        result
    }

    // @impl:FR-AUD-26
    /// Advance past a block comment (`/* ... */`), handling nested comments.
    /// Returns `None` if `bytes[i..]` does not start with `/*` (without advancing `i`),
    /// or if the comment is unterminated.
    /// Returns the index just after the closing `*/` when terminated.
    fn skip_block_comment(bytes: &[u8], mut i: usize, len: usize) -> Option<usize> {
        if !(i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*') {
            return None;
        }
        i += 2;
        let mut depth = 1;
        while i < len && depth > 0 {
            if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                depth += 1;
                i += 2;
                continue;
            }
            if bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                depth -= 1;
                i += 2;
                continue;
            }
            i += 1;
        }
        (depth == 0).then_some(i)
    }

    /// Collapse multiple blank lines to single, preserving trailing newline.
    fn collapse_blank_lines(&self, content: &str) -> String {
        let mut result = String::new();
        let mut blank_count = 0;
        let mut last_was_newline = false;

        for line in content.lines() {
            let is_blank = line.trim().is_empty();

            if is_blank {
                blank_count += 1;
            }
            if is_blank && blank_count == 1 {
                result.push('\n');
            }
            if !is_blank {
                blank_count = 0;
            }
            if !is_blank && !last_was_newline && !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            if !is_blank {
                result.push_str(line);
                result.push('\n');
            }
            last_was_newline = is_blank;
        }

        // Preserve trailing newline status
        if !content.ends_with('\n') {
            while result.ends_with('\n') {
                result.pop();
            }
        }

        result
    }

    // @impl:FR-AUD-33
    /// Truncate lines longer than `max_len`, handling UTF-8 correctly.
    /// The emitted line is at most `max_len` chars including the ellipsis.
    /// For `max_len < 4`, a long line becomes the ellipsis truncated to `max_len`.
    fn truncate_lines(&self, content: &str, max_len: usize) -> String {
        let ellipsis = "...";
        let keep = max_len.saturating_sub(ellipsis.chars().count());

        content
            .lines()
            .map(|line| {
                let chars: Vec<char> = line.chars().collect();
                if chars.len() <= max_len {
                    return line.to_string();
                }
                if keep == 0 {
                    return ellipsis.chars().take(max_len).collect();
                }
                let head = keep / 2;
                let tail = keep - head;
                let head_str: String = chars[..head].iter().collect();
                let tail_str: String = chars[chars.len() - tail..].iter().collect();
                format!("{head_str}{ellipsis}{tail_str}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Truncate total lines.
    fn truncate_total_lines(&self, content: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= max_lines {
            return content.to_string();
        }
        let mut truncated = lines[..max_lines].join("\n");
        truncated.push_str(&format!("\n... [truncated at {} lines]", max_lines));
        truncated
    }

    /// Keep the first 50 lines and last 50 lines, replacing the omitted middle with an ellipsis marker.
    fn summarize(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= 100 {
            return content.to_string();
        }

        let keep = 50;
        let mut result = Vec::new();
        result.extend(lines.iter().take(keep).map(|s| s.to_string()));
        let omitted = lines.len().saturating_sub(keep * 2);
        result.push(format!("... [{omitted} lines omitted] ..."));
        result.extend(
            lines
                .iter()
                .skip(lines.len().saturating_sub(keep))
                .map(|s| s.to_string()),
        );
        result.join("\n")
    }

    /// Estimate tokens using tiktoken on original content.
    fn estimate_tokens(&self, content: &str) -> usize {
        self.tokenizer.count(content)
    }

    /// Extract symbols using srcidx parser.
    fn extract_symbols(&self, path: &Path, content: &str) -> Option<Vec<SymbolInfo>> {
        self.parsers
            .with_parser(path, |parser| {
                let result = parser.parse_file(path, content);
                result
                    .symbols
                    .iter()
                    .map(|s| SymbolInfo {
                        kind: format!("{:?}", s.kind).to_lowercase(),
                        name: s.name.clone(),
                        line: s.line,
                        signature: s.signature.clone(),
                    })
                    .collect()
            })
            .filter(|symbols: &Vec<SymbolInfo>| !symbols.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use srcidx::parser::ParserRegistry;

    fn test_formatter() -> Formatter {
        let config = Config::default();
        let parsers = ParserRegistry::default();
        Formatter::new(config, parsers).unwrap()
    }

    #[test]
    fn test_strip_comments_rust() {
        let formatter = test_formatter();
        let code = r##"
// This is a line comment
fn main() {
    let url = "https://example.com//not_a_comment";
    let raw = r#"raw string with // and /* not comments */"#;
    let ch = '/';
    /* Block comment
       with multiple lines */
    println!("{}", url); /* inline block */ // trailing comment
}
"##;
        let stripped = formatter.strip_comments(code, "rust");
        assert!(!stripped.contains("This is a line comment"));
        assert!(!stripped.contains("Block comment"));
        assert!(!stripped.contains("inline block"));
        assert!(!stripped.contains("trailing comment"));
        assert!(stripped.contains("https://example.com//not_a_comment"));
        assert!(stripped.contains(r##"r#"raw string with // and /* not comments */"#"##));
        assert!(stripped.contains("let ch = '/';"));
    }

    #[test]
    fn test_strip_comments_rust_utf8_literal() {
        let formatter = test_formatter();
        let code = r#"let s = "Halo 🚀 Dunia 🌍 // not a comment";"#;
        let stripped = formatter.strip_comments(code, "rust");
        assert_eq!(stripped, code);

        let raw_code = r##"let s = r#"Halo 🚀 Dunia 🌍 // not a comment"#;"##;
        let stripped_raw = formatter.strip_comments(raw_code, "rust");
        assert_eq!(stripped_raw, raw_code);
    }

    #[test]
    fn test_strip_comments_typescript() {
        let formatter = test_formatter();
        let code = r#"
// TypeScript comment
const url = "https://example.com/api";
/* Block comment */
function test() {
    return "/* not a comment */";
}
"#;
        let stripped = formatter.strip_comments(code, "typescript");
        assert!(!stripped.contains("TypeScript comment"));
        assert!(!stripped.contains("Block comment"));
        assert!(stripped.contains("https://example.com/api"));
        assert!(stripped.contains("/* not a comment */"));
    }

    #[test]
    fn test_strip_comments_python() {
        let formatter = test_formatter();
        let code = r##"
# Python comment
url = "https://example.com#anchor"
def foo():
    return "# not comment"
"##;
        let stripped = formatter.strip_comments(code, "python");
        assert!(!stripped.contains("Python comment"));
        assert!(stripped.contains("https://example.com#anchor"));
        assert!(stripped.contains("# not comment"));
    }

    #[test]
    fn test_collapse_blank_lines() {
        let formatter = test_formatter();
        let text = "line 1\n\n\n\nline 2\n\nline 3\n";
        let collapsed = formatter.collapse_blank_lines(text);
        assert_eq!(collapsed, "line 1\n\nline 2\n\nline 3\n");
    }

    #[test]
    fn test_collapse_blank_lines_trailing_newline_preserved() {
        let formatter = test_formatter();
        assert_eq!(formatter.collapse_blank_lines("hello\n"), "hello\n");
        assert_eq!(
            formatter.collapse_blank_lines("line 1\n\n\nline 2\n"),
            "line 1\n\nline 2\n"
        );
        assert_eq!(formatter.collapse_blank_lines("line 1\n\n"), "line 1\n\n");
        assert_eq!(formatter.collapse_blank_lines("\n"), "\n");
    }

    #[test]
    fn test_collapse_blank_lines_no_trailing_newline_when_absent() {
        let formatter = test_formatter();
        assert_eq!(formatter.collapse_blank_lines("hello"), "hello");
        assert_eq!(
            formatter.collapse_blank_lines("line 1\n\n\nline 2"),
            "line 1\n\nline 2"
        );
        assert_eq!(formatter.collapse_blank_lines(""), "");
        assert_eq!(formatter.collapse_blank_lines("   "), "");
    }

    #[test]
    fn test_collapse_blank_lines_multiple_to_single() {
        let formatter = test_formatter();
        assert_eq!(formatter.collapse_blank_lines("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(formatter.collapse_blank_lines("\n\n\na\n"), "\na\n");
        assert_eq!(formatter.collapse_blank_lines("a\n\n\n\n"), "a\n\n");
    }

    #[test]
    fn test_collapse_blank_lines_idempotence() {
        let formatter = test_formatter();
        let cases = [
            "",
            "\n",
            "\n\n",
            "\n\n\n",
            "hello",
            "hello\n",
            "line 1\n\nline 2",
            "line 1\n\n\n\nline 2",
            "line 1\n\n\n\nline 2\n",
            "line 1\n\n",
            "line 1\n\n\n\n",
            "\n\n\nline 1\n\n\n",
            "   ",
            "   \n   \n",
            "line 1\n   \n   \nline 2",
            "line 1\n   \n   \nline 2\n",
            "line 1\r\n\r\n\r\nline 2\r\n",
        ];

        for &case in &cases {
            let once = formatter.collapse_blank_lines(case);
            let twice = formatter.collapse_blank_lines(&once);
            assert_eq!(once, twice, "Idempotence failed for input: {:?}", case);
            assert_eq!(
                once.ends_with('\n'),
                case.ends_with('\n'),
                "trailing newline mismatch for input: {:?}",
                case
            );
        }
    }

    #[test]
    fn test_truncate_lines() {
        let formatter = test_formatter();
        let long_line = "a".repeat(100);
        let truncated = formatter.truncate_lines(&long_line, 10);
        assert_eq!(truncated, "aaa...aaaa");
        assert_eq!(truncated.chars().count(), 10);

        let small = formatter.truncate_lines("test", 2);
        assert_eq!(small.chars().count(), 2);
        assert_ne!(small, "test");
    }

    #[test]
    fn test_summarize() {
        let formatter = test_formatter();
        let lines: Vec<String> = (1..=200).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let summarized = formatter.summarize(&content);
        assert!(summarized.contains("line 1"));
        assert!(summarized.contains("line 50"));
        assert!(summarized.contains("lines omitted"));
        assert!(summarized.contains("line 151"));
        assert!(summarized.contains("line 200"));
        assert!(!summarized.contains("line 100\n"));
    }

    #[test]
    fn test_format_files_parallel() {
        let temp_dir = std::env::temp_dir().join(format!(
            "srcdump_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        let mut test_files = Vec::new();
        for i in 0..10 {
            let file_path = temp_dir.join(format!("file_{i}.rs"));
            fs::write(
                &file_path,
                format!("// Comment in file {i}\npub fn foo_{i}() {{}}\n"),
            )
            .unwrap();
            test_files.push(file_path);
        }

        let config = Config {
            root: temp_dir.clone(),
            strip_comments: true,
            ..Config::default()
        };
        let parsers = ParserRegistry::default();
        let mut formatter = Formatter::new(config, parsers).unwrap();

        let mut output = Vec::new();
        formatter
            .format_files(test_files.clone(), &mut output)
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 11);

        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["schema"], "srcdump.v1");
        assert_eq!(header["files"], 10);

        for (i, line) in lines[1..].iter().enumerate() {
            let entry: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(entry["type"], "file");
            assert_eq!(entry["path"], format!("file_{i}.rs"));
            let content = entry["content"].as_str().unwrap();
            assert!(!content.contains("Comment"));
            assert!(content.contains(&format!("pub fn foo_{i}()")));
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_custom_tokenizer_configuration() {
        let config = Config {
            tokenizer: "p50k_base".to_string(),
            ..Config::default()
        };
        let parsers = ParserRegistry::default();
        let formatter = Formatter::new(config, parsers).unwrap();

        let sample_text = "pub fn calculate(x: i32, y: i32) -> i32 { x + y }";
        let tokens = formatter.estimate_tokens(sample_text);

        let p50k = get_encoding("p50k_base").unwrap();
        assert_eq!(tokens, p50k.count(sample_text));
        assert!(tokens > 0);

        // Also verify o200k_base
        let o200k_config = Config {
            tokenizer: "o200k_base".to_string(),
            ..Config::default()
        };
        let o200k_formatter = Formatter::new(o200k_config, ParserRegistry::default()).unwrap();
        assert_eq!(
            o200k_formatter.estimate_tokens(sample_text),
            get_encoding("o200k_base").unwrap().count(sample_text)
        );

        // Verify invalid tokenizer returns error
        let invalid_config = Config {
            tokenizer: "invalid_tokenizer".to_string(),
            ..Config::default()
        };
        let res = Formatter::new(invalid_config, ParserRegistry::default());
        match res {
            Ok(_) => panic!("expected error for invalid tokenizer"),
            Err(err) => assert!(err
                .to_string()
                .contains("Failed to load invalid_tokenizer encoding")),
        }
    }

    // @test:FR-AUD-26
    #[test]
    fn comment_stripper_preserves_surrounding_code() {
        let formatter = test_formatter();
        let code = "let a = 1; /* comment */ let b = 2;";
        let stripped = formatter.strip_comments(code, "rust");
        assert!(stripped.contains("let a = 1;"));
        assert!(stripped.contains("let b = 2;"));
        assert!(!stripped.contains("comment"));
    }

    #[test]
    fn comment_stripper_handles_nested_string_unterminated() {
        let formatter = test_formatter();

        let nested = "/* outer /* inner */ end */ code";
        let stripped = formatter.strip_comments(nested, "rust");
        assert_eq!(stripped, " code");

        let with_string = r#"let s = "/* not a comment */"; let x = 1;"#;
        let stripped = formatter.strip_comments(with_string, "rust");
        assert_eq!(stripped, with_string);

        let unterminated = "let a = 1; /* unterminated";
        let stripped = formatter.strip_comments(unterminated, "rust");
        assert_eq!(stripped, unterminated);
    }

    // @test:FR-AUD-33
    #[test]
    fn truncate_respects_max_len_with_ellipsis() {
        let formatter = test_formatter();
        let ascii = "a".repeat(100);
        let unicode = "ｈｅｌｌｏｗｏｒｌｄ".repeat(10);
        let cases = [&ascii, &unicode];
        for max_len in 0..12 {
            for line in cases {
                let result = formatter.truncate_lines(line, max_len);
                assert!(
                    result.chars().count() <= max_len,
                    "max_len {max_len}: {:?} is {} chars",
                    result,
                    result.chars().count()
                );
            }
        }
    }

    // @test:FR-AUD-27
    #[test]
    fn fr_aud_27_format_dispatch_modes() {
        let temp_dir = std::env::temp_dir().join(format!(
            "srcdump_fr_aud_27_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let file_a = temp_dir.join("a.rs");
        let file_b = temp_dir.join("b.rs");
        fs::write(&file_a, "pub fn foo() {}\n").unwrap();
        fs::write(&file_b, "pub fn bar() {}\n").unwrap();
        let files = vec![file_a.clone(), file_b.clone()];

        let parsers = ParserRegistry::default();

        // JSON: one document, `files` is an array of entries.
        let mut config = Config::default();
        config.root = temp_dir.clone();
        config.format = OutputFormat::Json;
        let mut formatter = Formatter::new(config.clone(), parsers.clone()).unwrap();
        let mut output = Vec::new();
        formatter
            .format_dispatch(files.clone(), &mut output)
            .unwrap();
        let output_str = String::from_utf8(output).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&output_str).unwrap();
        assert_eq!(doc["schema"], "srcdump.v1");
        let entries = doc["files"].as_array().expect("files is an array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["type"], "file");
        assert!(doc["total_tokens_est"].as_u64().unwrap() > 0);

        // NDJSON: every line parses as a JSON object.
        config.format = OutputFormat::Ndjson;
        let mut formatter = Formatter::new(config.clone(), parsers.clone()).unwrap();
        let mut output = Vec::new();
        formatter
            .format_dispatch(files.clone(), &mut output)
            .unwrap();
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert!(lines.len() >= 3);
        for line in &lines {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(value.is_object());
        }
        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["schema"], "srcdump.v1");
        assert_eq!(header["files"], 2);

        // Text: plain text lines, not NDJSON.
        config.format = OutputFormat::Text;
        let mut formatter = Formatter::new(config, parsers).unwrap();
        let mut output = Vec::new();
        formatter.format_dispatch(files, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert!(!lines.is_empty());
        assert!(lines
            .iter()
            .any(|line| serde_json::from_str::<serde_json::Value>(line).is_err()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    // @test:FR-AUD-28
    #[test]
    fn fr_aud_28_token_counts_and_skips() {
        let temp_dir = std::env::temp_dir().join(format!(
            "srcdump_fr_aud_28_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        let long_file = temp_dir.join("long.rs");
        let content = (0..300)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&long_file, &content).unwrap();

        let missing_file = temp_dir.join("missing.rs");

        let mut config = Config::default();
        config.root = temp_dir.clone();
        config.max_lines = Some(10);
        let parsers = ParserRegistry::default();
        let formatter = Formatter::new(config, parsers).unwrap();

        let counts = formatter.token_counts(&[long_file, missing_file]);
        assert_eq!(counts.skipped, 1);
        assert!(!counts.skip_reasons.is_empty());
        assert!(counts.skip_reasons[0].contains("unreadable"));
        assert_ne!(counts.original_tokens, counts.emitted_tokens);
        assert!(counts.emitted_tokens > 0);
        assert!(counts.original_tokens > 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
