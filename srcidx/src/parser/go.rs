//! Go parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::SourceParser;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use tree_sitter::Query;
use anyhow::Result;

/// Create a Go parser.
pub fn create_go_parser() -> Result<Box<dyn SourceParser>> {
    let go_language = tree_sitter_go::LANGUAGE.into();
    
    let go_query = Query::new(
        &go_language,
        include_str!("queries/go/tags.scm"),
    )?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/go/tags.scm:
    // 0: function_declaration -> Fn
    // 1: method_declaration -> Fn
    // 2: struct -> Struct
    // 3: interface -> Interface
    // 4: type_alias -> Type
    // 5: const_declaration -> Const
    // 6: var_declaration -> Variable
    // 7: short_var_declaration -> Variable
    // 8: import_declaration (single) -> Import
    // 9: import_declaration (list) -> Import
    // 10: doc comment
    let pattern_kind_map = vec![
        SymbolKind::Fn,         // 0: function_declaration
        SymbolKind::Fn,         // 1: method_declaration
        SymbolKind::Struct,     // 2: struct
        SymbolKind::Interface,  // 3: interface
        SymbolKind::Type,       // 4: type_alias
        SymbolKind::Const,      // 5: const_declaration
        SymbolKind::Variable,   // 6: var_declaration
        SymbolKind::Variable,   // 7: short_var_declaration
        SymbolKind::Import,     // 8: import_declaration (single)
        SymbolKind::Import,     // 9: import_declaration (list)
        SymbolKind::Other("doc".into()),   // 10: doc comment
    ];

    let config = TreeSitterConfig {
        language_id: "go",
        extensions: &["go"],
        language: go_language,
        query: go_query,
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
    fn test_parse_go_function() {
        let code = r#"
package main

func add(x int, y int) int {
    return x + y
}
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
    }

    #[test]
    fn test_parse_go_method() {
        let code = r#"
package main

type Point struct {
    X int
    Y int
}

func (p Point) Distance() float64 {
    return 0.0
}
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        let method_sym = result.symbols.iter().find(|s| s.kind == SymbolKind::Fn && s.name == "Distance").unwrap();
        assert_eq!(method_sym.name, "Distance");
    }

    #[test]
    fn test_parse_go_struct() {
        let code = r#"
package main

type Point struct {
    X int
    Y int
}
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        let structs: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Struct).collect();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Point");
    }

    #[test]
    fn test_parse_go_interface() {
        let code = r#"
package main

type Shape interface {
    Area() float64
}
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        let interfaces: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Interface).collect();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Shape");
    }

    #[test]
    fn test_parse_go_import() {
        let code = r#"
package main

import (
    "fmt"
    "math"
)
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        let imports: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Import).collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_go_const() {
        let code = r#"
package main

const Pi = 3.14159
const MaxSize = 100
"#;
        let mut parser = create_go_parser().unwrap();
        let result = parser.parse_file(Path::new("test.go"), code);
        let consts: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(consts.len(), 2);
    }
}