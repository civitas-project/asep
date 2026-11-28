// @impl:FR-MCP-03
//! Tool definitions and execution dispatch for MCP.
// @impl:FR-CAP-02

use crate::acceptance_scoped::run_scoped_acceptance;
use crate::adr::create_new_adr;
use crate::affected::affected;
use crate::agent_protocol::{Kind, MessageStore, Role};

use crate::config::settings::{load_config, load_config_checked};
use crate::context::compile_context;
use crate::diagram::render_plantuml_diagrams;
use crate::observability::read_trace_events;
use crate::config::budget::ResourceBudget;
use crate::scanner::scan_project_tags;
use serde_json::json;
use srcdump::config::{Config as DumpConfig, OutputFormat, OutputMode};
use srcdump::formatter::Formatter;
use srcdump::walker::walk_files;
use srcidx::parser::ParserRegistry;
use std::fs;
use std::path::{Path, PathBuf};

// @impl:FR-MCP-PROTO-02-B @impl:FR-MCP-02
// @impl:FR-MCP-PROTO-03-A

pub fn get_tools_list() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "asep_check",
                "description": "Validates @req/@impl/@test tags; audits docstring contracts.",
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
                "description": "Legacy alias for asep_check; validates traceability tags.",
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
                "description": "Creates a numbered Architecture Decision Record (ADR).",
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
                "description": "Source symbol indexer emitting NDJSON (Rust, TS, Go, etc.).",
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
                "description": "Dumps source as token-efficient NDJSON for LLM consumption.",
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
            },
            {
                "name": "asep_accept_scoped",
                "description": "Runs scoped non-root acceptance via the trusted runner.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "commit": {
                            "type": "string",
                            "description": "Commit to accept (40 lowercase hex characters)"
                        },
                        "dir": {
                            "type": "string",
                            "description": "Repository root (defaults to current directory)"
                        }
                    },
                    "required": ["commit"]
                }
            },
            {
                "name": "asep_trace_report",
                "description": "Report a recorded agent trace as NDJSON summary",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "trace_id": {
                            "type": "string",
                            "description": "Trace id: [A-Za-z0-9._-]"
                        },
                        "dir": {
                            "type": "string",
                            "description": "Repository root (defaults to current directory)"
                        }
                    },
                    "required": ["trace_id"]
                }
            },
            {
                "name": "asep_agent_send",
                "description": "Sends an agent message envelope; returns id and path only.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "run_dir": {
                            "type": "string",
                            "description": "Run directory (required; ASEP_RUN_DIR env fallback)"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Caller role: leader, executor, judge, explorer"
                        },
                        "to": {
                            "type": "string",
                            "description": "Recipient role: leader, executor, judge, explorer"
                        },
                        "kind": {
                            "type": "string",
                            "description": "Message kind: TASK_DISPATCH, RULING, TASK_REPORT, VERDICT, MAP_REPORT, CUSTOM"
                        },
                        "body": {
                            "type": "object",
                            "description": "Message body payload (kept in a sidecar file, not in the response)"
                        },
                        "ref": {
                            "type": "string",
                            "description": "Optional ref to a file under the run dir"
                        },
                        "reply_to": {
                            "type": "string",
                            "description": "Optional id of the message this is a reply to"
                        },
                        "required_capabilities": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Required capabilities the recipient must have"
                        },
                        "media": {
                            "type": "string",
                            "description": "Media type of the message body"
                        },
                        "domain": {
                            "type": "string",
                            "description": "Domain of the message"
                        },
                        "tool_filter": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Allowed tools for the delegated task"
                        },
                        "delegation_threshold": {
                            "type": "integer",
                            "description": "Delegation threshold"
                        }
                    },
                    "required": ["run_dir", "agent", "to", "kind", "body"]
                }
            },
            {
                "name": "asep_agent_recv",
                "description": "Lists message ids/paths for a role; only 'sent' messages.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "run_dir": {
                            "type": "string",
                            "description": "Run directory (required; ASEP_RUN_DIR env fallback)"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Recipient role: leader, executor, judge, explorer"
                        }
                    },
                    "required": ["run_dir", "agent"]
                }
            },
            {
                "name": "asep_agent_read",
                "description": "Return the body payload of a message by id.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "run_dir": {
                            "type": "string",
                            "description": "Run directory (required; ASEP_RUN_DIR env fallback)"
                        },
                        "id": {
                            "type": "string",
                            "description": "Message id"
                        }
                    },
                    "required": ["run_dir", "id"]
                }
            },
            {
                "name": "asep_agent_ack",
                "description": "Acks a message by id with status accepted/rejected/done.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "run_dir": {
                            "type": "string",
                            "description": "Run directory (required; ASEP_RUN_DIR env fallback)"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Caller role: leader, executor, judge, explorer"
                        },
                        "id": {
                            "type": "string",
                            "description": "Message id"
                        },
                        "status": {
                            "type": "string",
                            "description": "Ack status: accepted, rejected, done"
                        }
                    },
                    "required": ["run_dir", "agent", "id", "status"]
                }
            },
            // @impl:FR-MCP-IMPACT-01
            {
                "name": "asep_affected",
                "description": "Lists impls, requirements, tests affected by changed files",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Changed file paths to impact-analyze"
                        },
                        "root": {
                            "type": "string",
                            "description": "Repository root (defaults to current directory)"
                        }
                    },
                    "required": ["paths"]
                }
            },
            {
                "name": "asep_context",
                "description": "Assembles context bundle from changed file paths",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Changed file paths to build context for"
                        },
                        "root": {
                            "type": "string",
                            "description": "Repository root (defaults to current directory)"
                        },
                        "max_tokens": {
                            "type": "integer",
                            "description": "Maximum estimated tokens for excerpts"
                        },
                        "explicit": {
                            "type": "boolean",
                            "description": "Treat paths as explicit files; skip impact analysis"
                        }
                    },
                    "required": ["paths"]
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

            let outcome = crate::ci::check_core(&target, &rigor, strict)
                .map_err(|e| format!("Failed to verify project: {e}"))?;
            let mut summary = String::new();
            let gate = match outcome.passed {
                true => "PASSED",
                false => "FAILED",
            };
            let rigor_label = match rigor {
                "T0" => "T0",
                _ => "T1+",
            };
            summary.push_str(&format!(
                "Traceability Gate: {} (rigor: {})\n",
                gate, rigor_label
            ));
            summary.push_str(&format!(
                "Verification: {:?} | structural={} | skipped={:?}\n",
                outcome.status, outcome.structural, outcome.skipped
            ));
            summary.push_str(&format!(
                "Requirements: {} total | {} closed | {} broken MUST | {} pending SHOULD | {} DRAFT\n",
                outcome.must_broken + outcome.structural as usize,
                outcome.structural as usize,
                outcome.must_broken,
                outcome.should_pending,
                outcome.draft_total
            ));
            summary.push_str(&format!(
                "Docstring Contract Violations: {}\n",
                outcome.doc_contract_violations
            ));

            Ok(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": summary
                    }
                ],
                "isError": !outcome.passed
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
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let exclude_ext: Vec<String> = arguments
                .get("exclude_ext")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
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

            let files =
                walk_files(formatter.config()).map_err(|e| format!("Failed to walk files: {e}"))?;

            let mut output = Vec::new();
            formatter
                .format_files(files, &mut output)
                .map_err(|e| format!("Failed to format files: {e}"))?;

            let output_str =
                String::from_utf8(output).map_err(|e| format!("Invalid UTF-8 in output: {e}"))?;

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

            let files =
                walk_files(formatter.config()).map_err(|e| format!("Failed to walk files: {e}"))?;

            let mut output = Vec::new();
            formatter
                .format_files(files, &mut output)
                .map_err(|e| format!("Failed to format files: {e}"))?;

            let output_str =
                String::from_utf8(output).map_err(|e| format!("Invalid UTF-8 in output: {e}"))?;

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

        "asep_agent_send" => {
            let run_dir = mcp_run_dir(arguments)?;
            let from = mcp_role(mcp_require_str(arguments, "agent")?)?;
            let to = mcp_role(mcp_require_str(arguments, "to")?)?;
            let kind = mcp_kind(arguments, "kind")?;
            let mut body = arguments
                .get("body")
                .ok_or_else(|| "Missing required parameter 'body'".to_string())?
                .clone();
            let ref_path = arguments.get("ref").and_then(|v| v.as_str()).map(Path::new);
            let reply_to = arguments.get("reply_to").and_then(|v| v.as_str());

            let required_capabilities =
                mcp_opt_array_of_strings(arguments, "required_capabilities")?;
            let media = mcp_opt_string(arguments, "media")?;
            let domain = mcp_opt_string(arguments, "domain")?;
            let tool_filter = mcp_opt_array_of_strings(arguments, "tool_filter")?;

            match body.as_object_mut() {
                Some(obj) => {
                    if let Some(caps) = required_capabilities.as_ref() {
                        obj.entry("required_capabilities").or_insert(json!(caps));
                    }
                    if let Some(m) = media.as_ref() {
                        obj.entry("media").or_insert(json!(m));
                    }
                    if let Some(d) = domain {
                        obj.entry("domain").or_insert(json!(d));
                    }
                    if let Some(f) = tool_filter {
                        obj.entry("tool_filter").or_insert(json!(f));
                    }
                }
                None => {}
            }

            // Fail-closed capability/media guard before any message is stored.
            let guard_reason = match (
                required_capabilities.as_deref(),
                media.as_deref().filter(|m| !m.is_empty()),
            ) {
                (None, None) => None,
                (caps, med) => match load_config_checked(default_dir) {
                    Ok(config) => {
                        let names: Vec<&str> = caps
                            .map(|vec| vec.iter().map(String::as_str).collect())
                            .unwrap_or_default();
                        config.capabilities.covers(&names, med).err()
                    }
                    Err(e) => Some(format!("BLOCKED: invalid capability configuration: {e}")),
                }
            };
            match guard_reason {
                Some(reason) => {
                    return Ok(serde_json::json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::json!({"blocked": true, "reason": reason})
                                    .to_string()
                            }
                        ],
                        "isError": true
                    }));
                }
                None => {}
            }

            let store = MessageStore::new(&run_dir);
            let envelope = store.send(from, to, kind, body, ref_path, reply_to)?;
            mcp_text(json!({
                "id": envelope.id,
                "path": mcp_msg_path(&envelope.id),
            }))
        }

        "asep_agent_recv" => {
            let run_dir = mcp_run_dir(arguments)?;
            let agent = mcp_role(mcp_require_str(arguments, "agent")?)?;
            let store = MessageStore::new(&run_dir);
            let list: Vec<serde_json::Value> = store
                .recv(agent)
                .iter()
                .map(|envelope| {
                    json!({
                        "id": envelope.id,
                        "kind": envelope.kind,
                        "from": envelope.from.as_str(),
                        "path": mcp_msg_path(&envelope.id),
                    })
                })
                .collect();
            mcp_text(json!(list))
        }

        "asep_agent_read" => {
            let run_dir = mcp_run_dir(arguments)?;
            let id = mcp_require_str(arguments, "id")?;
            let store = MessageStore::new(&run_dir);
            let body = store.read(id)?;
            mcp_text(body)
        }

        "asep_agent_ack" => {
            let run_dir = mcp_run_dir(arguments)?;
            let agent = mcp_role(mcp_require_str(arguments, "agent")?)?;
            let id = mcp_require_str(arguments, "id")?;
            let status = mcp_require_str(arguments, "status")?;
            let store = MessageStore::new(&run_dir);
            store.ack(agent, id, status)?;
            mcp_text(json!({
                "id": id,
                "path": mcp_msg_path(id),
            }))
        }

        "asep_accept_scoped" => {
            let commit = mcp_require_str(arguments, "commit")?;
            if !is_scoped_commit(commit) {
                return Ok(serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!(
                                "invalid commit '{commit}': expected a 40-character lowercase hex string"
                            )
                        }
                    ],
                    "isError": true
                }));
            }

            let dir = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            match run_scoped_acceptance(&dir, commit) {
                Ok(evidence_dir) => Ok(serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!(
                                "Scoped acceptance evidence at {}",
                                evidence_dir.display()
                            )
                        }
                    ],
                    "isError": false
                })),
                Err(diagnostic) => Ok(serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Scoped acceptance failed: {diagnostic}")
                        }
                    ],
                    "isError": true
                })),
            }
        }

        "asep_trace_report" => {
            let trace_id = mcp_require_str(arguments, "trace_id")?;
            let valid_id = !trace_id.is_empty()
                && trace_id.bytes().all(
                    |b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'),
                );
            if !valid_id {
                return Ok(serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!(
                                "invalid trace_id '{trace_id}': must be non-empty [A-Za-z0-9._-]"
                            )
                        }
                    ],
                    "isError": true
                }));
            }

            let dir = arguments
                .get("dir")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            match build_trace_summary(&dir, trace_id) {
                Ok(summary) => mcp_text(summary),
                Err(diagnostic) => Ok(serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": diagnostic
                        }
                    ],
                    "isError": true
                })),
            }
        }

        // @impl:FR-MCP-IMPACT-02
        "asep_affected" => {
            let paths: Vec<String> = arguments
                .get("paths")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Missing required parameter 'paths'".to_string())?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| "invalid 'paths': expected array of strings".to_string())
                })
                .collect::<Result<Vec<String>, String>>()?;

            if paths.is_empty() {
                return Err("'paths' must not be empty".to_string());
            }

            let root = arguments
                .get("root")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let changed: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            let report = affected(&root, &changed)
                .map_err(|e| format!("Failed to compute affected: {e}"))?;

            let value = serde_json::to_value(&report)
                .map_err(|e| format!("Failed to serialize affected report: {e}"))?;
            mcp_text(value)
        }

        // @impl:FR-CTX-03
        "asep_context" => {
            let paths: Vec<String> = arguments
                .get("paths")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Missing required parameter 'paths'".to_string())?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| "invalid 'paths': expected array of strings".to_string())
                })
                .collect::<Result<Vec<String>, String>>()?;

            if paths.is_empty() {
                return Err("'paths' must not be empty".to_string());
            }

            let root = arguments
                .get("root")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_dir.to_path_buf());

            let max_tokens = arguments.get("max_tokens").and_then(|v| v.as_u64());
            let budget = ResourceBudget {
                max_tokens,
                ..Default::default()
            };

            let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            let explicit = arguments.get("explicit").and_then(|v| v.as_bool()).unwrap_or(false);
            let bundle = match explicit {
                true => crate::context::compile_context_files(
                    &root,
                    &path_bufs,
                    format!("Explicit context for {} files", path_bufs.len()),
                    budget,
                    Vec::new(),
                ),
                false => compile_context(&root, &path_bufs, budget),
            }
            .map_err(|e| format!("Failed to compile context: {e}"))?;
            let value = serde_json::to_value(&bundle)
                .map_err(|e| format!("Failed to serialize context bundle: {e}"))?;
            mcp_text(value)
        }

        other => Err(format!("Unknown tool: {other}")),
    }
}

/// Commit accepted by scoped acceptance must be exactly 40 lowercase hex chars.
fn is_scoped_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

// Read-only trace report: groups parsed NDJSON lines by task_id/event and
// adds reconciliation status from .spec-engine/verification.json.
fn build_trace_summary(dir: &Path, trace_id: &str) -> Result<serde_json::Value, String> {
    let config = load_config(dir);
    let trace_dir = dir.join(&config.telemetry.dir);
    let events = read_trace_events(&trace_dir, trace_id)?;

    let mut events_values: Vec<serde_json::Value> = Vec::new();
    let mut by_task: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for event in &events {
        events_values
            .push(serde_json::to_value(event).map_err(|e| e.to_string())?);
        let group = by_task
            .entry(event.task_id.clone())
            .or_insert_with(|| serde_json::json!({}));
        let counts = group.as_object_mut().expect("by_task group is object");
        let count = counts
            .get(&event.event)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        counts.insert(event.event.clone(), json!(count + 1));
    }

    let trace_found = !events.is_empty();

    let verification_path = dir.join(".spec-engine").join("verification.json");
    let reconciliation = fs::read_to_string(&verification_path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|value| {
            value
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(serde_json::json!({
        "trace_id": trace_id,
        "trace_found": trace_found,
        "events": events_values,
        "by_task": by_task,
        "reconciliation": reconciliation,
    }))
}

/// Run directory for agent tools: explicit `run_dir` argument, `ASEP_RUN_DIR` env fallback only.
fn mcp_run_dir(arguments: &serde_json::Value) -> Result<PathBuf, String> {
    arguments
        .get("run_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| std::env::var("ASEP_RUN_DIR").ok().map(PathBuf::from))
        .ok_or_else(|| "Missing required parameter 'run_dir'".to_string())
}

fn mcp_require_str<'a>(arguments: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required parameter '{key}'"))
}

fn mcp_opt_array_of_strings(
    arguments: &serde_json::Value,
    key: &str,
) -> Result<Option<Vec<String>>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_array()
            .ok_or_else(|| format!("invalid {key}"))?
            .iter()
            .map(|item| {
                item.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("invalid {key}"))
            })
            .collect::<Result<Vec<String>, String>>()
            .map(Some),
    }
}

fn mcp_opt_string(arguments: &serde_json::Value, key: &str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("invalid {key}"))
            .map(Some),
    }
}

fn mcp_opt_i64(arguments: &serde_json::Value, key: &str) -> Result<Option<i64>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(v) => v.as_i64().ok_or_else(|| format!("invalid {key}")).map(Some),
    }
}

fn mcp_role(value: &str) -> Result<Role, String> {
    match value {
        "leader" => Ok(Role::Leader),
        "executor" => Ok(Role::Executor),
        "judge" => Ok(Role::Judge),
        "explorer" => Ok(Role::Explorer),
        other => Err(format!("invalid role '{other}'")),
    }
}

fn mcp_kind(arguments: &serde_json::Value, key: &str) -> Result<Kind, String> {
    serde_json::from_value::<Kind>(
        arguments
            .get(key)
            .ok_or_else(|| format!("Missing required parameter '{key}'"))?
            .clone(),
    )
    .map_err(|e| format!("invalid {key}: {e}"))
}

fn mcp_msg_path(id: &str) -> String {
    format!("messages/{id}.json")
}

fn mcp_text(value: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "content": [
            {
                "type": "text",
                "text": value.to_string()
            }
        ],
        "isError": false
    }))
}
