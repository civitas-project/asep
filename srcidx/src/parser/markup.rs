//! Lightweight, dependency-free markup parsers.
//!
//! Parses Markdown/MDX, reStructuredText, AsciiDoc, HTML, XML, and LaTeX
//! with no external parsing dependencies. Extracts headings, anchors, and
//! traceability tags (`@req:<id>`, `@design:<id>`, `@impl:<id>`, `@test:<id>`,
//! `@adr:<id>`) from comment lines.

use crate::index::{Symbol, SymbolKind, Visibility};
use crate::parser::{ParseResult, SourceParser};
use std::path::Path;

/// Markup flavor handled by [`MarkupParser`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupFlavor {
    Markdown,
    Rst,
    Asciidoc,
    Html,
    Xml,
    Latex,
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
    /// Create a Markdown/MDX parser (`md`, `mdx`).
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

    /// Build an AsciiDoc parser (`adoc`).
    pub fn asciidoc() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Asciidoc,
            extensions: &["adoc"],
            language_id: "asciidoc",
        })
    }

    /// An HTML parser (`html`, `htm`).
    pub fn html() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Html,
            extensions: &["html", "htm"],
            language_id: "html",
        })
    }

    /// An XML parser (`xml`).
    pub fn xml() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Xml,
            extensions: &["xml"],
            language_id: "xml",
        })
    }

    /// A LaTeX parser (`tex`, `latex`).
    pub fn latex() -> Box<dyn SourceParser> {
        Box::new(MarkupParser {
            flavor: MarkupFlavor::Latex,
            extensions: &["tex", "latex"],
            language_id: "latex",
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
        let mut skip_state = ScriptStyleState {
            in_script: false,
            in_style: false,
        };
        let mut title_state = HtmlTitleState {
            open: false,
            acc: String::new(),
        };
        let mut xml_state = XmlState::default();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();
            let visible = match self.flavor {
                MarkupFlavor::Html => strip_script_style(trimmed, &mut skip_state),
                _ => trimmed.to_string(),
            };

            // Headings / structural elements.
            match self.flavor {
                MarkupFlavor::Markdown | MarkupFlavor::Asciidoc => {
                    if let Some(text) = atx_heading(&visible, self.flavor) {
                        symbols.push(mk_symbol("heading", text, line_num, trimmed));
                    }
                }
                MarkupFlavor::Rst => {
                    if let Some(title) = rst_title(i, &visible, &lines) {
                        symbols.push(mk_symbol("heading", title.clone(), i, &title));
                    }
                }
                MarkupFlavor::Html => {
                    if let Some(text) = html_title(&visible, &mut title_state) {
                        symbols.push(mk_symbol("document-title", text, line_num, trimmed));
                    }
                    if let Some(text) = html_heading(&visible) {
                        symbols.push(mk_symbol("heading", text, line_num, trimmed));
                    }
                    for name in html_anchors(&visible) {
                        symbols.push(mk_symbol("anchor", name, line_num, trimmed));
                    }
                }
                MarkupFlavor::Xml => {
                    for event in xml_events(&visible, &mut xml_state) {
                        match event {
                            XmlEvent::Root(name) => {
                                symbols.push(mk_symbol("root", name, line_num, trimmed));
                            }
                            XmlEvent::Element(name) => {
                                symbols.push(mk_symbol("element", name, line_num, trimmed));
                            }
                            XmlEvent::Schema(name) => {
                                symbols.push(mk_symbol("schema", name, line_num, trimmed));
                            }
                        }
                    }
                }
                MarkupFlavor::Latex => {
                    for cmd in latex_commands(&visible) {
                        match cmd {
                            LatexCommand::Heading(name) => {
                                symbols.push(mk_symbol("heading", name, line_num, trimmed));
                            }
                            LatexCommand::Anchor(name) => {
                                symbols.push(mk_symbol("anchor", name, line_num, trimmed));
                            }
                        }
                    }
                }
            }

            // Anchors on the current line (Markdown / RST / AsciiDoc).
            if matches!(
                self.flavor,
                MarkupFlavor::Markdown | MarkupFlavor::Rst | MarkupFlavor::Asciidoc
            ) {
                for name in extract_anchors(self.flavor, &visible) {
                    symbols.push(mk_symbol("anchor", name, line_num, trimmed));
                }
            }

            // Trace tags in comment lines attach to the closest symbol.
            for comment in extract_comments(self.flavor, line, &mut html_state) {
                let tags = scan_tags(&comment);
                if tags.is_empty() {
                    continue;
                }
                match closest_symbol(&symbols, line_num) {
                    Some(idx) => symbols[idx].tags.extend(tags),
                    None => {
                        for tag in tags {
                            symbols.push(mk_symbol("trace-comment", tag.clone(), line_num, &comment));
                        }
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
    let marker = match flavor == MarkupFlavor::Asciidoc {
        true => b'=',
        false => b'#',
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
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
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

/// RST section title: the line under a 3+ char underline, when non-empty.
/// Returns `None` when there is no previous line, the underline is invalid,
/// or the title text is empty.
fn rst_title(i: usize, visible: &str, lines: &[&str]) -> Option<String> {
    if i == 0 || !is_underline(visible) {
        return None;
    }
    let title = lines[i - 1].trim();
    if title.is_empty() {
        return None;
    }
    Some(title.to_string())
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
                match after.find('}') {
                    Some(end) => {
                        let id = after[..end].trim();
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                        rest = &after[end + 1..];
                    }
                    None => break,
                }
            }
            // <a id="...">
            let mut rest = trimmed;
            while let Some(pos) = rest.find("id=\"") {
                let after = &rest[pos + 4..];
                match after.find('"') {
                    Some(end) => {
                        let id = after[..end].trim();
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                        rest = &after[end + 1..];
                    }
                    None => break,
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
                rest = match len {
                    0 => &rest[pos + 4..],
                    _ => &after[len..],
                };
            }
        }
        MarkupFlavor::Asciidoc => {
            // [[id]]
            let mut rest = trimmed;
            while let Some(pos) = rest.find("[[") {
                let after = &rest[pos + 2..];
                match after.find("]]") {
                    Some(end) => {
                        let id = after[..end].trim();
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                        rest = &after[end + 2..];
                    }
                    None => break,
                }
            }
            // [#id]
            let mut rest = trimmed;
            while let Some(pos) = rest.find("[#") {
                let after = &rest[pos + 2..];
                match after.find(']') {
                    Some(end) => {
                        let id = after[..end].trim();
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                        rest = &after[end + 1..];
                    }
                    None => break,
                }
            }
        }
        MarkupFlavor::Html | MarkupFlavor::Xml | MarkupFlavor::Latex => {}
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
        MarkupFlavor::Markdown | MarkupFlavor::Html | MarkupFlavor::Xml => {
            extract_html_comments(line, state)
        }
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
        MarkupFlavor::Latex => {
            let mut out = Vec::new();
            if let Some(rest) = line.trim_start().strip_prefix('%') {
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
        match state.open {
            true => {
                match s.find("-->") {
                    Some(end) => {
                        state.acc.push_str(&s[..end]);
                        out.push(std::mem::take(&mut state.acc));
                        s = &s[end + 3..];
                        state.open = false;
                        continue;
                    }
                    None => {
                        state.acc.push_str(s);
                        state.acc.push('\n');
                        break;
                    }
                }
            }
            false => {
                match s.find("<!--") {
                    Some(start) => {
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
                    }
                    None => break,
                }
            }
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
            let keyword_ok = bytes.get(end) == Some(&b':') && &bytes[i + 1..end] == kb;
            let mut j = end + 1;
            let start = j;
            while keyword_ok && j < bytes.len() && is_tag_char(bytes[j]) {
                j += 1;
            }
            let should_push = keyword_ok && j > start;
            if should_push {
                tags.push(text[i..j].to_string());
            }
            if keyword_ok {
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

/// State for skipping `<script>`/`<style>` blocks (simple line-state heuristic).
struct ScriptStyleState {
    in_script: bool,
    in_style: bool,
}

/// State for a `<title>...</title>` element that may span multiple lines.
struct HtmlTitleState {
    open: bool,
    acc: String,
}

/// State for XML parsing: whether the root element has already been seen.
#[derive(Default)]
struct XmlState {
    root_seen: bool,
}

/// Event produced while scanning an XML line.
enum XmlEvent {
    Root(String),
    Element(String),
    Schema(String),
}

/// Command produced while scanning a LaTeX line.
enum LatexCommand {
    Heading(String),
    Anchor(String),
}

/// Removes the contents of `<script>`/`<style>` blocks using a simple
/// line-state heuristic. Returns the visible portions of `line`.
fn strip_script_style(line: &str, state: &mut ScriptStyleState) -> String {
    let mut visible = String::new();
    let mut rest = line;
    loop {
        if state.in_script || state.in_style {
            let close_tag = match state.in_script {
                true => "</script>",
                false => "</style>",
            };
            match rest.find(close_tag) {
                Some(end) => {
                    rest = &rest[end + close_tag.len()..];
                    state.in_script = false;
                    state.in_style = false;
                    continue;
                }
                None => break,
            }
        }
        let open_script = rest.find("<script");
        let open_style = rest.find("<style");
        match (open_script, open_style) {
            (Some(s), Some(t)) => {
                match s <= t {
                    true => {
                        visible.push_str(&rest[..s]);
                        state.in_script = true;
                        rest = &rest[s + "<script".len()..];
                    }
                    false => {
                        visible.push_str(&rest[..t]);
                        state.in_style = true;
                        rest = &rest[t + "<style".len()..];
                    }
                }
            }
            (Some(s), None) => {
                visible.push_str(&rest[..s]);
                state.in_script = true;
                rest = &rest[s + "<script".len()..];
            }
            (None, Some(t)) => {
                visible.push_str(&rest[..t]);
                state.in_style = true;
                rest = &rest[t + "<style".len()..];
            }
            (None, None) => {
                visible.push_str(rest);
                break;
            }
        }
    }
    visible
}

/// Extracts `<title>...</title>` text, handling multi-line state.
fn html_title(line: &str, state: &mut HtmlTitleState) -> Option<String> {
    let mut s = line;
    let mut found = None;
    loop {
        match state.open {
            true => {
                match s.find("</title>") {
                    Some(end) => {
                        state.acc.push_str(&s[..end]);
                        let text = state.acc.trim();
                        if !text.is_empty() {
                            found = Some(text.to_string());
                        }
                        state.open = false;
                        state.acc.clear();
                        s = &s[end + 8..];
                        continue;
                    }
                    None => {
                        state.acc.push_str(s);
                        state.acc.push('\n');
                        break;
                    }
                }
            }
            false => {
                match s.find("<title>") {
                    Some(start) => {
                        let after = &s[start + 7..];
                        match after.find("</title>") {
                            Some(end) => {
                                let text = after[..end].trim();
                                if !text.is_empty() {
                                    found = Some(text.to_string());
                                }
                                s = &after[end + 8..];
                            }
                            None => {
                                state.open = true;
                                state.acc.push_str(after);
                                state.acc.push('\n');
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        }
    }
    found
}

/// Extracts heading text from `<h1>`–`<h6>` (single line, attributes allowed).
fn html_heading(line: &str) -> Option<String> {
    for n in 1..=6u8 {
        let open = format!("<h{n}");
        match line.find(&open) {
            Some(pos) => {
                let after_open = &line[pos + open.len()..];
                let first = after_open.as_bytes().first().copied();
                if !matches!(
                    first,
                    Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t')
                ) {
                    continue;
                }
                let close_open = match after_open.find('>') {
                    Some(g) => g,
                    None => continue,
                };
                let content = &after_open[close_open + 1..];
                let close = format!("</h{n}>");
                match content.find(&close) {
                    Some(end) => {
                        let text = content[..end].trim();
                        if text.is_empty() {
                            continue;
                        }
                        return Some(text.to_string());
                    }
                    None => continue,
                }
            }
            None => continue,
        }
    }
    None
}

/// Extracts `id="..."` / `name="..."` attribute values from an HTML line.
fn html_anchors(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for attr in ["id", "name"] {
        let pat = format!("{attr}=\"");
        let mut rest = line;
        while let Some(pos) = rest.find(&pat) {
            let after = &rest[pos + pat.len()..];
            match after.find('"') {
                Some(end) => {
                    let value = after[..end].trim();
                    if !value.is_empty() {
                        out.push(value.to_string());
                    }
                    rest = &after[end + 1..];
                }
                None => break,
            }
        }
    }
    out
}

/// Scans an XML line for root, element, and schema events.
fn xml_events(line: &str, state: &mut XmlState) -> Vec<XmlEvent> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        if after.is_empty() {
            break;
        }
        match after.as_bytes().first().copied() {
            Some(b'/') | Some(b'?') | Some(b'!') => {
                // Closing tag, processing instruction, comment, or doctype.
                let end = rest.find('>').map(|g| g + 1).unwrap_or(rest.len());
                rest = &rest[end..];
                continue;
            }
            _ => {}
        }
        let name_len = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '-' | '.'))
            .count();
        if name_len == 0 {
            rest = &after[1..];
            continue;
        }
        let name = &after[..name_len];
        let gt = match after.find('>') {
            Some(g) => g,
            None => break,
        };
        let tag_part = &after[..gt];
        rest = &after[gt + 1..];

        match (is_xsd_element(name), state.root_seen) {
            (true, _) => {
                if let Some(value) = xml_attribute(tag_part, "name") {
                    out.push(XmlEvent::Schema(value));
                }
                continue;
            }
            (false, false) => {
                state.root_seen = true;
                out.push(XmlEvent::Root(name.to_string()));
                continue;
            }
            (false, true) => {
                if let Some(value) = xml_id_name_key(tag_part) {
                    out.push(XmlEvent::Element(value));
                }
                continue;
            }
        }
    }
    out
}

/// True for XSD `xs:element`/`xs:complexType`/`xs:simpleType`/`xs:attribute`.
fn is_xsd_element(name: &str) -> bool {
    name.starts_with("xs:")
        && matches!(
            name.split(':').next_back().unwrap_or(name),
            "element" | "complexType" | "simpleType" | "attribute"
        )
}

/// Extracts the value of `attr="..."` from an XML tag portion.
fn xml_attribute(tag: &str, attr: &str) -> Option<String> {
    let pat = format!("{attr}=\"");
    let pos = tag.find(&pat)?;
    let after = &tag[pos + pat.len()..];
    let end = after.find('"')?;
    let value = after[..end].trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

/// First of `id`, `name`, `key` attribute values on an XML element tag.
fn xml_id_name_key(tag: &str) -> Option<String> {
    for attr in ["id", "name", "key"] {
        if let Some(value) = xml_attribute(tag, attr) {
            return Some(value);
        }
    }
    None
}

/// Scans a LaTeX line for `\chapter`/`\section`/`\subsection` headings and
/// `\label` anchors.
fn latex_commands(line: &str) -> Vec<LatexCommand> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find('\\') {
        let after = &rest[pos + 1..];
        let name_len = after.chars().take_while(|c| c.is_alphabetic()).count();
        if name_len == 0 {
            rest = &after[1..];
            continue;
        }
        let cmd = &after[..name_len];
        let rest_after_cmd = &after[name_len..];
        match cmd {
            "chapter" | "section" | "subsection" => {
                if let Some(content) = take_brace_content(rest_after_cmd) {
                    out.push(LatexCommand::Heading(content));
                }
            }
            "label" => {
                if let Some(content) = take_brace_content(rest_after_cmd) {
                    out.push(LatexCommand::Anchor(content));
                }
            }
            _ => {}
        }
        rest = rest_after_cmd;
    }
    out
}

/// Extracts the balanced brace content of `{...}` at the start of `rest`.
fn take_brace_content(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    let bytes = rest.as_bytes();
    if bytes.first() != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                match depth == 0 {
                    true => return Some(rest[1..i].to_string()),
                    false => {}
                }
            }
            _ => {}
        }
    }
    None
}
