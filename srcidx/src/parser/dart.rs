//! Dart parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use crate::parser::SourceParser;
use anyhow::Result;
use tree_sitter::Query;

/// Create a Dart parser.
pub fn create_dart_parser() -> Result<Box<dyn SourceParser>> {
    let dart_language = tree_sitter_dart::LANGUAGE.into();

    let dart_query = Query::new(&dart_language, include_str!("queries/dart/tags.scm"))?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/dart/tags.scm:
    // 0: function_declaration -> Fn
    // 1: method_declaration -> Fn
    // 2: constructor_signature -> Fn
    // 3: class_declaration -> Class
    // 4: enum_declaration -> Enum
    // 5: mixin_declaration -> Mixin
    // 6: extension_declaration -> Extension
    // 7: type_alias -> Typedef
    // 8: import_or_export -> Import
    // 9: doc comment
    // 10: attrs (annotation)
    let pattern_kind_map = vec![
        SymbolKind::Fn,                    // 0: function_declaration
        SymbolKind::Fn,                    // 1: method_declaration
        SymbolKind::Fn,                    // 2: constructor_signature
        SymbolKind::Class,                 // 3: class_declaration
        SymbolKind::Enum,                  // 4: enum_declaration
        SymbolKind::Mixin,                 // 5: mixin_declaration
        SymbolKind::Extension,             // 6: extension_declaration
        SymbolKind::Typedef,               // 7: type_alias
        SymbolKind::Import,                // 8: import_or_export
        SymbolKind::Other("doc".into()),   // 9: doc comment
        SymbolKind::Other("attrs".into()), // 10: attrs
    ];

    let config = TreeSitterConfig {
        language_id: "dart",
        extensions: &["dart"],
        language: dart_language,
        query: dart_query,
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
    fn test_parse_dart_function() {
        let code = r#"
int add(int x, int y) {
  return x + y;
}
"#;
        let mut parser = create_dart_parser().unwrap();
        let result = parser.parse_file(Path::new("test.dart"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
    }

    #[test]
    fn test_parse_dart_class() {
        let code = r#"
class Point {
  int x;
  int y;
  
  Point(this.x, this.y);
  
  double distance() {
    return (x * x + y * y).sqrt();
  }
}
"#;
        let mut parser = create_dart_parser().unwrap();
        let result = parser.parse_file(Path::new("test.dart"), code);
        let class_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Class)
            .unwrap();
        assert_eq!(class_sym.name, "Point");
        let method_sym = result
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Fn && s.name == "distance")
            .unwrap();
        assert_eq!(method_sym.name, "distance");
    }

    #[test]
    fn test_parse_dart_import() {
        let code = r#"
import 'dart:math';
import 'package:flutter/material.dart';
"#;
        let mut parser = create_dart_parser().unwrap();
        let result = parser.parse_file(Path::new("test.dart"), code);
        let imports: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Import)
            .collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_dart_enum() {
        let code = r#"
enum Color {
  red,
  green,
  blue,
}
"#;
        let mut parser = create_dart_parser().unwrap();
        let result = parser.parse_file(Path::new("test.dart"), code);
        let enums: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Color");
    }

    #[test]
    fn test_parse_dart_mixin() {
        let code = r#"
mixin Logger {
  void log(String message) {
    print(message);
  }
}
"#;
        let mut parser = create_dart_parser().unwrap();
        let result = parser.parse_file(Path::new("test.dart"), code);
        let mixins: Vec<_> = result
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Mixin)
            .collect();
        assert_eq!(mixins.len(), 1);
    }
}
