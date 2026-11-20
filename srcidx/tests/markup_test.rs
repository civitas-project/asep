//! Tests for the lightweight markup parsers (Markdown/MDX, reST, AsciiDoc).

use srcidx::index::SymbolKind;
use srcidx::parser::markup::MarkupParser;
use srcidx::parser::ParserRegistry;

fn write_doc(content: &str, ext: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join(format!("doc.{ext}"));
    std::fs::write(&path, content).expect("write temp file");
    (dir, path)
}

fn symbol_named<'a>(
    symbols: &'a [srcidx::index::Symbol],
    kind: &str,
    name: &str,
) -> &'a srcidx::index::Symbol {
    symbols
        .iter()
        .find(|s| s.name == name && matches!(s.kind, SymbolKind::Other(ref k) if k == kind))
        .unwrap_or_else(|| panic!("no {kind} symbol named '{name}'"))
}

#[test]
fn test_markup_parser_constructors() {
    assert_eq!(MarkupParser::markdown().language_id(), "markdown");
    assert_eq!(MarkupParser::markdown().extensions(), &["md", "mdx"]);
    assert_eq!(MarkupParser::rst().language_id(), "rst");
    assert_eq!(MarkupParser::rst().extensions(), &["rst"]);
    assert_eq!(MarkupParser::asciidoc().language_id(), "asciidoc");
    assert_eq!(MarkupParser::asciidoc().extensions(), &["adoc"]);
}

#[test]
fn test_markdown_symbols_and_tags() {
    let content = "# Title\n## Section {#sec}\n<!-- @req:FR-A:MUST -->\n";
    let (_dir, path) = write_doc(content, "md");
    let registry = ParserRegistry::default();
    let result = registry.parse_file(&path, content);
    let symbols = &result.symbols;

    let heading = symbol_named(symbols, "heading", "Title");
    assert_eq!(heading.line, 1);
    assert_eq!(heading.end_line, 1);
    assert!(heading.tags.is_empty());

    let anchor = symbol_named(symbols, "anchor", "sec");
    assert_eq!(anchor.line, 2);

    // The trace tag on the comment line attaches to the closest symbol.
    assert!(
        symbols
            .iter()
            .any(|s| s.tags.iter().any(|tag| tag.contains("@req:FR-A"))),
        "trace tag @req:FR-A should be attached to some symbol"
    );
}

#[test]
fn test_rst_symbols() {
    let content = "Title\n=====\n\n.. _target:\n";
    let (_dir, path) = write_doc(content, "rst");
    let registry = ParserRegistry::default();
    let result = registry.parse_file(&path, content);
    let symbols = &result.symbols;

    let heading = symbol_named(symbols, "heading", "Title");
    assert_eq!(heading.line, 1);

    let anchor = symbol_named(symbols, "anchor", "target");
    assert_eq!(anchor.line, 4);
}

#[test]
fn test_asciidoc_symbols() {
    let content = "== Section\n\n[[id]]\n";
    let (_dir, path) = write_doc(content, "adoc");
    let registry = ParserRegistry::default();
    let result = registry.parse_file(&path, content);
    let symbols = &result.symbols;

    let heading = symbol_named(symbols, "heading", "Section");
    assert_eq!(heading.line, 1);

    let anchor = symbol_named(symbols, "anchor", "id");
    assert_eq!(anchor.line, 3);
}
