# Context Bundle — Task 1

## Objective
Understand impact of the MCP.2 asep_affected tool implementation (tools.rs) on its impls, requirements, and tests.

## Changed path
spec-engine/src/mcp/tools.rs

## Impact source
- Tool: `asep affected --json` (MCP: `asep_affected`), run at repo root.
- Source excerpts: MCP `srcdump` on containing directory, mode=signatures, format=ndjson, tokenizer=cl100k_base.

## Impacted node IDs (from affected())
### Impls (14)
- FR-CAP-02 — spec-engine/src/mcp/tools.rs:3 (function=None)
- FR-MCP-01 — spec-engine/src/mcp/server.rs:8 (function=None)
- FR-MCP-02 — spec-engine/src/mcp/tools.rs:23 (function=None)
- FR-MCP-03 — spec-engine/src/mcp/tools.rs:1 (function=None)
- FR-MCP-IMPACT-01 — spec-engine/src/mcp/tools.rs:384 (function=None)
- FR-MCP-IMPACT-02 — spec-engine/src/mcp/tools.rs:998 (function=None)
- FR-MCP-PROTO-01-A — spec-engine/src/mcp/protocol.rs:5 (function=None)
- FR-MCP-PROTO-01-B — spec-engine/src/mcp/server.rs:8 (function=None)
- FR-MCP-PROTO-02-A — spec-engine/src/mcp/protocol.rs:6 (function=None)
- FR-MCP-PROTO-02-B — spec-engine/src/mcp/tools.rs:23 (function=None)
- FR-MCP-PROTO-02-C — spec-engine/src/mcp/server.rs:9 (function=None)
- FR-MCP-PROTO-03-A — spec-engine/src/mcp/tools.rs:24 (function=None)
- FR-MCP-PROTO-03-B — spec-engine/src/mcp/server.rs:10 (function=None)
- FR-MCP-PROTO-04 — spec-engine/src/mcp/server.rs:11 (function=None)

### Requirements (6)
- FR-CAP-02 — docs/PRD.md:39 (function=None)
- FR-MCP-01 — docs/PRD.md:8 (function=None)
- FR-MCP-02 — docs/PRD.md:9 (function=None)
- FR-MCP-03 — docs/PRD.md:54 (function=None)
- FR-MCP-IMPACT-01 — docs/PRD.md:94 (function=None)
- FR-MCP-IMPACT-02 — docs/PRD.md:95 (function=None)

### Tests (6)
- FR-CAP-02 — spec-engine/src/traceability_tests.rs:10 (function=None)
- FR-MCP-01 — spec-engine/src/traceability_tests.rs:137 (function=test_jsonrpc_request_deserialization_and_processing)
- FR-MCP-02 — spec-engine/src/traceability_tests.rs:152 (function=test_mcp_tool_catalogue_contains_expected_tools)
- FR-MCP-03 — spec-engine/src/traceability_tests.rs:3 (function=None)
- FR-MCP-IMPACT-01 — spec-engine/src/traceability_tests.rs:8 (function=None)
- FR-MCP-IMPACT-02 — spec-engine/src/traceability_tests.rs:8 (function=None)

### ADRs (0)
- (none)

### Unmapped (0)
- (none)

## Commands (from affected())
- `cargo test --lib -- test_jsonrpc_request_deserialization_and_processing --exact`
- `cargo test --lib -- test_mcp_tool_catalogue_contains_expected_tools --exact`

## Source excerpts (from srcdump signatures mode)
### `protocol.rs`  (lang=rust, lines=0, tokens_est=376)
```

```

### `server.rs`  (lang=rust, lines=0, tokens_est=713)
```

```

### `tools.rs`  (lang=rust, lines=0, tokens_est=8952)
```

```

## Unmapped changes
- none reported by affected()

## Budget status
- Candidate source tokens (impacted files only, tool-reported cl100k_base tokens_est, summed): 10041
- Markdown wrapper text not separately tokenized (no tiktoken in spike harness).
- Status: explicit truncation none — bundle includes all impacted node IDs and all in-dir impacted source excerpts.

## Coverage check
- Every impacted node ID from affected() is listed above under Impacted node IDs.
