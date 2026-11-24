//! Symbol index types and serialization.

use serde::{Deserialize, Serialize};

/// Symbol kind extracted from source code (multi-language).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    // Common across languages
    Fn,
    Struct,
    Enum,
    Trait,
    Const,
    Type,
    Class,
    Interface,
    Variable,
    Import,
    Export,
    Decorator,
    // Rust-specific
    Impl,
    Mod,
    Macro,
    // Dart-specific
    Mixin,
    Extension,
    Typedef,
    // Language identifier for debugging
    Other(String),
}

/// Visibility of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Pub,
    PubCrate,
    PubSuper,
    Private,
}

/// A symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol kind
    pub kind: SymbolKind,
    /// Simple symbol name
    pub name: String,
    /// Fully qualified name (crate::module::name)
    pub qualified_name: String,
    /// Full signature as string
    pub signature: String,
    /// Relative file path from root
    pub file: String,
    /// Start line (1-indexed)
    pub line: usize,
    /// End line (1-indexed, inclusive)
    pub end_line: usize,
    /// Doc comment (trimmed, without leading `///` or `//!`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Traceability tags (e.g., ["\@impl:FR-PAY-01", "\@test:FR-PAY-01"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Visibility
    pub visibility: Visibility,
    /// Attributes (e.g., ["derive(Debug)", "inline"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<String>,
}

/// Symbol index containing all extracted symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndex {
    /// Header metadata
    pub header: IndexHeader,
    /// All extracted symbols
    pub symbols: Vec<Symbol>,
}

impl SymbolIndex {
    pub fn new(root: String) -> Self {
        Self {
            header: IndexHeader::new(root, 0, 0, 0),
            symbols: Vec::new(),
        }
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.push(symbol);
        self.header.symbols = self.symbols.len();
    }
}

/// Field schema for self-describing output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    /// Field type
    pub ty: String,
    /// Human-readable description
    pub description: String,
    /// Enum values (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Index fields schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFields {
    pub kind: FieldSchema,
    pub name: FieldSchema,
    pub qualified_name: FieldSchema,
    pub signature: FieldSchema,
    pub file: FieldSchema,
    pub line: FieldSchema,
    pub end_line: FieldSchema,
    pub doc: FieldSchema,
    pub tags: FieldSchema,
    pub visibility: FieldSchema,
    pub attrs: FieldSchema,
}

/// Header metadata for NDJSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHeader {
    /// Format version
    pub version: u32,
    /// Schema identifier
    pub schema: String,
    /// Root directory indexed
    pub root: String,
    /// Total symbols found
    pub symbols: usize,
    /// Files processed
    pub files: usize,
    /// Files skipped
    pub skipped: usize,
    /// Generation timestamp (RFC3339)
    pub generated: String,
    /// Symbol field schema (optional - only included when schema is requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<IndexFields>,
}

impl IndexHeader {
    pub fn new(root: String, symbols: usize, files: usize, skipped: usize) -> Self {
        Self {
            version: 1,
            schema: "semantic-index.v1".to_string(),
            root,
            symbols,
            files,
            skipped,
            generated: chrono::Utc::now().to_rfc3339(),
            fields: None, // Only populated when schema requested
        }
    }

    pub fn with_fields(mut self, fields: IndexFields) -> Self {
        self.fields = Some(fields);
        self
    }
}

/// Generate the full IndexFields schema.
pub fn generate_index_fields() -> IndexFields {
    IndexFields {
        kind: FieldSchema {
            ty: "string".to_string(),
            description: "Symbol kind: fn, struct, enum, trait, const, type, class, interface, variable, import, export, decorator, impl, mod, macro, mixin, extension, typedef".to_string(),
            enum_values: Some(vec![
                "fn".into(), "struct".into(), "enum".into(), "trait".into(), "const".into(), "type".into(),
                "class".into(), "interface".into(), "variable".into(), "import".into(), "export".into(),
                "decorator".into(), "impl".into(), "mod".into(), "macro".into(), "mixin".into(),
                "extension".into(), "typedef".into(), "other".into(),
            ]),
        },
        name: FieldSchema { ty: "string".to_string(), description: "Simple symbol name".to_string(), enum_values: None },
        qualified_name: FieldSchema { ty: "string".to_string(), description: "Fully qualified name (crate::module::name)".to_string(), enum_values: None },
        signature: FieldSchema { ty: "string".to_string(), description: "Full signature as string".to_string(), enum_values: None },
        file: FieldSchema { ty: "string".to_string(), description: "Relative file path from root".to_string(), enum_values: None },
        line: FieldSchema { ty: "integer".to_string(), description: "Start line (1-indexed)".to_string(), enum_values: None },
        end_line: FieldSchema { ty: "integer".to_string(), description: "End line (1-indexed, inclusive)".to_string(), enum_values: None },
        doc: FieldSchema { ty: "string".to_string(), description: "Doc comment (trimmed, without /// or //!)".to_string(), enum_values: None },
        tags: FieldSchema { ty: "array<string>".to_string(), description: r"Traceability tags (\@impl:FR-*, \@test:FR-*, \@criticality:high|low)".to_string(), enum_values: None },
        visibility: FieldSchema {
            ty: "string".to_string(),
            description: "Visibility: pub, pub_crate, pub_super, private".to_string(),
            enum_values: Some(vec!["pub".into(), "pub_crate".into(), "pub_super".into(), "private".into()]),
        },
        attrs: FieldSchema { ty: "array<string>".to_string(), description: "Attributes (derive, inline, etc.)".to_string(), enum_values: None },
    }
}

/// Generate JSON schema string for the semantic index format.
pub fn schema_json_string() -> String {
    let fields = generate_index_fields();
    let header = IndexHeader::new("".to_string(), 0, 0, 0).with_fields(fields);
    serde_json::to_string_pretty(&header).unwrap_or_default()
}
