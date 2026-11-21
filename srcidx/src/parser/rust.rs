//! Rust parser using syn.

use crate::index::{Symbol, SymbolKind, Visibility};
use crate::parser::{ParseResult, SourceParser};
use std::path::{Path, PathBuf};
use syn::Item;

pub struct RustParser {
    module_stack: Vec<String>,
}

fn clean_identifier(s: &str) -> String {
    let mut result = String::new();
    let mut prev_underscore = false;
    for c in s.chars() {
        let is_underscore = c == '_';
        let is_alphanumeric = c.is_alphanumeric();
        match (is_underscore, is_alphanumeric) {
            (true, _) => {
                if !prev_underscore && !result.is_empty() {
                    result.push('_');
                    prev_underscore = true;
                }
            }
            (false, true) => {
                result.push(c);
                prev_underscore = false;
            }
            (false, false) => {
                if !prev_underscore && !result.is_empty() {
                    result.push('_');
                    prev_underscore = true;
                }
            }
        }
    }
    result.trim_matches('_').to_string()
}

fn clean_type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => match type_path.path.segments.last() {
            Some(last) => last.ident.to_string(),
            None => {
                let s = quote::quote!(#ty).to_string();
                clean_identifier(&s)
            }
        },
        syn::Type::Reference(r) => clean_type_name(&r.elem),
        syn::Type::Slice(s) => format!("slice_{}", clean_type_name(&s.elem)),
        syn::Type::Array(a) => format!("array_{}", clean_type_name(&a.elem)),
        syn::Type::Tuple(t) => match t.elems.is_empty() {
            true => "unit".to_string(),
            false => "tuple".to_string(),
        },
        _ => {
            let s = quote::quote!(#ty).to_string();
            clean_identifier(&s)
        }
    }
}

fn clean_path_name(path: &syn::Path) -> String {
    match path.segments.last() {
        Some(last) => last.ident.to_string(),
        None => {
            let s = quote::quote!(#path).to_string();
            clean_identifier(&s)
        }
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustParser {
    pub fn new() -> Self {
        Self {
            module_stack: Vec::new(),
        }
    }

    fn span_line(&self, span: proc_macro2::Span) -> u32 {
        let lc = span.start();
        lc.line.try_into().unwrap_or(1) // BUG-23: Return 1 instead of 0 as fallback
    }

    fn span_line_end(&self, span: proc_macro2::Span) -> u32 {
        let lc = span.end();
        lc.line.try_into().unwrap_or(1)
    }

    // antislop: single-arm matches kept intentionally; behavior unchanged.
    #[allow(clippy::single_match)]
    fn extract_doc(&self, attrs: &[syn::Attribute]) -> Option<String> {
        let mut docs = Vec::new();
        for attr in attrs {
            if !attr.path().is_ident("doc") {
                continue;
            }
            match &attr.meta {
                syn::Meta::NameValue(meta) => {
                    match &meta.value {
                        syn::Expr::Lit(expr) => {
                            match &expr.lit {
                                syn::Lit::Str(lit) => {
                                    let s = lit.value();
                                    docs.push(s.trim_start_matches(' ').to_string());
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if docs.is_empty() {
            return None;
        }
        Some(docs.join("\n"))
    }

    fn extract_tags(&self, doc: &Option<String>) -> Vec<String> {
        let mut tags = Vec::new();
        let doc = match doc {
            Some(d) => d,
            None => return tags,
        };
        for line in doc.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@impl:") || trimmed.starts_with("@test:") || trimmed.starts_with("@criticality:") {
                tags.push(trimmed.to_string());
            }
        }
        tags
    }

    fn extract_attrs(&self, attrs: &[syn::Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter(|a| !a.path().is_ident("doc"))
            .map(|a| quote::quote!(#a).to_string())
            .collect()
    }

    fn syn_vis_to_visibility(vis: &syn::Visibility) -> Visibility {
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

    fn push_module(&mut self, name: &str) {
        self.module_stack.push(name.to_string());
    }

    fn pop_module(&mut self) {
        self.module_stack.pop();
    }

    fn current_module(&self) -> String {
        self.module_stack.join("::")
    }

    #[allow(clippy::too_many_arguments)]
    fn add_symbol(&self, symbols: &mut Vec<Symbol>, kind: SymbolKind, name: String, signature: String, line: u32, end_line: u32, doc: Option<String>, visibility: Visibility, attrs: Vec<String>) {
        let module_prefix = self.current_module();
        let qualified_name = match module_prefix.is_empty() {
            true => name.clone(),
            false => format!("{}::{}", module_prefix, name),
        };

        let tags = self.extract_tags(&doc);

        symbols.push(Symbol {
            kind,
            name,
            qualified_name,
            signature,
            file: String::new(), // Will be set by caller
            line: line as usize,
            end_line: end_line as usize,
            doc,
            tags,
            visibility,
            attrs,
        });
    }

    // antislop: guard clauses translate if/else into match on bool/Option by design.
    #[allow(clippy::match_bool, clippy::single_match)]
    fn parse_item(&mut self, item: &Item, symbols: &mut Vec<Symbol>) {
        match item {
            Item::Fn(node) => {
                let name = node.sig.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.sig.fn_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let mut signature = format!("fn {}", name);
                let generics = &node.sig.generics;
                if !generics.params.is_empty() {
                    let (_, ty_generics, _) = generics.split_for_impl();
                    signature.push_str(&quote::quote!(#ty_generics).to_string());
                }
                signature.push('(');
                for (i, input) in node.sig.inputs.iter().enumerate() {
                    if i > 0 { signature.push_str(", "); }
                    signature.push_str(&quote::quote!(#input).to_string());
                }
                signature.push(')');
                if let syn::ReturnType::Type(_, ty) = &node.sig.output {
                    signature.push_str(&format!(" -> {}", quote::quote!(#ty)));
                }

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Fn, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Struct(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.struct_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let mut signature = format!("struct {}", name);
                let generics = &node.generics;
                if !generics.params.is_empty() {
                    let (_, ty_generics, _) = generics.split_for_impl();
                    signature.push_str(&quote::quote!(#ty_generics).to_string());
                }
                signature.push_str(" { ");
                for (i, field) in node.fields.iter().enumerate() {
                    if i > 0 { signature.push_str(", "); }
                    let ty = &field.ty;
                    match &field.ident {
                        Some(ident) => {
                            signature.push_str(&format!("{}: {}", ident, quote::quote!(#ty)));
                        }
                        None => {
                            signature.push_str(&quote::quote!(#ty).to_string());
                        }
                    }
                }
                signature.push_str(" }");

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Struct, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Enum(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.enum_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let mut signature = format!("enum {}", name);
                let generics = &node.generics;
                if !generics.params.is_empty() {
                    let (_, ty_generics, _) = generics.split_for_impl();
                    signature.push_str(&quote::quote!(#ty_generics).to_string());
                }
                signature.push_str(" { ");
                for (i, variant) in node.variants.iter().enumerate() {
                    if i > 0 { signature.push_str(", "); }
                    signature.push_str(&variant.ident.to_string());
                    if !variant.fields.is_empty() {
                        signature.push_str(" { ... }");
                    }
                }
                signature.push_str(" }");

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Enum, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Trait(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.trait_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let mut signature = format!("trait {}", name);
                let generics = &node.generics;
                if !generics.params.is_empty() {
                    let (_, ty_generics, _) = generics.split_for_impl();
                    signature.push_str(&quote::quote!(#ty_generics).to_string());
                }
                if let Some(where_clause) = &generics.where_clause {
                    signature.push_str(&format!(" where {}", quote::quote!(#where_clause)));
                }
                signature.push_str(" { ... }");

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Trait, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Impl(node) => {
                let line = self.span_line(node.impl_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let (impl_generics, _, _) = node.generics.split_for_impl();
                let impl_gen_str = quote::quote!(#impl_generics).to_string();
                let mut signature = match impl_gen_str.is_empty() {
                    true => "impl".to_string(),
                    false => format!("impl {}", impl_gen_str),
                };

                let self_ty = &node.self_ty;
                let name = match &node.trait_ {
                    Some((_, trait_ref, _)) => {
                        signature.push_str(&format!(" {} for {}", quote::quote!(#trait_ref), quote::quote!(#self_ty)));
                        let trait_name = clean_path_name(trait_ref);
                        let type_name = clean_type_name(self_ty);
                        format!("impl_{}_for_{}", trait_name, type_name)
                    }
                    None => {
                        signature.push_str(&format!(" {}", quote::quote!(#self_ty)));
                        let type_name = clean_type_name(self_ty);
                        format!("impl_{}", type_name)
                    }
                };

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);
                let visibility = Visibility::Private;

                self.add_symbol(symbols, SymbolKind::Impl, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Mod(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.mod_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let signature = format!("mod {}", name);
                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Mod, name.clone(), signature, line, end_line, doc, visibility, attrs);

                // For inline modules, visit contents recursively
                if let Some((_, items)) = &node.content {
                    self.push_module(&name);
                    for item in items {
                        self.parse_item(item, symbols);
                    }
                    self.pop_module();
                }
            }
            Item::Const(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.const_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let ty = &node.ty;
                let expr = &node.expr;
                let signature = format!("const {}: {} = {}", name, quote::quote!(#ty), quote::quote!(#expr));

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Const, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Type(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.type_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let ty = &node.ty;
                let signature = format!("type {} = {}", name, quote::quote!(#ty));

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Type, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Static(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.static_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let ty = &node.ty;
                let mut_str = match &node.mutability {
                    syn::StaticMutability::Mut(_) => "mut ",
                    _ => "",
                };
                let signature = format!("static {}{}: {}", mut_str, name, quote::quote!(#ty));

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Variable, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Union(node) => {
                let name = node.ident.to_string();
                let visibility = Self::syn_vis_to_visibility(&node.vis);
                let line = self.span_line(node.union_token.span);
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));

                let generics = &node.generics;
                let mut signature = format!("union {}", name);
                if !generics.params.is_empty() {
                    let (_, ty_generics, _) = generics.split_for_impl();
                    signature.push_str(&quote::quote!(#ty_generics).to_string());
                }
                match node.fields.named.is_empty() {
                    true => signature.push_str(" {}"),
                    false => {
                        signature.push_str(" { ");
                        for (i, field) in node.fields.named.iter().enumerate() {
                            if i > 0 { signature.push_str(", "); }
                            let ty = &field.ty;
                            if let Some(ident) = &field.ident {
                                signature.push_str(&format!("{}: {}", ident, quote::quote!(#ty)));
                            }
                        }
                        signature.push_str(" }");
                    }
                }

                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);

                self.add_symbol(symbols, SymbolKind::Struct, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::Macro(node) => {
                let path = &node.mac.path;
                let (name, signature) = match &node.ident {
                    Some(ident) => {
                        let id_str = ident.to_string();
                        let sig = format!("macro_rules! {}", id_str);
                        (id_str, sig)
                    }
                    None => {
                        let path_str = quote::quote!(#path).to_string();
                        let name = match path_str.is_empty() {
                            true => "macro_rules!".to_string(),
                            false => path_str.clone(),
                        };
                        let sig = format!("{}!", path_str);
                        (name, sig)
                    }
                };

                let line = self.span_line(syn::spanned::Spanned::span(node));
                let end_line = self.span_line_end(syn::spanned::Spanned::span(node));
                let doc = self.extract_doc(&node.attrs);
                let attrs = self.extract_attrs(&node.attrs);
                let visibility = match node.attrs.iter().any(|a| a.path().is_ident("macro_export")) {
                    true => Visibility::Pub,
                    false => Visibility::Private,
                };

                self.add_symbol(symbols, SymbolKind::Macro, name, signature, line, end_line, doc, visibility, attrs);
            }
            Item::ForeignMod(_) => {}
            Item::ExternCrate(_) => {}
            Item::Use(_) => {}
            _ => {}
        }
    }
}

impl SourceParser for RustParser {
    fn parse_file(&mut self, path: &Path, content: &str) -> ParseResult {
        let mut symbols = Vec::new();
        let mut errors = Vec::new();

        let syntax = match syn::parse_file(content) {
            Ok(file) => file,
            Err(e) => {
                errors.push(format!("Parse error: {}", e));
                return ParseResult { symbols, errors };
            }
        };

        for item in &syntax.items {
            self.parse_item(item, &mut symbols);
        }

        // Set file paths for all symbols
        let rel_path = path.strip_prefix(std::env::current_dir().unwrap_or(PathBuf::from("."))).unwrap_or(path).to_string_lossy().to_string();
        for symbol in &mut symbols {
            symbol.file = rel_path.clone();
        }

        ParseResult { symbols, errors: Vec::new() }
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn language_id(&self) -> &'static str {
        "rust"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_static() {
        let code = r#"
/// Global configuration counter
pub static COUNTER: usize = 42;

static mut BUFFER: [u8; 1024] = [0; 1024];
"#;
        let mut parser = RustParser::new();
        let result = parser.parse_file(Path::new("test.rs"), code);
        assert_eq!(result.errors.len(), 0);

        let counter = result.symbols.iter().find(|s| s.name == "COUNTER").expect("COUNTER not found");
        assert_eq!(counter.kind, SymbolKind::Variable);
        assert_eq!(counter.visibility, Visibility::Pub);
        assert_eq!(counter.signature, "static COUNTER: usize");
        assert_eq!(counter.doc.as_deref(), Some("Global configuration counter"));

        let buffer = result.symbols.iter().find(|s| s.name == "BUFFER").expect("BUFFER not found");
        assert_eq!(buffer.kind, SymbolKind::Variable);
        assert_eq!(buffer.visibility, Visibility::Private);
        assert_eq!(buffer.signature, "static mut BUFFER: [u8 ; 1024]");
    }

    #[test]
    fn test_parse_rust_union() {
        let code = r#"
/// Untagged union representation
#[repr(C)]
pub union Value {
    pub i: i32,
    pub f: f32,
}
"#;
        let mut parser = RustParser::new();
        let result = parser.parse_file(Path::new("test.rs"), code);
        assert_eq!(result.errors.len(), 0);

        let union_sym = result.symbols.iter().find(|s| s.name == "Value").expect("Value not found");
        assert_eq!(union_sym.kind, SymbolKind::Struct);
        assert_eq!(union_sym.visibility, Visibility::Pub);
        assert_eq!(union_sym.signature, "union Value { i: i32, f: f32 }");
        assert_eq!(union_sym.doc.as_deref(), Some("Untagged union representation"));
    }

    #[test]
    fn test_parse_rust_macro() {
        let code = r#"
/// A logging macro
#[macro_export]
macro_rules! my_log {
    ($($arg:tt)*) => { println!($($arg)*); };
}
"#;
        let mut parser = RustParser::new();
        let result = parser.parse_file(Path::new("test.rs"), code);
        assert_eq!(result.errors.len(), 0);

        let mac = result.symbols.iter().find(|s| s.name == "my_log").expect("my_log not found");
        assert_eq!(mac.kind, SymbolKind::Macro);
        assert_eq!(mac.visibility, Visibility::Pub);
        assert_eq!(mac.signature, "macro_rules! my_log");
        assert_eq!(mac.doc.as_deref(), Some("A logging macro"));
    }

    #[test]
    fn test_parse_rust_impl_naming() {
        let code = r#"
struct MyStruct;
trait MyTrait {}

impl MyStruct {
    pub fn new() -> Self { MyStruct }
}

impl MyTrait for MyStruct {}

impl<T> std::fmt::Display for Vec<T> {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}

impl<T> Vec<T> {}
"#;
        let mut parser = RustParser::new();
        let result = parser.parse_file(Path::new("test.rs"), code);
        assert_eq!(result.errors.len(), 0);

        let impls: Vec<_> = result.symbols.iter().filter(|s| s.kind == SymbolKind::Impl).collect();
        assert_eq!(impls.len(), 4);

        assert_eq!(impls[0].name, "impl_MyStruct");
        assert_eq!(impls[0].signature, "impl MyStruct");

        assert_eq!(impls[1].name, "impl_MyTrait_for_MyStruct");
        assert_eq!(impls[1].signature, "impl MyTrait for MyStruct");

        assert_eq!(impls[2].name, "impl_Display_for_Vec");
        assert!(impls[2].signature.contains("std :: fmt :: Display for Vec"));

        assert_eq!(impls[3].name, "impl_Vec");
        assert!(impls[3].signature.contains("impl < T > Vec < T >") || impls[3].signature.contains("impl<T> Vec<T>"));

        for imp in &impls {
            assert!(!imp.name.ends_with('_'), "impl name {} ends with underscore", imp.name);
            assert!(!imp.name.contains('<'), "impl name {} contains <", imp.name);
            assert!(!imp.name.contains('>'), "impl name {} contains >", imp.name);
            assert!(!imp.name.contains(' '), "impl name {} contains space", imp.name);
        }
    }
}
