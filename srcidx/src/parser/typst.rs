//! Typst parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::SourceParser;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use tree_sitter::Query;
use anyhow::Result;

pub fn create_typst_parser() -> Result<Box<dyn SourceParser>> {
    let typst_language = codebook_tree_sitter_typst::LANGUAGE.into();

    let typst_query = Query::new(
        &typst_language,
        include_str!("queries/typst/tags.scm"),
    )?;

    let pattern_kind_map = vec![
        SymbolKind::Fn,         // 0: let (call) -> Fn
        SymbolKind::Type,       // 1: let (ident) -> Type
        SymbolKind::Variable,   // 2: let (ident) -> Variable
        SymbolKind::Import,     // 3: import
        SymbolKind::Other("doc".into()),   // 4: doc comment
    ];

    let config = TreeSitterConfig {
        language_id: "typst",
        extensions: &["typ"],
        language: typst_language,
        query: typst_query,
        kind_capture: "",
        name_capture: "name",
        signature_capture: Some("signature"),
        return_type_capture: Some("return_type"),
        doc_capture: Some("doc"),
        visibility_capture: None,
        attrs_capture: None,
        pattern_kind_map: Some(pattern_kind_map),
        capture_names: Vec::new(), // Will be populated in new()
    };

    let parser = TreeSitterParser::new(config)?;
    Ok(Box::new(parser))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SymbolKind;
    use std::path::Path;

    #[test]
    fn test_parse_typst_function() {
        let code = r#"
#let add(x, y) = x + y
"#;
        let mut parser = create_typst_parser().unwrap();
        let result = parser.parse_file(Path::new("test.typ"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
    }

    #[test]
    fn test_parse_typst_import() {
        let code = r#"
#import "package.typ": helper
"#;
        let mut parser = create_typst_parser().unwrap();
        let result = parser.parse_file(Path::new("test.typ"), code);
        let imports: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Import).collect();
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_parse_typst_type_alias() {
        let code = r#"
#let Point = (x: int, y: int)
"#;
        let mut parser = create_typst_parser().unwrap();
        let result = parser.parse_file(Path::new("test.typ"), code);
        let types: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Type).collect();
        assert_eq!(types.len(), 1);
    }

    #[test]
    fn test_parse_typst_let_binding() {
        let code = r#"
#let x = 42
#let PI = 3.14159
"#;
        let mut parser = create_typst_parser().unwrap();
        let result = parser.parse_file(Path::new("test.typ"), code);
        let bindings: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Variable).collect();
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_parse_typst_const() {
        let code = r#"
#let PI = 3.14159
"#;
        let mut parser = create_typst_parser().unwrap();
        let result = parser.parse_file(Path::new("test.typ"), code);
        let consts: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(consts.len(), 0); // Typst grammar does not distinguish constants from other let bindings
    }
}