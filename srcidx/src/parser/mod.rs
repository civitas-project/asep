//! Parser trait for extracting symbols from source files.

pub mod dart;
pub mod go;
pub mod java;
pub mod markup;
pub mod php;
pub mod python;
pub mod rust;
pub mod tree_sitter_base;
pub mod typescript;
pub mod typst;

use crate::index::{Symbol, Visibility};
use std::path::Path;
use std::sync::{Arc, RwLock};

// Re-export tree-sitter languages for use in srcdump

/// Result of parsing a single file.
#[derive(Debug, Default)]
pub struct ParseResult {
    pub symbols: Vec<Symbol>,
    pub errors: Vec<String>,
}

/// Trait for language-specific parsers.
pub trait SourceParser: Send + Sync {
    /// Parse a single file and extract symbols.
    fn parse_file(&mut self, path: &Path, content: &str) -> ParseResult;

    /// File extensions this parser handles.
    fn extensions(&self) -> &[&str];

    /// Language identifier (e.g., "rust", "typescript", "python")
    fn language_id(&self) -> &'static str;

    /// Check if this parser can handle the given file.
    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.extensions().contains(&ext))
            .unwrap_or(false)
    }
}

type ParserFactory = Box<dyn Fn() -> Box<dyn SourceParser> + Send + Sync>;
type PathPredicate = Box<dyn Fn(&Path) -> bool + Send + Sync>;

struct ParserEntry {
    matcher: PathPredicate,
    factory: Option<ParserFactory>,
    instance: Option<Arc<RwLock<Box<dyn SourceParser>>>>,
}

impl ParserEntry {
    fn matches(&self, path: &Path) -> bool {
        (self.matcher)(path)
    }

    fn get_or_init(&mut self) -> Arc<RwLock<Box<dyn SourceParser>>> {
        if let Some(ref inst) = self.instance {
            return Arc::clone(inst);
        }
        let factory = match &self.factory {
            Some(f) => f.as_ref(),
            None => panic!("ParserEntry without instance or factory"),
        };
        let parser = factory();
        let inst = Arc::new(RwLock::new(parser));
        self.instance = Some(Arc::clone(&inst));
        inst
    }

    fn is_initialized(&self) -> bool {
        self.instance.is_some()
    }
}

/// Thread-safe parser registry with lazy initialization.
#[derive(Clone)]
pub struct ParserRegistry {
    // ponytail: linear scan over small Vec of parsers (14 entries); upgrade to extension HashMap if parser count exceeds 50.
    entries: Arc<RwLock<Vec<ParserEntry>>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn register(&mut self, parser: Box<dyn SourceParser>) {
        let extensions: Vec<String> = parser.extensions().iter().map(|s| s.to_string()).collect();
        let instance = Arc::new(RwLock::new(parser));
        let inst_clone = Arc::clone(&instance);
        let matcher: PathPredicate = Box::new(move |p: &Path| {
            match inst_clone.read() {
                Ok(guard) => guard.can_parse(p),
                Err(_) => p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| extensions.iter().any(|s| s == ext))
                    .unwrap_or(false),
            }
        });

        self.entries.write().unwrap().push(ParserEntry {
            matcher,
            factory: None,
            instance: Some(instance),
        });
    }

    pub fn register_lazy<P, F>(&mut self, predicate: P, factory: F)
    where
        P: Fn(&Path) -> bool + Send + Sync + 'static,
        F: Fn() -> Box<dyn SourceParser> + Send + Sync + 'static,
    {
        self.entries.write().unwrap().push(ParserEntry {
            matcher: Box::new(predicate),
            factory: Some(Box::new(factory)),
            instance: None,
        });
    }

    pub fn register_lazy_extensions<F>(&mut self, extensions: &'static [&'static str], factory: F)
    where
        F: Fn() -> Box<dyn SourceParser> + Send + Sync + 'static,
    {
        self.register_lazy(
            move |path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| extensions.contains(&ext))
                    .unwrap_or(false)
            },
            factory,
        );
    }

    pub fn register_factory(
        &mut self,
        factory: Box<dyn Fn() -> Box<dyn SourceParser> + Send + Sync>,
    ) {
        let parser = factory();
        self.register(parser);
    }

    /// Find a parser that can handle the given path, initializing it on first use.
    pub fn find_parser(&self, path: &Path) -> Option<Arc<RwLock<Box<dyn SourceParser>>>> {
        let mut matched_uninitialized = false;
        // Fast path: check with read lock
        {
            let entries = self.entries.read().unwrap();
            for entry in entries.iter() {
                if !entry.matches(path) {
                    continue;
                }
                if let Some(ref inst) = entry.instance {
                    return Some(Arc::clone(inst));
                }
                matched_uninitialized = true;
                break;
            }
        }

        // If no matching entry exists, avoid taking write lock
        if !matched_uninitialized {
            return None;
        }

        // Slow path: matching entry found, initialize under write lock
        let mut entries = self.entries.write().unwrap();
        for entry in entries.iter_mut() {
            if entry.matches(path) {
                return Some(entry.get_or_init());
            }
        }
        None
    }

    /// Get the tree-sitter Language for a given language ID.
    pub fn get_language_for_id(language_id: &str) -> Option<tree_sitter::Language> {
        match language_id {
            "typescript" | "javascript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "python" => Some(tree_sitter_python::LANGUAGE.into()),
            "dart" => Some(tree_sitter_dart::LANGUAGE.into()),
            "go" => Some(tree_sitter_go::LANGUAGE.into()),
            "php" => Some(tree_sitter_php::LANGUAGE_PHP.into()),
            "java" => Some(tree_sitter_java::LANGUAGE.into()),
            "typst" => Some(codebook_tree_sitter_typst::LANGUAGE.into()),
            _ => None,
        }
    }

    /// Get the tree-sitter Language for a parser that can handle the given path.
    pub fn get_language_for_path(&self, path: &Path) -> Option<tree_sitter::Language> {
        self.find_parser(path).and_then(|parser| {
            let parser_guard = parser.read().unwrap();
            Self::get_language_for_id(parser_guard.language_id())
        })
    }

    /// Find a parser and call a closure with it mutably.
    pub fn with_parser<F, R>(&self, path: &Path, f: F) -> Option<R>
    where
        F: FnOnce(&mut dyn SourceParser) -> R,
    {
        let parser = self.find_parser(path)?;
        let mut guard = parser.write().unwrap();
        Some(f(guard.as_mut()))
    }

    pub fn can_parse(&self, path: &Path) -> bool {
        self.entries.read().unwrap().iter().any(|e| e.matches(path))
    }

    pub fn parse_file(&self, path: &Path, content: &str) -> ParseResult {
        if let Some(parser) = self.find_parser(path) {
            return parser.write().unwrap().parse_file(path, content);
        }
        ParseResult {
            symbols: Vec::new(),
            errors: vec![format!("No parser for file: {}", path.display())],
        }
    }

    /// Returns the number of registered parsers (both initialized and lazy).
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Returns true if there are no registered parsers.
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }

    /// Returns the count of currently initialized parsers.
    pub fn initialized_count(&self) -> usize {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.is_initialized())
            .count()
    }

    /// Checks if a parser matching `path` is currently initialized.
    /// Returns `Some(true)` if initialized, `Some(false)` if registered but not yet initialized,
    /// or `None` if no parser matches `path`.
    pub fn is_initialized(&self, path: &Path) -> Option<bool> {
        self.entries
            .read()
            .unwrap()
            .iter()
            .find(|e| e.matches(path))
            .map(|e| e.is_initialized())
    }

    /// Get all parsers, initializing any uninitialized parsers.
    pub fn iter_parsers(&self) -> Vec<Arc<RwLock<Box<dyn SourceParser>>>> {
        let mut entries = self.entries.write().unwrap();
        entries
            .iter_mut()
            .map(|entry| entry.get_or_init())
            .collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry
            .register_lazy_extensions(&["rs"], || Box::new(crate::parser::rust::RustParser::new()));
        registry.register_lazy_extensions(&["ts", "tsx", "js", "jsx", "mjs", "cjs"], || {
            crate::parser::typescript::create_typescript_parser().unwrap()
        });
        registry.register_lazy_extensions(&["typ"], || {
            crate::parser::typst::create_typst_parser().unwrap()
        });
        registry.register_lazy_extensions(&["py", "pyi", "pyx", "pyw"], || {
            crate::parser::python::create_python_parser().unwrap()
        });
        registry.register_lazy_extensions(&["dart"], || {
            crate::parser::dart::create_dart_parser().unwrap()
        });
        registry
            .register_lazy_extensions(&["go"], || crate::parser::go::create_go_parser().unwrap());
        registry.register_lazy_extensions(&["php", "php8", "phtml"], || {
            crate::parser::php::create_php_parser().unwrap()
        });
        registry.register_lazy_extensions(&["java"], || {
            crate::parser::java::create_java_parser().unwrap()
        });
        registry.register_lazy_extensions(&["md", "mdx"], || {
            crate::parser::markup::MarkupParser::markdown()
        });
        registry.register_lazy_extensions(&["rst"], || crate::parser::markup::MarkupParser::rst());
        registry.register_lazy_extensions(&["adoc"], || {
            crate::parser::markup::MarkupParser::asciidoc()
        });
        registry.register_lazy_extensions(&["html", "htm"], || {
            crate::parser::markup::MarkupParser::html()
        });
        registry.register_lazy_extensions(&["xml"], || crate::parser::markup::MarkupParser::xml());
        registry.register_lazy_extensions(&["tex", "latex"], || {
            crate::parser::markup::MarkupParser::latex()
        });
        registry
    }
}

/// Convert syn Visibility to our Visibility enum.
pub fn syn_vis_to_visibility(vis: &syn::Visibility) -> Visibility {
    match vis {
        syn::Visibility::Public(_) => Visibility::Pub,
        syn::Visibility::Restricted(vis) => {
            if vis.path.segments.len() != 1 {
                return Visibility::Private;
            }
            match vis.path.segments[0].ident.to_string().as_str() {
                "crate" => Visibility::PubCrate,
                "super" => Visibility::PubSuper,
                _ => Visibility::Private,
            }
        }
        syn::Visibility::Inherited => Visibility::Private,
    }
}

/// Extract doc comment from attributes.
pub fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if let Some(s) = doc_string_value(attr) {
            docs.push(s);
        }
    }
    if docs.is_empty() {
        return None;
    }
    Some(docs.join("\n"))
}

/// Extract a single doc string value from an attribute, if present.
fn doc_string_value(attr: &syn::Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }
    let meta = match &attr.meta {
        syn::Meta::NameValue(meta) => meta,
        _ => return None,
    };
    let expr = match &meta.value {
        syn::Expr::Lit(expr) => expr,
        _ => return None,
    };
    let lit = match &expr.lit {
        syn::Lit::Str(lit) => lit,
        _ => return None,
    };
    Some(lit.value().trim_start_matches(' ').to_string())
}

/// Extract traceability tags from doc comment.
pub fn extract_tags(doc: &Option<String>) -> Vec<String> {
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

/// Extract attributes as strings (for debugging/display).
pub fn extract_attrs(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc"))
        .map(|a| quote::quote!(#a).to_string())
        .collect()
}

/// Convert syn Visibility to our Visibility enum.
pub fn parse_visibility(text: &str) -> Visibility {
    match text {
        "pub" | "public" | "export" => Visibility::Pub,
        "pub(crate)" | "crate" => Visibility::PubCrate,
        "pub(super)" | "super" => Visibility::PubSuper,
        "private" | "protected" => Visibility::Private,
        _ => Visibility::Private,
    }
}

/// Extract traceability tags from doc comment.
pub fn extract_tags_from_doc(doc: &Option<String>) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_initialization() {
        let registry = ParserRegistry::default();
        assert_eq!(registry.len(), 14);
        assert_eq!(registry.initialized_count(), 0);
        assert_eq!(registry.is_initialized(Path::new("test.rs")), Some(false));
        assert_eq!(registry.is_initialized(Path::new("test.py")), Some(false));
        assert_eq!(registry.is_initialized(Path::new("test.go")), Some(false));

        // can_parse probes do not eagerly initialize the parser
        assert!(registry.can_parse(Path::new("src/lib.rs")));
        assert_eq!(registry.initialized_count(), 0);
        assert_eq!(registry.is_initialized(Path::new("test.rs")), Some(false));
        assert_eq!(registry.is_initialized(Path::new("test.py")), Some(false));

        // find_parser for Rust instantiates Rust parser
        assert!(registry.find_parser(Path::new("src/lib.rs")).is_some());
        assert_eq!(registry.initialized_count(), 1);
        assert_eq!(registry.is_initialized(Path::new("test.rs")), Some(true));
        assert_eq!(registry.is_initialized(Path::new("test.py")), Some(false));

        // find_parser for Python instantiates Python parser
        assert!(registry.find_parser(Path::new("script.py")).is_some());
        assert_eq!(registry.initialized_count(), 2);
        assert_eq!(registry.is_initialized(Path::new("test.py")), Some(true));
        assert_eq!(registry.is_initialized(Path::new("test.go")), Some(false));

        // Non-existent extension does not instantiate any parsers
        assert!(!registry.can_parse(Path::new("notes.txt")));
        assert_eq!(registry.initialized_count(), 2);
        assert_eq!(registry.is_initialized(Path::new("notes.txt")), None);
    }

    #[test]
    fn test_eager_parser_registration() {
        let mut registry = ParserRegistry::new();
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.initialized_count(), 0);

        registry.register(Box::new(crate::parser::rust::RustParser::new()));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.initialized_count(), 1);
        assert_eq!(registry.is_initialized(Path::new("main.rs")), Some(true));
        assert!(registry.can_parse(Path::new("main.rs")));
    }

    #[test]
    fn test_with_parser_and_parse_file_lazy() {
        let registry = ParserRegistry::default();
        assert_eq!(registry.initialized_count(), 0);

        let result = registry.parse_file(
            Path::new("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        );
        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "add");
        assert_eq!(registry.initialized_count(), 1);

        let lang = registry.with_parser(Path::new("lib.rs"), |p| p.language_id());
        assert_eq!(lang, Some("rust"));
    }

    #[test]
    fn test_concurrent_lazy_initialization() {
        use std::sync::Arc;
        use std::thread;

        let registry = Arc::new(ParserRegistry::default());
        let mut handles = Vec::new();

        for i in 0..20 {
            let reg = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let path = match i % 4 {
                    0 => Path::new("main.rs"),
                    1 => Path::new("app.py"),
                    2 => Path::new("index.ts"),
                    _ => Path::new("lib.go"),
                };
                assert!(reg.can_parse(path));
                assert!(reg.find_parser(path).is_some());
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(registry.initialized_count(), 4);
    }
}
