//! PHP parser using tree-sitter.

use crate::index::SymbolKind;
use crate::parser::SourceParser;
use crate::parser::tree_sitter_base::{TreeSitterConfig, TreeSitterParser};
use tree_sitter::Query;
use anyhow::Result;

/// Create a PHP parser.
pub fn create_php_parser() -> Result<Box<dyn SourceParser>> {
    let php_language = tree_sitter_php::LANGUAGE_PHP.into();
    
    let php_query = Query::new(
        &php_language,
        include_str!("queries/php/tags.scm"),
    )?;

    // Pattern kind map: maps pattern index to symbol kind
    // Pattern indices (0-based) correspond to patterns in queries/php/tags.scm:
    // 0: function_definition -> Fn
    // 1: method_declaration -> Fn
    // 2: class_declaration -> Class
    // 3: interface_declaration -> Interface
    // 4: trait_declaration -> Trait
    // 5: enum_declaration -> Enum
    // 6: const_declaration -> Const
    // 7: property_declaration -> Variable
    // 8: namespace_use_declaration (name) -> Import
    // 9: namespace_use_declaration (qualified_name) -> Import
    // 10: namespace_use_declaration (relative_name) -> Import
    // 11: namespace_use_declaration (namespace_use_group name) -> Import
    // 12: namespace_use_declaration (namespace_use_group qualified_name) -> Import
    // 13: namespace_definition -> Mod
    // 14: doc comment
    // 15: attrs (attribute)
    let pattern_kind_map = vec![
        SymbolKind::Fn,         // 0: function_definition
        SymbolKind::Fn,         // 1: method_declaration
        SymbolKind::Class,      // 2: class_declaration
        SymbolKind::Interface,  // 3: interface_declaration
        SymbolKind::Trait,      // 4: trait_declaration
        SymbolKind::Enum,       // 5: enum_declaration
        SymbolKind::Const,      // 6: const_declaration
        SymbolKind::Variable,   // 7: property_declaration
        SymbolKind::Import,     // 8: namespace_use_declaration (name)
        SymbolKind::Import,     // 9: namespace_use_declaration (qualified_name)
        SymbolKind::Import,     // 10: namespace_use_declaration (relative_name)
        SymbolKind::Import,     // 11: namespace_use_declaration (namespace_use_group name)
        SymbolKind::Import,     // 12: namespace_use_declaration (namespace_use_group qualified_name)
        SymbolKind::Mod,        // 13: namespace_definition
        SymbolKind::Other("doc".into()),   // 14: doc comment
        SymbolKind::Other("attrs".into()), // 15: attrs
    ];

    let config = TreeSitterConfig {
        language_id: "php",
        extensions: &["php", "php8", "phtml"],
        language: php_language,
        query: php_query,
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
    fn test_parse_php_function() {
        let code = r#"
<?php
function add(int $x, int $y): int {
    return $x + $y;
}
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        assert_eq!(result.symbols.len(), 1);
        let sym = &result.symbols[0];
        assert_eq!(sym.kind, SymbolKind::Fn);
        assert_eq!(sym.name, "add");
    }

    #[test]
    fn test_parse_php_class() {
        let code = r#"
<?php
class Point {
    public int $x;
    public int $y;
    public function __construct(int $x, int $y) {
        $this->x = $x;
        $this->y = $y;
    }
    public function distance(): float {
        return sqrt($this->x ** 2 + $this->y ** 2);
    }
}
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        let class_sym = result.symbols.iter().find(|s| s.kind == SymbolKind::Class).unwrap();
        assert_eq!(class_sym.name, "Point");
        let method_sym = result.symbols.iter().find(|s| s.kind == SymbolKind::Fn && s.name == "distance").unwrap();
        assert_eq!(method_sym.name, "distance");
    }

    #[test]
    fn test_parse_php_interface() {
        let code = r#"
<?php
interface Shape {
    public function area(): float;
}
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        let interfaces: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Interface).collect();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Shape");
    }

    #[test]
    fn test_parse_php_trait() {
        let code = r#"
<?php
trait Logger {
    public function log(string $message): void {
        echo $message;
    }
}
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        let traits: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Trait).collect();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Logger");
    }

    #[test]
    fn test_parse_php_import() {
        let code = r#"
<?php
use App\Models\User;
use function App\Helpers\format;
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        let imports: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Import).collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_php_enum() {
        let code = r#"
<?php
enum Status: string {
    case Draft = 'draft';
    case Published = 'published';
}
"#;
        let mut parser = create_php_parser().unwrap();
        let result = parser.parse_file(Path::new("test.php"), code);
        let enums: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Enum).collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Status");
    }
}