//! Python parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::SourceParser;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use tree_sitter::Query;
use anyhow::Result;

/// Create a Python parser.
pub fn create_python_parser() -> Result<Box<dyn SourceParser>> {
    let py_language = tree_sitter_python::LANGUAGE.into();
    
    let py_query = Query::new(
        &py_language,
        include_str!("queries/python/tags.scm"),
    )?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/python/tags.scm:
    // 0: function_definition (sync) -> Fn
    // 1: decorated_definition (function) -> Fn
    // 2: class_method -> Fn
    // 2: class_definition (with superclasses) -> Class
    // 3: class_definition (without superclasses) -> Class
    // 4: decorator -> Decorator
    // 5: import_statement (combined) -> Import
    // 6: import_from_statement -> Import
    // 7: assignment -> Variable
    // 8: augmented_assignment -> Variable
    // 9: type_alias_statement -> Type
    // 10: doc comment
    // 11: attrs (decorator)
    let pattern_kind_map = vec![
        SymbolKind::Fn,         // 0: function_definition (sync)
        SymbolKind::Fn,         // 1: decorated_definition (function)
        SymbolKind::Fn,         // 2: class_method
        SymbolKind::Class,      // 3: class_definition (with superclasses)
        SymbolKind::Class,      // 4: class_definition (without superclasses)
        SymbolKind::Decorator,  // 5: decorator
        SymbolKind::Import,     // 6: import_statement (combined)
        SymbolKind::Import,     // 7: import_from_statement
        SymbolKind::Variable,   // 8: assignment
        SymbolKind::Variable,   // 9: augmented_assignment
        SymbolKind::Type,       // 10: type_alias_statement
        SymbolKind::Other("doc".into()),   // 11: doc comment
        SymbolKind::Other("attrs".into()), // 12: attrs
    ];

    let config = TreeSitterConfig {
        language_id: "python",
        extensions: &["py", "pyi", "pyx", "pyw"],
        language: py_language,
        query: py_query,
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
    fn test_parse_python_function() {
        let code = r#"
def add(x: int, y: int) -> int:
    return x + y
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
        assert!(sym.signature.contains("x: int"));
        assert!(sym.signature.contains("y: int"));
        assert!(sym.signature.contains("-> int"));
    }

    #[test]
    fn test_parse_python_class() {
        let code = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
    
    def distance(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        eprintln!("DEBUG: All symbols found ({}):", result.symbols.len());
        for sym in &result.symbols {
            eprintln!("  - kind: {:?}, name: {}, line: {}", sym.kind, sym.name, sym.line);
        }
        let class_sym = result.symbols.iter().find(|s| s.kind == SymbolKind::Class);
        if class_sym.is_none() {
            eprintln!("DEBUG: No class found! All kinds: {:?}", result.symbols.iter().map(|s| format!("{:?}", s.kind)).collect::<Vec<_>>());
        }
        let class_sym = class_sym.unwrap();
        assert_eq!(class_sym.name, "Point");
        let method_sym = result.symbols.iter().find(|s| s.kind == SymbolKind::Fn && s.name == "distance").unwrap();
        assert_eq!(method_sym.name, "distance");
    }

    #[test]
    fn test_parse_python_async_function() {
        let code = r#"
async def fetch_data(url: str) -> dict:
    return {"url": url}
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "fetch_data");
    }

    #[test]
    fn test_parse_python_import() {
        let code = r#"
import os
import sys as system
from typing import List, Dict
from collections import defaultdict
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        let imports: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Import).collect();
        assert_eq!(imports.len(), 4);
    }

    #[test]
    fn test_parse_python_variable() {
        let code = r#"
x = 42
y: int = 10
PI = 3.14159
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        let variables: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Variable).collect();
        assert_eq!(variables.len(), 3);
    }

    #[test]
    fn test_parse_python_decorator() {
        let code = r#"
@property
def name(self) -> str:
    return self._name

@staticmethod
def helper() -> int:
    return 42
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        let decorators: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Decorator).collect();
        assert_eq!(decorators.len(), 2);
    }

    #[test]
    fn test_parse_python_type_alias() {
        let code = r#"
type UserId = int
type Point = tuple[int, int]
"#;
        let mut parser = create_python_parser().unwrap();
        let result = parser.parse_file(Path::new("test.py"), code);
        let types: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Type).collect();
        assert_eq!(types.len(), 2);
    }
}