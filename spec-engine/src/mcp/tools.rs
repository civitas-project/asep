//! Tool definitions and execution dispatch for MCP.

use crate::adr::create_new_adr;
use crate::auditor::audit_project_docstrings;
use crate::diagram::render_plantuml_diagrams;
use crate::graph::builder::build_trace_graph;
use crate::scanner::scan_project_tags;
use srcdump::config::{Config as DumpConfig, OutputFormat, OutputMode};
use srcdump::formatter::Formatter;
use srcdump::walker::walk_files;
use srcidx::parser::ParserRegistry;
use std::path::{Path, PathBuf};

// @impl:FR-MCP-PROTO-02
// @impl:FR-MCP-PROTO-03

pub fn get_tools_list() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "asep_check",
                "description": "Deterministic SDLC validator: scans workspace for 1:1 traceability tags (@req <-> @impl <-> @test) and audits docstring contracts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": {
                            "type": "string",
                            "description": "Root path to scan (defaults to repository root)"
                        },
                        "strict": {
                            "type": "boolean",
                            "description": "Treat pending SHOULD requirements as errors"
                        },
                        "rigor": {
                            "type": "string",
                            "description": "Rigor tier: 'T0'..'T4' (default: T1 for code, T0 for docs-only)"
                        },
                        "deny_drafts": {
                            "type": "boolean",
                            "description": "Deny all @req:*:DRAFT tags (enforced on main production branch)"
                        }
                    }
                }
            },
            {
                "name": "spec_check",
                "description": "Deterministic SDLC validator (legacy alias for asep_check): scans workspace for 1:1 traceability tags (@req <-> @impl <-> @test) and audits docstring contracts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": {
                            "type": "string",
                            "description": "Root path to scan (defaults to repository root)"
                        },
                        "strict": {
                            "type": "boolean",
                            "description": "Treat pending SHOULD requirements as errors"
                        },
                        "rigor": {
                            "type": "string",
                            "description": "Rigor tier: 'T0'..'T4' (default: T1 for code, T0 for docs-only)"
                        },
                        "deny_drafts": {
                            "type": "boolean",
                            "description": "Deny all @req:*:DRAFT tags (enforced on main production branch)"
                        }
                    }
                }
            },
            {
                "name": "adr_new",
                "description": "Scaffolds a newly numbered Rich Architecture Decision Record (ADR).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Title of the architecture decision"
                        },
                        "dir": {
                            "type": "string",
                            "description": "Target repository path"
                        }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "render_diagrams",
                "description": "Compiles PlantUML diagrams (.puml) into vector SVG assets.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": {
                            "type": "string",
                            "description": "Target directory to scan for diagrams"
                        }
                    }
                }
            },
            {
                "name": "srcidx",
                "description": "Semantic source indexer: extracts symbols (functions, structs, traits, etc.) from source files and emits NDJSON with self-describing header. Supports Rust, TypeScript, Python, Dart, Go, PHP, Java, Typst.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": {
                            "type": "string",
                            "description": "Root directory to index (defaults to repository root)"
                        },
                        "include_ext": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File extensions to include (e.g., [\"rs\", \"py\", \"ts\"])"
                        },
                        "exclude_ext": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File extensions to exclude"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["full", "symbols-only", "summary", "signatures"],
                            "description": "Output mode: 'full' (default), 'symbols-only', 'summary', 'signatures'"
                        },
                        "strip_comments": {
                            "type": "boolean",
                            "description": "Strip comments from source before indexing (default: true)"
                        },
                        "include_symbols": {
                            "type": "boolean",
                            "description": "Include symbol annotations in output (default: true)"
                        },
                        "gitignore": {
                            "type": "boolean",
                            "description": "Respect .gitignore files (default: true)"
                        },
                        "summary_threshold": {
                            "type": "integer",
                            "description": "Line threshold for summary mode (default: 1000)"
                        },
                        "follow_symlinks": {
                            "type": "boolean",
                            "description": "Follow symlinks with safety checks (default: true)"
                        }
                    }
                }
            },
            {
                "name": "srcdump",
                "description": "Source code dumper for LLM consumption: streams source files as token-efficient NDJSON with configurable formatting modes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dir": {
                            "type": "string",
                            "description": "Root directory to dump (defaults to repository root)"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["full", "symbols-only", "summary", "signatures"],
                            "description": "Output mode: 'full' (default), 'symbols-only', 'summary', 'signatures'"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["ndjson", "json", "text"],
                            "description": "Output format (default: ndjson)"
                        },
                        "tokenizer": {
                            "type": "string",
                            "enum": ["cl100k_base", "o200k_base", "p50k_base", "r50k_base"],
                            "description": "Tokenizer encoding for token estimation (default: cl100k_base)"
                        },
                        "strip_comments": {
                            "type": "boolean",
                            "description": "Strip all comments (default: true)"
                        },
                        "strip_blank_lines": {
                            "type": "boolean",
                            "description": "Collapse multiple blank lines to single (default: true)"
                        },
                        "max_lines": {
                            "type": "integer",
                            "description": "Maximum lines per file (truncate longer files)"
                        },
                        "max_line_length": {
                            "type": "integer",
                            "description": "Maximum line length (truncate longer lines)"
                        },
                        "summary_threshold": {
                            "type": "integer",
                            "description": "Threshold for summary mode (files larger than this use summary, default: 1000)"
                        },
                        "include_symbols": {
                            "type": "boolean",
                            "description": "Include symbol annotations from srcidx (default: false)"
                        },
                        "no_content": {
                            "type": "boolean",
                            "description": "Output only metadata, no content (default: false)"
                        },
                        "estimate_tokens": {
                            "type": "boolean",
                            "description": "Show token estimates in output (default: false)"
                        },
                        "gitignore": {
                            "type": "boolean",
                            "description": "Respect .gitignore files (default: true)"
                        },
                        "follow_symlinks": {
                            "type": "boolean",
                            "description": "Follow symlinks with safety checks (default: true)"
                        }
                    }
                }
            }
        ]
    })
}

pub fn execute_tool(
    name: &str,
    arguments: &serde_json::Value,
    default_dir: &Path,
) -> Result<serde_json::Value, String> {
    match name {
        "asep_check" | "spec_check" => {
            let target = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let strict = arguments
                .get("strict")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let rigor = arguments
                .get("rigor")
                .and_then(|v| v.as_str())
                .unwrap_or("T1");

            let deny_drafts = arguments
                .get("deny_drafts")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let is_relaxed = rigor == "T0";

            let tags = scan_project_tags(&target)
                .map_err(|e| format!("Failed to scan project tags: {e}"))?;

            let report = build_trace_graph(&tags);
            let violations = audit_project_docstrings(&target)
                .map_err(|e| format!("Failed to audit docstrings: {e}"))?;

            let has_must_failure = !report.is_closed;
            let has_doc_contract_failure = !is_relaxed && !violations.is_empty();
            let has_strict_failure = strict && !report.should_pending.is_empty();
            let has_deny_drafts_failure = deny_drafts && report.draft_total > 0;
            let passed = !has_must_failure && !has_doc_contract_failure && !has_strict_failure && !has_deny_drafts_failure;

            let mut summary = String::new();
            summary.push_str(&format!(
                "Traceability Gate: {} (rigor: {})\n",
                if passed { "PASSED" } else { "FAILED" },
                if is_relaxed { "T0" } else { "T1+" }
            ));
            summary.push_str(&format!(
                "Requirements: {} total | {} closed | {} broken MUST | {} pending SHOULD | {} DRAFT\n",
                report.total_requirements,
                report.must_satisfied,
                report.must_broken.len(),
                report.should_pending.len(),
                report.draft_total
            ));
            summary.push_str(&format!(
                "Docstring Contract Violations: {}\n",
                violations.len()
            ));

            if !report.must_broken.is_empty() {
                summary.push_str("\nBroken MUST Requirements:\n");
                for broken in &report.must_broken {
                    summary.push_str(&format!(
                        "  - {} ({}:{}) missing tags\n",
                        broken.id,
                        broken.file_path.display(),
                        broken.line_number
                    ));
                }
            }

            if !violations.is_empty() {
                summary.push_str("\nDocstring Contract Violations:\n");
                for v in &violations {
                    summary.push_str(&format!(
                        "  - {}:{}: {}\n",
                        v.file_path.display(),
                        v.line_number,
                        v.message
                    ));
                }
            }

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": summary
                    }
                ],
                "isError": !passed
            }))
        }

        "adr_new" => {
            let title = arguments
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'title'".to_string())?;

            let target = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let path =
                create_new_adr(&target, title).map_err(|e| format!("Failed to create ADR: {e}"))?;

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Successfully created ADR at {}", path.display())
                    }
                ],
                "isError": false
            }))
        }

        "render_diagrams" => {
            let target = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let count = render_plantuml_diagrams(&target)
                .map_err(|e| format!("Failed to render diagrams: {e}"))?;

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Rendered {count} PlantUML diagram(s) into SVG")
                    }
                ],
                "isError": false
            }))
        }

        "srcidx" => {
            let target = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let include_ext: Vec<String> = arguments
                .get("include_ext")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let exclude_ext: Vec<String> = arguments
                .get("exclude_ext")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let mode = arguments
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("full");

            let strip_comments = arguments
                .get("strip_comments")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let include_symbols = arguments
                .get("include_symbols")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let gitignore = arguments
                .get("gitignore")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let summary_threshold = arguments
                .get("summary_threshold")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as usize;

            let follow_symlinks = arguments
                .get("follow_symlinks")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let config = DumpConfig {
                root: target,
                mode: match mode {
                    "full" => OutputMode::Full,
                    "symbols-only" => OutputMode::SymbolsOnly,
                    "signatures" => OutputMode::Signatures,
                    "summary" => OutputMode::Summary,
                    _ => OutputMode::Full,
                },
                format: OutputFormat::Ndjson,
                use_default_includes: include_ext.is_empty(),
                include_ext,
                exclude_ext,
                strip_comments,
                include_symbols,
                gitignore,
                summary_threshold,
                follow_symlinks,
                ..Default::default()
            };

            let parsers = ParserRegistry::default();
            let mut formatter = Formatter::new(config, parsers)
                .map_err(|e| format!("Failed to create formatter: {e}"))?;

            let files = walk_files(formatter.config())
                .map_err(|e| format!("Failed to walk files: {e}"))?;

            let mut output = Vec::new();
            formatter.format_files(files, &mut output)
                .map_err(|e| format!("Failed to format files: {e}"))?;

            let output_str = String::from_utf8(output)
                .map_err(|e| format!("Invalid UTF-8 in output: {e}"))?;

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": output_str
                    }
                ],
                "isError": false
            }))
        }

        "srcdump" => {
            let target = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let mode = arguments
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("full");

            let format = arguments
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("ndjson");

            let tokenizer = arguments
                .get("tokenizer")
                .and_then(|v| v.as_str())
                .unwrap_or("cl100k_base");

            let strip_comments = arguments
                .get("strip_comments")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let strip_blank_lines = arguments
                .get("strip_blank_lines")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let max_lines = arguments
                .get("max_lines")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let max_line_length = arguments
                .get("max_line_length")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            let summary_threshold = arguments
                .get("summary_threshold")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as usize;

            let include_symbols = arguments
                .get("include_symbols")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let no_content = arguments
                .get("no_content")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let estimate_tokens = arguments
                .get("estimate_tokens")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let gitignore = arguments
                .get("gitignore")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let follow_symlinks = arguments
                .get("follow_symlinks")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let config = DumpConfig {
                root: target,
                mode: match mode {
                    "full" => OutputMode::Full,
                    "symbols-only" => OutputMode::SymbolsOnly,
                    "signatures" => OutputMode::Signatures,
                    "summary" => OutputMode::Summary,
                    _ => OutputMode::Full,
                },
                format: match format {
                    "ndjson" => OutputFormat::Ndjson,
                    "json" => OutputFormat::Json,
                    "text" => OutputFormat::Text,
                    _ => OutputFormat::Ndjson,
                },
                tokenizer: tokenizer.to_string(),
                strip_comments,
                strip_blank_lines,
                max_lines,
                max_line_length,
                summary_threshold,
                include_symbols,
                no_content,
                estimate_tokens,
                gitignore,
                follow_symlinks,
                ..Default::default()
            };

            let parsers = ParserRegistry::default();
            let mut formatter = Formatter::new(config, parsers)
                .map_err(|e| format!("Failed to create formatter: {e}"))?;

            let files = walk_files(formatter.config())
                .map_err(|e| format!("Failed to walk files: {e}"))?;

            let mut output = Vec::new();
            formatter.format_files(files, &mut output)
                .map_err(|e| format!("Failed to format files: {e}"))?;

            let output_str = String::from_utf8(output)
                .map_err(|e| format!("Invalid UTF-8 in output: {e}"))?;

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": output_str
                    }
                ],
                "isError": false
            }))
        }

        other => Err(format!("Unknown tool: {other}")),
    }
}
