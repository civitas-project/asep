//! Base Tree-sitter parser implementation.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator};

use crate::index::{Symbol, SymbolKind, Visibility};
use crate::parser::{ParseResult, SourceParser};

/// Configuration for a tree-sitter parser.
pub struct TreeSitterConfig {
    pub language_id: &'static str,
    pub extensions: &'static [&'static str],
    pub language: Language,
    pub query: Query,
    pub kind_capture: &'static str,
    pub name_capture: &'static str,
    pub signature_capture: Option<&'static str>,
    pub return_type_capture: Option<&'static str>,
    pub doc_capture: Option<&'static str>,
    pub visibility_capture: Option<&'static str>,
    pub attrs_capture: Option<&'static str>,
    pub pattern_kind_map: Option<Vec<SymbolKind>>,
    pub capture_names: Vec<String>,
}

/// A parser using tree-sitter.
pub struct TreeSitterParser {
    config: TreeSitterConfig,
    parser: Parser,
    root: PathBuf,
    current_file: PathBuf,
}

impl TreeSitterParser {
    pub fn new(mut config: TreeSitterConfig) -> Result<Self> {
        let capture_names: Vec<String> = config
            .query
            .capture_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        config.capture_names = capture_names;
        let mut parser = Parser::new();
        parser.set_language(&config.language)?;
        Ok(Self {
            config,
            parser,
            root: PathBuf::new(),
            current_file: PathBuf::new(),
        })
    }

    fn set_context(&mut self, root: &Path, path: &Path) {
        self.root = root.to_path_buf();
        self.current_file = path.to_path_buf();
    }

    fn rel_path(&self) -> String {
        self.current_file
            .strip_prefix(&self.root)
            .unwrap_or(&self.current_file)
            .to_string_lossy()
            .to_string()
    }

    fn node_to_symbol(
        &self,
        node: Node,
        source: &str,
        captures: &[(Node, &str)],
        pattern_index: usize,
    ) -> Option<Symbol> {
        let mut kind = SymbolKind::Other(self.config.language_id.to_string());
        let mut name = String::new();
        let mut signature = String::new();
        let mut return_type = String::new();
        let mut doc = None;
        let mut visibility_node = None;
        let mut attrs = Vec::new();

        let position_node = captures.first().map(|(n, _)| *n).unwrap_or(node);

        for (capture_node, capture_name) in captures {
            let capture_name_str = *capture_name;
            if capture_name_str == self.config.kind_capture {
                kind = self.node_kind_to_symbol_kind(*capture_node);
                break;
            }
        }

        if matches!(kind, SymbolKind::Other(_)) {
            kind = self
                .config
                .pattern_kind_map
                .as_ref()
                .and_then(|map| map.get(pattern_index))
                .cloned()
                .unwrap_or(kind);
        }

        for (capture_node, capture_name) in captures {
            let capture_name_str = *capture_name;
            if capture_name_str == self.config.name_capture {
                name = capture_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                continue;
            }
            if self.config.signature_capture == Some(capture_name_str) {
                signature = capture_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                continue;
            }
            if self.config.return_type_capture == Some(capture_name_str) {
                return_type = capture_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                continue;
            }
            if self.config.doc_capture == Some(capture_name_str) {
                doc = Some(
                    capture_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .to_string(),
                );
                continue;
            }
            if self.config.visibility_capture == Some(capture_name_str) {
                visibility_node = Some(*capture_node);
                continue;
            }
            if self.config.attrs_capture == Some(capture_name_str) {
                attrs.push(
                    capture_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .to_string(),
                );
            }
        }

        if !return_type.is_empty() {
            signature = append_return_type(&signature, &return_type);
        }

        if matches!(kind, SymbolKind::Fn) && !name.is_empty() && !signature.starts_with(&name) {
            signature = format!("function {} {}", name, signature);
        }

        if matches!(kind, SymbolKind::Class) && !name.is_empty() {
            signature = prefixed_signature("class", &name, signature);
        }

        if matches!(kind, SymbolKind::Interface) && !name.is_empty() {
            signature = prefixed_signature("interface", &name, signature);
        }

        if name.is_empty() {
            return None;
        }

        let line = (position_node.start_position().row as u32 + 1) as usize;
        let end_line = (position_node.end_position().row as u32 + 1) as usize;

        let visibility = (|| {
            match (visibility_node.is_some(), name.is_empty()) {
                (false, true) => return self.parse_visibility(position_node, source),
                (false, false) => return visibility_by_convention(self.config.language_id, &name),
                (true, _) => {}
            }
            let vis_node = visibility_node.unwrap();
            let vis = self.parse_visibility(vis_node, source);
            let vis_text_empty = vis_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .is_empty();
            if !name.is_empty() && vis_text_empty {
                return visibility_by_convention(self.config.language_id, &name);
            }
            vis
        })();

        let qualified_name = self.qualified_name(&name);
        let tags = self.extract_tags(&doc);

        Some(Symbol {
            kind,
            name,
            qualified_name,
            signature,
            file: self.rel_path(),
            line,
            end_line,
            doc,
            tags,
            visibility,
            attrs,
        })
    }

    fn node_kind_to_symbol_kind(&self, node: Node) -> SymbolKind {
        let kind = node.kind();
        match kind {
            "function_declaration"
            | "function_definition"
            | "method_definition"
            | "arrow_function"
            | "function_signature" => SymbolKind::Fn,
            "class_declaration" | "class_definition" => SymbolKind::Class,
            "interface_declaration" | "interface_definition" => SymbolKind::Interface,
            "enum_declaration" | "enum_definition" => SymbolKind::Enum,
            "const_declaration" | "const_definition" | "lexical_declaration" => SymbolKind::Const,
            "type_alias_declaration" | "type_definition" => SymbolKind::Type,
            "variable_declaration" | "variable_definition" => SymbolKind::Variable,
            "import_statement" | "import_declaration" | "from_import" => SymbolKind::Import,
            "export_statement" | "export_declaration" => SymbolKind::Export,
            "impl_item" => SymbolKind::Impl,
            "mod_item" => SymbolKind::Mod,
            "macro_definition" | "macro_rules" => SymbolKind::Macro,
            "mixin_declaration" => SymbolKind::Mixin,
            "extension_declaration" => SymbolKind::Extension,
            "typedef_declaration" => SymbolKind::Typedef,
            _ => SymbolKind::Other(self.config.language_id.to_string()),
        }
    }

    fn parse_visibility(&self, node: Node, source: &str) -> Visibility {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
        match text {
            "pub" | "public" | "export" => Visibility::Pub,
            "pub(crate)" | "crate" => Visibility::PubCrate,
            "pub(super)" | "super" => Visibility::PubSuper,
            "protected" | "private" => Visibility::Private,
            _ => {
                // If node text is empty or unknown, check if name starts with `_` for Python/Dart private convention.
                let is_private = match self.extract_node_name(node, source) {
                    Some(name) => name.starts_with('_'),
                    None => text.starts_with('_') || text.is_empty(),
                };
                private_or_pub(is_private)
            }
        }
    }

    fn extract_node_name<'a>(&self, node: Node<'a>, source: &'a str) -> Option<&'a str> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return name_node.utf8_text(source.as_bytes()).ok();
        }
        if let Some(name_node) = node
            .parent()
            .and_then(|parent| parent.child_by_field_name("name"))
        {
            return name_node.utf8_text(source.as_bytes()).ok();
        }
        let kind = node.kind();
        if kind == "identifier" || kind == "type_identifier" || kind == "field_identifier" {
            return node.utf8_text(source.as_bytes()).ok();
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let child_kind = child.kind();
            if child_kind == "identifier" || child_kind == "type_identifier" {
                return child.utf8_text(source.as_bytes()).ok();
            }
        }
        None
    }

    fn extract_tags(&self, doc: &Option<String>) -> Vec<String> {
        let mut tags = Vec::new();
        let doc = match doc {
            Some(d) => d,
            None => return tags,
        };
        for line in doc.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@impl:")
                || trimmed.starts_with("@test:")
                || trimmed.starts_with("@criticality:")
            {
                tags.push(trimmed.to_string());
            }
        }
        tags
    }

    fn qualified_name(&self, name: &str) -> String {
        let rel = self.rel_path();
        let normalized = rel.replace('\\', "/");
        let path = Path::new(&normalized);

        let mut parts = Vec::new();
        if let Some(parent) = path.parent() {
            for s in parent.components().filter_map(|comp| match comp {
                std::path::Component::Normal(c) => c.to_str().filter(|s| !s.is_empty()),
                _ => None,
            }) {
                parts.push(s);
            }
        }
        if let Some(stem) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
        {
            parts.push(stem);
        }

        if parts.is_empty() {
            return name.to_string();
        }
        format!("{}::{}", parts.join("::"), name)
    }
}

fn append_return_type(signature: &str, return_type: &str) -> String {
    let clean_return_type = return_type.trim_start_matches(':').trim();
    if signature.is_empty() {
        return format!(" -> {}", clean_return_type);
    }
    format!("{} -> {}", signature, clean_return_type)
}

fn prefixed_signature(prefix: &str, name: &str, signature: String) -> String {
    if signature.is_empty() {
        return format!("{prefix} {name}");
    }
    let marked = format!("{prefix} ");
    if signature.starts_with(&marked) {
        return signature;
    }
    format!("{prefix} {name} {signature}")
}

fn visibility_by_convention(language_id: &str, name: &str) -> Visibility {
    if language_id == "go" {
        let is_uppercase = name.chars().next().is_some_and(|c| c.is_uppercase());
        return match is_uppercase {
            true => Visibility::Pub,
            false => Visibility::Private,
        };
    }
    if name.starts_with('_') {
        return Visibility::Private;
    }
    Visibility::Pub
}

fn private_or_pub(is_private: bool) -> Visibility {
    if is_private {
        return Visibility::Private;
    }
    Visibility::Pub
}

impl SourceParser for TreeSitterParser {
    fn parse_file(&mut self, path: &Path, content: &str) -> ParseResult {
        let root = self.root.clone();
        self.set_context(&root, path);

        let tree = match self.parser.parse(content, None) {
            Some(t) => t,
            None => {
                return ParseResult {
                    symbols: Vec::new(),
                    errors: vec!["Failed to parse file".to_string()],
                };
            }
        };

        let mut query_cursor = QueryCursor::new();
        let mut matches_iter =
            query_cursor.matches(&self.config.query, tree.root_node(), content.as_bytes());

        let capture_names = &self.config.capture_names;
        let mut match_data = Vec::new();

        while let Some(match_) = matches_iter.next() {
            let captures: Vec<(Node, &str)> = match_
                .captures()
                .iter()
                .map(|c| {
                    let capture_name = &capture_names[c.index as usize];
                    (c.node, capture_name.as_str())
                })
                .collect();

            let pattern_node = match_
                .captures()
                .first()
                .map(|c| c.node)
                .unwrap_or_else(|| tree.root_node());
            match_data.push((pattern_node, captures, match_.pattern_index));
        }

        let mut symbols = Vec::new();
        for (pattern_node, captures, pattern_index) in match_data {
            if let Some(symbol) =
                self.node_to_symbol(pattern_node, content, &captures, pattern_index)
            {
                symbols.push(symbol);
            }
        }

        ParseResult {
            symbols,
            errors: Vec::new(),
        }
    }

    fn extensions(&self) -> &[&str] {
        self.config.extensions
    }

    fn language_id(&self) -> &'static str {
        self.config.language_id
    }
}

pub fn load_language(language_fn: fn() -> Language) -> Language {
    language_fn()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_parser() -> TreeSitterParser {
        let ts_language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let query_source = "(function_declaration name: (identifier) @name) @fn";
        let query = Query::new(&ts_language, query_source).unwrap();
        let config = TreeSitterConfig {
            language_id: "typescript",
            extensions: &["ts", "tsx"],
            language: ts_language,
            query,
            kind_capture: "fn",
            name_capture: "name",
            signature_capture: None,
            return_type_capture: None,
            doc_capture: None,
            visibility_capture: None,
            attrs_capture: None,
            pattern_kind_map: None,
            capture_names: Vec::new(),
        };
        TreeSitterParser::new(config).unwrap()
    }

    #[test]
    fn test_qualified_name_various_extensions() {
        let mut parser = create_test_parser();
        let root = Path::new("/project");

        parser.set_context(root, Path::new("/project/src/components/button.tsx"));
        assert_eq!(
            parser.qualified_name("Button"),
            "src::components::button::Button"
        );

        parser.set_context(root, Path::new("/project/src/utils.ts"));
        assert_eq!(parser.qualified_name("helper"), "src::utils::helper");

        parser.set_context(root, Path::new("/project/lib/App.jsx"));
        assert_eq!(parser.qualified_name("App"), "lib::App::App");

        parser.set_context(root, Path::new("/project/backend/Main.java"));
        assert_eq!(parser.qualified_name("main"), "backend::Main::main");

        parser.set_context(root, Path::new("/project/api/index.php"));
        assert_eq!(parser.qualified_name("run"), "api::index::run");

        parser.set_context(root, Path::new("/project/docs/paper.typ"));
        assert_eq!(parser.qualified_name("heading"), "docs::paper::heading");

        parser.set_context(root, Path::new("/project/single.dart"));
        assert_eq!(parser.qualified_name("run"), "single::run");
    }

    #[test]
    fn test_parse_visibility_keywords_and_conventions() {
        let parser = create_test_parser();
        let py_lang = tree_sitter_python::LANGUAGE.into();
        let mut py_parser = Parser::new();
        py_parser.set_language(&py_lang).unwrap();

        let source = r#"
def public_func():
    pass

def _private_func():
    pass
"#;
        let tree = py_parser.parse(source, None).unwrap();
        let root_node = tree.root_node();
        let fn1 = root_node.child(0).unwrap(); // public_func
        let fn2 = root_node.child(1).unwrap(); // _private_func

        assert_eq!(parser.parse_visibility(fn1, source), Visibility::Pub);
        assert_eq!(parser.parse_visibility(fn2, source), Visibility::Private);
    }

    #[test]
    fn test_parse_visibility_explicit_keywords() {
        let parser = create_test_parser();
        let ts_lang = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let mut ts_parser = Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();

        let source = r#"
export function fn1() {}
public function fn2() {}
protected function fn3() {}
private function fn4() {}
"#;
        let tree = ts_parser.parse(source, None).unwrap();
        let root_node = tree.root_node();
        // Check nodes with explicit keywords
        let mut cursor = root_node.walk();
        for child in root_node.children(&mut cursor) {
            let text = child.utf8_text(source.as_bytes()).unwrap();
            if text.starts_with("export") {
                // First token is export
                let export_kw = child.child(0).unwrap();
                assert_eq!(parser.parse_visibility(export_kw, source), Visibility::Pub);
            }
        }
    }

    #[test]
    fn test_parse_file_zero_copy_capture_names() {
        let mut parser = create_test_parser();
        let source = "function hello() {}\nfunction world() {}";
        let result = parser.parse_file(Path::new("src/index.ts"), source);
        assert_eq!(result.symbols.len(), 2);
        assert_eq!(result.symbols[0].name, "hello");
        assert_eq!(result.symbols[0].qualified_name, "src::index::hello");
        assert_eq!(result.symbols[1].name, "world");
        assert_eq!(result.symbols[1].qualified_name, "src::index::world");
        assert!(result.errors.is_empty());
    }
}
