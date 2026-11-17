//! Embedded JSON Schema generator for self-describing output.

use serde_json::{json, Value};

/// Generate the full JSON Schema for the semantic index format.
pub fn generate_json_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://aethelgard.dev/schemas/semantic-index.v1.json",
        "title": "Semantic Index v1",
        "description": "Self-describing semantic index of Rust source code symbols",
        "type": "object",
        "required": ["header", "symbols"],
        "properties": {
            "header": {
                "type": "object",
                "required": ["version", "schema", "root", "symbols", "files", "skipped", "generated", "fields"],
                "properties": {
                    "version": { "type": "integer", "const": 1 },
                    "schema": { "type": "string", "const": "semantic-index.v1" },
                    "root": { "type": "string" },
                    "symbols": { "type": "integer", "minimum": 0 },
                    "files": { "type": "integer", "minimum": 0 },
                    "skipped": { "type": "integer", "minimum": 0 },
                    "generated": { "type": "string", "format": "date-time" },
                    "fields": {
                        "type": "object",
                        "description": "Embedded field schemas for self-describing output",
                        "properties": {
                            "kind": { "$ref": "#/$defs/fieldSchema" },
                            "name": { "$ref": "#/$defs/fieldSchema" },
                            "qualified_name": { "$ref": "#/$defs/fieldSchema" },
                            "signature": { "$ref": "#/$defs/fieldSchema" },
                            "file": { "$ref": "#/$defs/fieldSchema" },
                            "line": { "$ref": "#/$defs/fieldSchema" },
                            "end_line": { "$ref": "#/$defs/fieldSchema" },
                            "doc": { "$ref": "#/$defs/fieldSchema" },
                            "tags": { "$ref": "#/$defs/fieldSchema" },
                            "visibility": { "$ref": "#/$defs/fieldSchema" },
                            "attrs": { "$ref": "#/$defs/fieldSchema" }
                        },
                        "required": ["kind", "name", "qualified_name", "signature", "file", "line", "end_line", "doc", "tags", "visibility", "attrs"]
                    }
                }
            },
            "symbols": {
                "type": "array",
                "items": { "$ref": "#/$defs/symbol" }
            }
        },
        "$defs": {
            "fieldSchema": {
                "type": "object",
                "required": ["type", "description"],
                "properties": {
                    "type": { "type": "string" },
                    "description": { "type": "string" },
                    "enum_values": { "type": "array", "items": { "type": "string" } }
                }
            },
            "symbol": {
                "type": "object",
                "required": ["kind", "name", "qualified_name", "signature", "file", "line", "end_line", "visibility"],
                "properties": {
                    "kind": { "type": "string", "enum": ["fn", "struct", "enum", "trait", "const", "type", "class", "interface", "variable", "import", "export", "decorator", "impl", "mod", "macro", "mixin", "extension", "typedef", "other"] },
                    "name": { "type": "string" },
                    "qualified_name": { "type": "string" },
                    "signature": { "type": "string" },
                    "file": { "type": "string" },
                    "line": { "type": "integer", "minimum": 1 },
                    "end_line": { "type": "integer", "minimum": 1 },
                    "doc": { "type": ["string", "null"] },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "visibility": { "type": "string", "enum": ["pub", "pub_crate", "pub_super", "private"] },
                    "attrs": { "type": "array", "items": { "type": "string" } }
                }
            }
        }
    })
}

/// Get the schema as a pretty JSON string.
pub fn schema_json_string() -> String {
    serde_json::to_string_pretty(&generate_json_schema()).expect("valid JSON schema")
}