//! Formatting logic for srcdump.

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tiktoken::{CoreBpe, get_encoding};

use crate::config::{Config, OutputMode};

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

impl Formatter {
    /// Create a new formatter.
    pub fn new(config: Config, parsers: srcidx::parser::ParserRegistry) -> Result<Self> {
        let tokenizer = get_encoding(&config.tokenizer)
            .ok_or_else(|| anyhow::anyhow!("Failed to load {} encoding", config.tokenizer))?;

        // Load comment queries for each language
        let mut comment_queries = HashMap::new();
        let languages = [
            ("rust", include_str!("queries/comments/rust.scm")),
            ("typescript", include_str!("queries/comments/typescript.scm")),
            ("javascript", include_str!("queries/comments/typescript.scm")),
            ("python", include_str!("queries/comments/python.scm")),
            ("dart", include_str!("queries/comments/dart.scm")),
            ("go", include_str!("queries/comments/go.scm")),
            ("php", include_str!("queries/comments/php.scm")),
            ("java", include_str!("queries/comments/java.scm")),
            ("typst", include_str!("queries/comments/typst.scm")),
        ];

        for (lang, query_str) in languages {
            if let Some(language) = srcidx::parser::ParserRegistry::get_language_for_id(lang) {
                if let Ok(query) = Query::new(&language, query_str) {
                    comment_queries.insert(lang.to_string(), query);
                }
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
    pub fn format_files<W: Write>(&mut self, files: Vec<PathBuf>, writer: &mut W) -> Result<()> {
        let header = DumpHeader {
            version: 1,
            schema: "srcdump.v1".to_string(),
            root: self.config.root.to_string_lossy().to_string(),
            files: files.len(),
            total_tokens_est: 0,
            mode: format!("{:?}", self.config.mode).to_lowercase(),
            generated: chrono::Utc::now().to_rfc3339(),
        };

        let header_json = serde_json::to_string(&json!({
            "version": header.version,
            "schema": header.schema,
            "root": header.root,
            "files": header.files,
            "total_tokens_est": 0,
            "mode": header.mode,
            "generated": header.generated,
        }))?;
        writer.write_all(header_json.as_bytes())?;
        writer.write_all(b"\n")?;

        for chunk in files.chunks(256) {
            let entries: Vec<_> = chunk
                .par_iter()
                .map(|path| self.process_file(path.as_path()))
                .collect();

            for (entry, _tokens) in entries.into_iter().flatten() {
                let entry_json = serde_json::to_string(&entry)?;
                writer.write_all(entry_json.as_bytes())?;
                writer.write_all(b"\n")?;
            }
        }

        Ok(())
    }

    /// Process a single file.
    fn process_file(&self, path: &Path) -> Option<(FileEntry, usize)> {
        let rel_path = path.strip_prefix(&self.config.root).ok()?;
        let content = fs::read_to_string(path).ok()?;
        let lang = self.detect_language(path)?;

        let tokens_est = self.estimate_tokens(&content);

        let (processed_content, lines, symbols) = if self.config.mode == OutputMode::SymbolsOnly || self.config.mode == OutputMode::Signatures {
            // For symbol-only modes, don't process full content
            let symbols = if self.config.include_symbols {
                self.extract_symbols(path, &content)
            } else {
                None
            };
            (String::new(), 0, symbols)
        } else {
            let (processed_content, lines) = self.format_content(&content, &lang);
            let symbols = if self.config.include_symbols {
                self.extract_symbols(path, &content)
            } else {
                None
            };
            (processed_content, lines, symbols)
        };

        let entry = FileEntry {
            entry_type: "file".to_string(),
            path: rel_path.to_string_lossy().to_string(),
            lang,
            lines,
            tokens_est,
            symbols,
            content: if self.config.no_content { None } else { Some(processed_content) },
        };

        Some((
            entry,
            if self.config.no_content {
                0
            } else {
                self.estimate_tokens(&content)
            },
        ))
    }

    /// Detect language from file extension.
    fn detect_language(&self, path: &Path) -> Option<String> {
        if let Some(parser_arc) = self.parsers.find_parser(path) {
            let parser = parser_arc.read().unwrap();
            Some(parser.language_id().to_string())
        } else {
            path.extension()?.to_str().map(|s| s.to_string())
        }
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
        if let Some(query) = self.comment_queries.get(lang) {
            if let Some(language) = srcidx::parser::ParserRegistry::get_language_for_id(lang) {
                return self.strip_comments_with_query(content, query, language);
            }
        }

        // Fallback: simple line-based approach
        self.strip_comments_simple(content, lang)
    }

    /// Strip comments using tree-sitter query for accurate removal.
    fn strip_comments_with_query(&self, content: &str, query: &Query, language: tree_sitter::Language) -> String {
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
                if capture.index == 0 { // @comment capture
                    let range = capture.node.range();
                    comment_ranges.push((range.start_byte, range.end_byte));
                }
            }
        }

        // Sort and merge overlapping ranges
        comment_ranges.sort_by_key(|&(start, _)| start);
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in comment_ranges {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                } else {
                    merged.push((start, end));
                }
            } else {
                merged.push((start, end));
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
            // Check raw string: r#*"..."#* or br#*"..."#*
            let is_raw = if bytes[i] == b'r' {
                Some(i + 1)
            } else if bytes[i] == b'b' && i + 1 < len && bytes[i + 1] == b'r' {
                Some(i + 2)
            } else {
                None
            };

            if let Some(start_hashes) = is_raw {
                let mut hashes = 0;
                while start_hashes + hashes < len && bytes[start_hashes + hashes] == b'#' {
                    hashes += 1;
                }
                if start_hashes + hashes < len && bytes[start_hashes + hashes] == b'"' {
                    let str_start = i;
                    i = start_hashes + hashes + 1;
                    while i < len {
                        if bytes[i] == b'"'
                            && i + hashes < len
                            && bytes[i + 1..=i + hashes].iter().all(|&b| b == b'#')
                        {
                            i += 1 + hashes;
                            break;
                        }
                        i += 1;
                    }
                    let str_end = i;
                    result.push_str(&content[str_start..str_end]);
                    continue;
                }
            }

            // Check regular string literal: "..." or b"..."
            let is_str = if bytes[i] == b'"' {
                true
            } else {
                bytes[i] == b'b' && i + 1 < len && bytes[i + 1] == b'"'
            };

            if is_str {
                let str_start = i;
                if bytes[i] == b'b' {
                    i += 1;
                }
                i += 1;
                while i < len {
                    let b = bytes[i];
                    i += 1;
                    if b == b'\\' && i < len {
                        i += 1;
                    } else if b == b'"' {
                        break;
                    }
                }
                let str_end = i;
                result.push_str(&content[str_start..str_end]);
                continue;
            }

            // Check char literal: 'x' or '\n' (ignore lifetimes like 'static or 'a)
            if bytes[i] == b'\'' {
                let is_char_lit = if i + 2 < len && bytes[i + 1] != b'\\' && bytes[i + 2] == b'\'' {
                    Some(i + 2)
                } else if i + 3 < len && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
                    Some(i + 3)
                } else {
                    None
                };

                if let Some(end_char) = is_char_lit {
                    result.push_str(&content[i..=end_char]);
                    i = end_char + 1;
                    continue;
                }
            }

            // Check line comment //
            if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Check block comment /*
            if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i += 2;
                let mut depth = 1;
                while i < len && depth > 0 {
                    if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && i + 1 < len && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }

            // Normal character (handling UTF-8)
            let ch = content[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }

        result
    }
    
    /// Collapse multiple blank lines to single, preserving trailing newline.
    fn collapse_blank_lines(&self, content: &str) -> String {
        let mut result = String::new();
        let mut blank_count = 0;
        let mut last_was_newline = false;

        for line in content.lines() {
            if line.trim().is_empty() {
                blank_count += 1;
                if blank_count == 1 {
                    result.push('\n');
                }
            } else {
                blank_count = 0;
                if last_was_newline {
                    // Already have newline from previous blank line handling
                } else if !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str(line);
                result.push('\n');
            }
            last_was_newline = line.trim().is_empty();
        }

        // Preserve trailing newline status
        if !content.ends_with('\n') {
            while result.ends_with('\n') {
                result.pop();
            }
        }

        result
    }

    /// Truncate lines longer than max_len, handling UTF-8 correctly.
    fn truncate_lines(&self, content: &str, max_len: usize) -> String {
        if max_len < 4 {
            return content.to_string(); // Too small to truncate meaningfully
        }

        content.lines()
            .map(|line| {
                if line.chars().count() > max_len {
                    let half = max_len / 2;
                    let chars: Vec<char> = line.chars().collect();
                    format!("{}...{}", chars[..half].iter().collect::<String>(), chars[chars.len()-half..].iter().collect::<String>())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Truncate total lines.
    fn truncate_total_lines(&self, content: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > max_lines {
            let mut truncated = lines[..max_lines].join("\n");
            truncated.push_str(&format!("\n... [truncated at {} lines]", max_lines));
            truncated
        } else {
            content.to_string()
        }
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
        result.extend(lines.iter().skip(lines.len().saturating_sub(keep)).map(|s| s.to_string()));
        result.join("\n")
    }

    /// Estimate tokens using tiktoken on original content.
    fn estimate_tokens(&self, content: &str) -> usize {
        self.tokenizer.count(content)
    }

    /// Extract symbols using srcidx parser.
    fn extract_symbols(&self, path: &Path, content: &str) -> Option<Vec<SymbolInfo>> {
        self.parsers.with_parser(path, |parser| {
            let result = parser.parse_file(path, content);
            result.symbols.iter().map(|s| SymbolInfo {
                kind: format!("{:?}", s.kind).to_lowercase(),
                name: s.name.clone(),
                line: s.line,
                signature: s.signature.clone(),
            }).collect()
        }).filter(|symbols: &Vec<SymbolInfo>| !symbols.is_empty())
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
        assert_eq!(formatter.collapse_blank_lines("line 1\n\n\nline 2\n"), "line 1\n\nline 2\n");
        assert_eq!(formatter.collapse_blank_lines("line 1\n\n"), "line 1\n\n");
        assert_eq!(formatter.collapse_blank_lines("\n"), "\n");
    }

    #[test]
    fn test_collapse_blank_lines_no_trailing_newline_when_absent() {
        let formatter = test_formatter();
        assert_eq!(formatter.collapse_blank_lines("hello"), "hello");
        assert_eq!(formatter.collapse_blank_lines("line 1\n\n\nline 2"), "line 1\n\nline 2");
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
            if case.ends_with('\n') {
                assert!(once.ends_with('\n'), "Expected trailing newline for input: {:?}", case);
            } else {
                assert!(!once.ends_with('\n'), "Expected no trailing newline for input: {:?}", case);
            }
        }
    }

    #[test]
    fn test_truncate_lines() {
        let formatter = test_formatter();
        let long_line = "a".repeat(100);
        let truncated = formatter.truncate_lines(&long_line, 10);
        assert_eq!(truncated, "aaaaa...aaaaa");

        let small = formatter.truncate_lines("test", 2);
        assert_eq!(small, "test");
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
        let temp_dir = std::env::temp_dir().join(format!("srcdump_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).unwrap();

        let mut test_files = Vec::new();
        for i in 0..10 {
            let file_path = temp_dir.join(format!("file_{i}.rs"));
            fs::write(&file_path, format!("// Comment in file {i}\npub fn foo_{i}() {{}}\n")).unwrap();
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
        formatter.format_files(test_files.clone(), &mut output).unwrap();

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
            Err(err) => assert!(err.to_string().contains("Failed to load invalid_tokenizer encoding")),
        }
    }
}
