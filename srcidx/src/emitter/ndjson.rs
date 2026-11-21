//! NDJSON emitter with self-describing header line.

use crate::emitter::IndexEmitter;
use crate::index::SymbolIndex;
use anyhow::Result;
use serde_json::json;
use std::io::{BufWriter, Write};

/// NDJSON emitter: each line is a valid JSON object.
/// Line 1: header with embedded schema
/// Lines 2+: symbol objects
pub struct NdjsonEmitter {
    include_schema: bool,
}

impl NdjsonEmitter {
    pub fn new() -> Self {
        Self { include_schema: true }
    }

    pub fn without_schema() -> Self {
        Self { include_schema: false }
    }
}

impl Default for NdjsonEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexEmitter for NdjsonEmitter {
    fn emit(&self, index: &SymbolIndex, writer: &mut dyn Write) -> Result<()> {
        let mut buf = BufWriter::new(writer);

        // Line 1: Header with embedded field schemas
        let header_json = match self.include_schema {
            true => {
                // Include full field schemas in header for self-describing output
                json!({
                    "version": index.header.version,
                    "schema": index.header.schema,
                    "root": index.header.root,
                    "symbols": index.header.symbols,
                    "files": index.header.files,
                    "skipped": index.header.skipped,
                    "generated": index.header.generated,
                    "fields": index.header.fields,
                })
            }
            false => {
                json!({
                    "version": index.header.version,
                    "schema": index.header.schema,
                    "root": index.header.root,
                    "symbols": index.header.symbols,
                    "files": index.header.files,
                    "skipped": index.header.skipped,
                    "generated": index.header.generated,
                })
            }
        };
        writeln!(buf, "{}", serde_json::to_string(&header_json)?)?;

        // Lines 2+: Symbols
        for symbol in &index.symbols {
            let sym_json = json!({
                "kind": symbol.kind,
                "name": symbol.name,
                "qualified_name": symbol.qualified_name,
                "signature": symbol.signature,
                "file": symbol.file,
                "line": symbol.line,
                "end_line": symbol.end_line,
                "doc": symbol.doc,
                "tags": symbol.tags,
                "visibility": symbol.visibility,
                "attrs": symbol.attrs,
            });
            writeln!(buf, "{}", serde_json::to_string(&sym_json)?)?;
        }

        buf.flush()?;
        Ok(())
    }

    fn format_name(&self) -> &'static str {
        "ndjson"
    }

    fn extension(&self) -> &'static str {
        "ndjson"
    }
}

/// Streaming NDJSON reader for LLM consumption.
/// Reads header first, then yields symbols one at a time.
pub struct NdjsonReader<R: std::io::BufRead> {
    reader: R,
    header_read: bool,
    header: Option<crate::index::IndexHeader>,
}

impl<R: std::io::BufRead> NdjsonReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            header_read: false,
            header: None,
        }
    }

    /// Read and parse the header line.
    pub fn read_header(&mut self) -> Result<Option<crate::index::IndexHeader>> {
        if self.header_read {
            return Ok(self.header.clone());
        }

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }

        let header: crate::index::IndexHeader = serde_json::from_str(line.trim())?;
        self.header = Some(header.clone());
        self.header_read = true;
        Ok(Some(header))
    }

    /// Read the next symbol.
    pub fn next_symbol(&mut self) -> Result<Option<crate::index::Symbol>> {
        if !self.header_read {
            self.read_header()?;
        }

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }

        let symbol: crate::index::Symbol = serde_json::from_str(line.trim())?;
        Ok(Some(symbol))
    }

    /// Iterate over all symbols.
    pub fn symbols(&mut self) -> impl Iterator<Item = Result<crate::index::Symbol>> + '_ {
        std::iter::from_fn(move || self.next_symbol().transpose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexHeader, Symbol, SymbolIndex, SymbolKind, Visibility};
    use std::io::Cursor;

    #[test]
    fn test_ndjson_emit_and_read() {
        let mut index = SymbolIndex::new("/test".to_string());
        index.header.files = 1;
        index.header = index.header.with_fields(crate::index::generate_index_fields());
        index.add_symbol(Symbol {
            kind: SymbolKind::Fn,
            name: "test_fn".to_string(),
            qualified_name: "crate::test_fn".to_string(),
            signature: "pub fn test_fn() -> i32".to_string(),
            file: "src/lib.rs".to_string(),
            line: 10,
            end_line: 15,
            doc: Some("Test function".to_string()),
            tags: vec!["\\@impl:FR-TEST-01".to_string()],
            visibility: Visibility::Pub,
            attrs: vec![],
        });

        let mut output = Vec::new();
        let emitter = NdjsonEmitter::new();
        emitter.emit(&index, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 symbol

        // Verify header
        let header: IndexHeader = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.schema, "semantic-index.v1");
        assert!(header.fields.as_ref().unwrap().kind.enum_values.is_some());

        // Verify symbol
        let symbol: Symbol = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(symbol.name, "test_fn");
        assert_eq!(symbol.tags, vec!["\\@impl:FR-TEST-01"]);
    }

    #[test]
    fn test_streaming_reader() {
        let mut index = SymbolIndex::new("/test".to_string());
        index.add_symbol(Symbol {
            kind: SymbolKind::Fn,
            name: "fn1".to_string(),
            qualified_name: "crate::fn1".to_string(),
            signature: "fn fn1()".to_string(),
            file: "a.rs".to_string(),
            line: 1,
            end_line: 5,
            doc: None,
            tags: vec![],
            visibility: Visibility::Pub,
            attrs: vec![],
        });
        index.add_symbol(Symbol {
            kind: SymbolKind::Struct,
            name: "MyStruct".to_string(),
            qualified_name: "crate::MyStruct".to_string(),
            signature: "struct MyStruct".to_string(),
            file: "b.rs".to_string(),
            line: 10,
            end_line: 20,
            doc: None,
            tags: vec![],
            visibility: Visibility::Pub,
            attrs: vec![],
        });

        let mut output = Vec::new();
        let emitter = NdjsonEmitter::new();
        emitter.emit(&index, &mut output).unwrap();

        let cursor = Cursor::new(output);
        let mut reader = NdjsonReader::new(std::io::BufReader::new(cursor));

        let header = reader.read_header().unwrap().unwrap();
        assert_eq!(header.symbols, 2);

        let sym1 = reader.next_symbol().unwrap().unwrap();
        assert_eq!(sym1.name, "fn1");

        let sym2 = reader.next_symbol().unwrap().unwrap();
        assert_eq!(sym2.name, "MyStruct");

        assert!(reader.next_symbol().unwrap().is_none());
    }
}