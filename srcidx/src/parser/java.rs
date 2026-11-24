//! Java parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use crate::parser::SourceParser;
use anyhow::Result;
use tree_sitter::Query;

/// Create a Java parser.
pub fn create_java_parser() -> Result<Box<dyn SourceParser>> {
    let java_language = tree_sitter_java::LANGUAGE.into();

    let java_query = Query::new(&java_language, include_str!("queries/java/tags.scm"))?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/java/tags.scm:
    // 0: method_declaration -> Fn
    // 1: constructor_declaration -> Fn
    // 2: class_declaration -> Class
    // 3: interface_declaration -> Interface
    // 4: enum_declaration -> Enum
    // 5: record_declaration -> Struct
    // 6: field_declaration (static final) -> Const
    // 7: field_declaration -> Variable
    // 8: import_declaration (scoped_identifier) -> Import
    // 9: import_declaration (identifier) -> Import
    // 10: package_declaration (scoped_identifier) -> Mod
    // 11: package_declaration (identifier) -> Mod
    // 12: doc comment (line_comment)
    // 13: doc comment (block_comment)
    // 13: attrs (annotation)
    let pattern_kind_map = vec![
        SymbolKind::Fn,                    // 0: method_declaration
        SymbolKind::Fn,                    // 1: constructor_declaration
        SymbolKind::Class,                 // 2: class_declaration
        SymbolKind::Interface,             // 3: interface_declaration
        SymbolKind::Enum,                  // 4: enum_declaration
        SymbolKind::Struct,                // 5: record_declaration
        SymbolKind::Const,                 // 6: field_declaration (static final)
        SymbolKind::Variable,              // 7: field_declaration
        SymbolKind::Import,                // 8: import_declaration (scoped_identifier)
        SymbolKind::Import,                // 9: import_declaration (identifier)
        SymbolKind::Mod,                   // 10: package_declaration (scoped_identifier)
        SymbolKind::Mod,                   // 11: package_declaration (identifier)
        SymbolKind::Other("doc".into()),   // 12: line_comment
        SymbolKind::Other("doc".into()),   // 13: block_comment
        SymbolKind::Other("attrs".into()), // 14: attrs
    ];

    let config = TreeSitterConfig {
        language_id: "java",
        extensions: &["java"],
        language: java_language,
        query: java_query,
        kind_capture: "",
        name_capture: "name",
        signature_capture: Some("signature"),
        return_type_capture: Some("return_type"),
        doc_capture: Some("doc"),
        visibility_capture: None,
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
    fn test_parse_java_method() {
        let code = r#"
public class Test {
    public int add(int x, int y) {
        return x + y;
    }
}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Test.java"), code);
        let method_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Fn && s.name == "add")
            .unwrap();
        assert_eq!(method_sym.name, "add");
        assert_eq!(method_sym.kind, SymbolKind::Fn);
    }

    #[test]
    fn test_parse_java_class() {
        let code = r#"
public class Point {
    private int x;
    private int y;
    public Point(int x, int y) {
        this.x = x;
        this.y = y;
    }
}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Point.java"), code);
        let class_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Class)
            .unwrap();
        assert_eq!(class_sym.name, "Point");
        let constructor_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Fn && s.name == "Point")
            .unwrap();
        assert_eq!(constructor_sym.name, "Point");
    }

    #[test]
    fn test_parse_java_interface() {
        let code = r#"
public interface Shape {
    double area();
}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Shape.java"), code);
        let interfaces: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .collect();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Shape");
    }

    #[test]
    fn test_parse_java_enum() {
        let code = r#"
public enum Color {
    RED, GREEN, BLUE
}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Color.java"), code);
        let enums: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Color");
    }

    #[test]
    fn test_parse_java_record() {
        let code = r#"
public record Point(int x, int y) {}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Point.java"), code);
        let records: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "Point");
    }

    #[test]
    fn test_parse_java_import() {
        let code = r#"
package com.example;

import java.util.List;
import java.util.Map;
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Test.java"), code);
        let imports: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Import)
            .collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_java_const() {
        let code = r#"
public class Constants {
    public static final int MAX_SIZE = 100;
    public static final String NAME = "test";
}
"#;
        let mut parser = create_java_parser().unwrap();
        let result = parser.parse_file(Path::new("Constants.java"), code);
        let consts: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Const)
            .collect();
        assert_eq!(consts.len(), 2);
    }
}
