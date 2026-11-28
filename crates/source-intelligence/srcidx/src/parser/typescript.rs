//! TypeScript/JavaScript parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use crate::parser::SourceParser;
use anyhow::Result;
use tree_sitter::Query;

/// Create a TypeScript/JavaScript parser.
pub fn create_typescript_parser() -> Result<Box<dyn SourceParser>> {
    // TypeScript language
    let ts_language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();

    // Query for TypeScript symbols
    let ts_query = Query::new(&ts_language, include_str!("queries/typescript/tags.scm"))?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/typescript/tags.scm:
    // 0: function_declaration -> Fn
    // 1: generator_function_declaration -> Fn
    // 2: method_definition -> Fn
    // 3: arrow_function -> Fn
    // 4: class_declaration -> Class
    // 5: class -> Class
    // 6: interface_declaration -> Interface
    // 7: type_alias_declaration -> Type
    // 8: enum_declaration -> Enum
    // 9: variable_declaration -> Variable
    // 10: import_statement -> Import
    // 11: export_statement (re-exports) -> Import
    // 12: export_statement (@kind) -> Export
    // 13: comment -> Other (doc only)
    // 14: accessibility_modifier -> Other (visibility only)
    // 15: override_modifier -> Other (visibility only)
    // 15: decorator -> Other (attrs only)
    let pattern_kind_map = vec![
        SymbolKind::Fn,                              // 0: function_declaration
        SymbolKind::Fn,                              // 1: generator_function_declaration
        SymbolKind::Fn,                              // 2: method_definition
        SymbolKind::Fn,                              // 3: arrow_function
        SymbolKind::Class,                           // 4: class_declaration
        SymbolKind::Class,                           // 5: class
        SymbolKind::Interface,                       // 6: interface_declaration
        SymbolKind::Type,                            // 7: type_alias_declaration
        SymbolKind::Enum,                            // 8: enum_declaration
        SymbolKind::Variable,                        // 9: variable_declaration
        SymbolKind::Import,                          // 10: import_statement
        SymbolKind::Import,                          // 11: export_statement (re-exports)
        SymbolKind::Export,                          // 12: export_statement (@kind)
        SymbolKind::Other("typescript".to_string()), // 13: comment (doc only)
        SymbolKind::Other("typescript".to_string()), // 14: accessibility_modifier
        SymbolKind::Other("typescript".to_string()), // 15: override_modifier
        SymbolKind::Other("typescript".to_string()), // 16: decorator
    ];

    let config = TreeSitterConfig {
        language_id: "typescript",
        extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        language: ts_language,
        query: ts_query,
        kind_capture: "",
        name_capture: "name",
        signature_capture: Some("signature"),
        return_type_capture: Some("return_type"),
        doc_capture: Some("doc"),
        visibility_capture: Some("visibility"),
        attrs_capture: Some("attrs"),
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
    fn test_parse_typescript_function() {
        let code = r#"
/** Adds two numbers. */
export function add(a: number, b: number): number {
    return a + b;
}
"#;
        let mut parser = create_typescript_parser().unwrap();
        let result = parser.parse_file(Path::new("test.ts"), code);
        eprintln!("Symbols found: {}", result.symbols.len());
        for sym in &result.symbols {
            eprintln!(
                "  - {} ({:?}) at {}:{}-{} kind_node={:?}",
                sym.name, sym.kind, sym.file, sym.line, sym.end_line, sym.kind
            );
        }
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
        assert!(sym.signature.contains("a: number"));
        assert!(sym.signature.contains("b: number"));
        assert!(sym.signature.contains("-> number"));
    }

    #[test]
    fn test_parse_typescript_class() {
        let code = r#"
export class Point {
    x: number;
    y: number;
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
}
"#;
        let mut parser = create_typescript_parser().unwrap();
        let result = parser.parse_file(Path::new("test.ts"), code);
        let class_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Class)
            .unwrap();
        assert_eq!(class_sym.name, "Point");
        let method_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Fn && s.name == "constructor")
            .unwrap();
        assert_eq!(method_sym.name, "constructor");
    }

    #[test]
    fn test_parse_typescript_interface() {
        let code = r#"
export interface Shape {
    area(): number;
}
"#;
        let mut parser = create_typescript_parser().unwrap();
        let result = parser.parse_file(Path::new("test.ts"), code);
        let interfaces: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .collect();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Shape");
    }

    #[test]
    fn test_parse_typescript_type_alias() {
        let code = r#"
export type Point = { x: number; y: number };
"#;
        let mut parser = create_typescript_parser().unwrap();
        let result = parser.parse_file(Path::new("test.ts"), code);
        let types: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Type)
            .collect();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "Point");
    }

    #[test]
    fn test_parse_javascript_function() {
        let code = r#"
function add(a, b) {
    return a + b;
}
export default add;
"#;
        let mut parser = create_typescript_parser().unwrap();
        let result = parser.parse_file(Path::new("test.js"), code);
        let fn_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Fn)
            .unwrap();
        assert_eq!(fn_sym.name, "add");
    }
}
