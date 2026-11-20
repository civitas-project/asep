//! Lightweight, dependency-free markup parsers.
//!
//! Parses Markdown/MDX, reStructuredText, and AsciiDoc with no external
//! parsing dependencies. Extracts headings, anchors, and traceability tags
//! (`@req:...`, `@design:...`, `@impl:...`, `@test:...`, `@adr:...`) from
//! comment lines.

use crate::index::{Symbol, SymbolKind, Visibility};
use crate::parser::{ParseResult, SourceParser};
use std::path::Path;

/// Markup flavor handled by [`MarkupParser`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupFlavor {
    Markdown,
    Rst,
    Asciidoc,
}

/// Lightweight, dependency-free markup parser.
pub struct MarkupParser {
    flavor: MarkupFlavor,
    extensions: &'static [&'static str],
    language_id: &'static str,
}

/// State for HTML comments (`<!-- -->`) that span multiple lines.
struct HtmlCommentState {
    open: bool,
    acc: String,
}

impl MarkupParser {
    /// Markdown/MDX parser (`md`, `mdx`).
    pub fn markdown() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Markdown,
            extensions: &["md", "mdx"],
            language_id: "markdown",
        })
    }

    /// reStructuredText parser (`rst`).
    pub fn rst() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Rst,
            extensions: &["rst"],
            language_id: "rst",
        })
    }

    /// AsciiDoc parser (`adoc`).
    pub fn asciidoc() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Asciidoc,
            extensions: &["adoc"],
            language_id: "asciidoc",
        })
    }
}

impl SourceParser for MarkupParser {
    fn parse_file(&mut self, _path: &Path, content: &str) -> ParseResult {
        let mut symbols: Vec<Symbol> = Vec::new();
        let mut html_state = HtmlCommentState {
            open: false,
            acc: String::new(),
        };
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();

            // Headings.
            match self.flavor {
                MarkupFlavor::Markdown | MarkupFlavor::Asciidoc => {
                    if let Some(text) = atx_heading(trimmed, self.flavor) {
                        symbols.push(mk_symbol("heading", text, line_num, trimmed));
                    }
                }
                MarkupFlavor::Rst => {
                    if i > 0 && is_underline(trimmed) {
                        let title = lines[i - 1].trim();
                        if !title.is_empty() {
                            // Previous line (1-based line number == i).
                            symbols.push(mk_symbol("heading", title.to_string(), i, title));
                        }
                    }
                }
            }

            // Anchors on the current line.
            for name in extract_anchors(self.flavor, trimmed) {
                symbols.push(mk_symbol("anchor", name, line_num, trimmed));
            }

            // Trace tags in comment lines attach to the closest symbol.
            for comment in extract_comments(self.flavor, trimmed, &mut html_state) {
                let tags = scan_tags(&comment);
                if tags.is_empty() {
                    continue;
                }
                if let Some(idx) = closest_symbol(&symbols, line_num) {
                    symbols[idx].tags.extend(tags);
                } else {
                    for tag in tags {
                        symbols.push(mk_symbol("trace-comment", tag.clone(), line_num, &comment));
                    }
                }
            }
        }

        ParseResult {
            symbols,
            errors: Vec::new(),
        }
    }

    fn extensions(&self) -> &[&str] {
        self.extensions
    }

    fn language_id(&self) -> &'static str {
        self.language_id
    }
}

/// Build a symbol with the standard markup defaults.
fn mk_symbol(kind: &str, name: String, line: usize, signature: &str) -> Symbol {
    Symbol {
        kind: SymbolKind::Other(kind.to_string()),
        qualified_name: name.clone(),
        name,
        signature: signature.trim().to_string(),
        file: String::new(),
        line,
        end_line: line,
        doc: None,
        tags: Vec::new(),
        visibility: Visibility::Pub,
        attrs: Vec::new(),
    }
}

/// Atx heading: `^#{1,6}\s+(.+)$` for Markdown, `^={1,6}\s+(.+)$` for AsciiDoc.
/// Trailing marker runs (e.g. `Title ##`) are trimmed from the name.
fn atx_heading(trimmed: &str, flavor: MarkupFlavor) -> Option<String> {
    let marker = if flavor == MarkupFlavor::Asciidoc {
        b'='
    } else {
        b'#'
    };
    let bytes = trimmed.as_bytes();
    let mut n = 0;
    while n < bytes.len() && n < 6 && bytes[n] == marker {
        n += 1;
    }
    if n == 0 || n == bytes.len() || (bytes[n] != b' ' && bytes[n] != b'\t') {
        return None;
    }
    let name = trimmed[n..]
        .trim_start()
        .trim_end_matches(marker as char)
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

/// RST title underline: a line of 3+ identical chars from `= - ~ ^`.
fn is_underline(trimmed: &str) -> bool {
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '=' | '-' | '~' | '^') {
        return false;
    }
    let mut len = 1;
    for c in chars {
        if c != first {
            return false;
        }
        len += 1;
    }
    len >= 3
}

/// Extract anchor names from a line, flavor-dependent.
fn extract_anchors(flavor: MarkupFlavor, trimmed: &str) -> Vec<String> {
    let mut out = Vec::new();
    match flavor {
        MarkupFlavor::Markdown => {
            // {#id}
            let mut rest = trimmed;
            while let Some(start) = rest.find("{#") {
                let after = &rest[start + 2..];
                if let Some(end) = after.find('}') {
                    let id = after[..end].trim();
                    if !id.is_empty() {
                        out.push(id.to_string());
                    }
                    rest = &after[end + 1..];
                } else {
                    break;
                }
            }
            // <a id="...">
            let mut rest = trimmed;
            while let Some(pos) = rest.find("id=\"") {
                let after = &rest[pos + 4..];
                if let Some(end) = after.find('"') {
                    let id = after[..end].trim();
                    if !id.is_empty() {
                        out.push(id.to_string());
                    }
                    rest = &after[end + 1..];
                } else {
                    break;
                }
            }
        }
        MarkupFlavor::Rst => {
            // .. _name:
            let mut rest = trimmed;
            while let Some(pos) = rest.find(".. _") {
                let after = &rest[pos + 4..];
                let len = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
                    .count();
                if len > 0 && after.as_bytes().get(len) == Some(&b':') {
                    out.push(after[..len].to_string());
                }
                rest = if len == 0 {
                    &rest[pos + 4..]
                } else {
                    &after[len..]
                };
            }
        }
        MarkupFlavor::Asciidoc => {
            // [[id]]
            let mut rest = trimmed;
            while let Some(pos) = rest.find("[[") {
                let after = &rest[pos + 2..];
                if let Some(end) = after.find("]]") {
                    let id = after[..end].trim();
                    if !id.is_empty() {
                        out.push(id.to_string());
                    }
                    rest = &after[end + 2..];
                } else {
                    break;
                }
            }
            // [#id]
            let mut rest = trimmed;
            while let Some(pos) = rest.find("[#") {
                let after = &rest[pos + 2..];
                if let Some(end) = after.find(']') {
                    let id = after[..end].trim();
                    if !id.is_empty() {
                        out.push(id.to_string());
                    }
                    rest = &after[end + 1..];
                } else {
                    break;
                }
            }
        }
    }
    out
}

/// Extract comment text(s) from a line, flavor-dependent.
fn extract_comments(
    flavor: MarkupFlavor,
    line: &str,
    state: &mut HtmlCommentState,
) -> Vec<String> {
    match flavor {
        MarkupFlavor::Markdown => extract_html_comments(line, state),
        MarkupFlavor::Rst => {
            let mut out = Vec::new();
            if let Some(rest) = line.trim_start().strip_prefix(".. ") {
                out.push(rest.to_string());
            }
            out
        }
        MarkupFlavor::Asciidoc => {
            let mut out = Vec::new();
            if let Some(rest) = line.trim_start().strip_prefix("//") {
                out.push(rest.to_string());
            }
            out
        }
    }
}

/// Extract HTML comment contents from `line`, tracking multi-line state.
fn extract_html_comments(line: &str, state: &mut HtmlCommentState) -> Vec<String> {
    let mut out = Vec::new();
    let mut s = line;
    loop {
        if state.open {
            match s.find("-->") {
                Some(end) => {
                    state.acc.push_str(&s[..end]);
                    out.push(std::mem::take(&mut state.acc));
                    s = &s[end + 3..];
                    state.open = false;
                }
                None => {
                    state.acc.push_str(s);
                    state.acc.push('\n');
                    break;
                }
            }
        } else if let Some(start) = s.find("<!--") {
            let after = &s[start + 4..];
            match after.find("-->") {
                Some(end) => {
                    out.push(after[..end].to_string());
                    s = &after[end + 3..];
                }
                None => {
                    state.open = true;
                    state.acc.push_str(after);
                    state.acc.push('\n');
                    break;
                }
            }
        } else {
            break;
        }
    }
    out
}

/// Trace tags: `@(req|design|impl|test|adr):[A-Za-z0-9_.:/-]+`
fn scan_tags(text: &str) -> Vec<String> {
    let keywords = ["req", "design", "impl", "test", "adr"];
    let bytes = text.as_bytes();
    let mut tags = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        let mut matched = false;
        for kw in keywords {
            let kb = kw.as_bytes();
            let end = i + 1 + kb.len();
            if bytes.get(end) == Some(&b':') && &bytes[i + 1..end] == kb {
                let mut j = end + 1;
                let start = j;
                while j < bytes.len() && is_tag_char(bytes[j]) {
                    j += 1;
                }
                if j > start {
                    tags.push(text[i..j].to_string());
                }
                i = j;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        i += 1;
    }
    tags
}

fn is_tag_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'/' | b'-')
}

/// Index of the symbol whose line is closest to `line` (earliest line wins ties).
/// Returns `None` when there are no symbols at all.
fn closest_symbol(symbols: &[Symbol], line: usize) -> Option<usize> {
    symbols
        .iter()
        .enumerate()
        .min_by_key(|(_, s)| (s.line.abs_diff(line), s.line))
        .map(|(idx, _)| idx)
}
