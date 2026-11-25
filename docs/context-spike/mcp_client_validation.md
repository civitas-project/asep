# MCP Client Validation — Criterion 4.5

## Session: `asep mcp <repo>` over JSON-RPC stdio
- Binary: /home/da578/.cargo/bin/asep
- Run dir (ASEP_RUN_DIR): /tmp/opencode/asep-context-spike/mcp_run

### 1. initialize
Request: initialize {protocolVersion: 2025-06-18, capabilities: {}}
Response: protocolVersion=4.0.0-alpha {'tools': {}}

### 2. tools/list
Tools (13): asep_check, spec_check, adr_new, render_diagrams, srcidx, srcdump, asep_accept_scoped, asep_trace_report, asep_agent_send, asep_agent_recv, asep_agent_read, asep_agent_ack, asep_affected

### 3. tools/call asep_affected (paths=[spec-engine/src/runtime/level.rs])
Response isError=False
```json
{"adrs":[],"commands":["cargo test --lib -- round_trip_serde --exact"],"impls":[{"file":"spec-engine/src/runtime/level.rs","id":"FR-LEVEL-01","line":1}],"provenance":{"Design:FR-LEVEL-01":[{"edge":null,"node":"Impl:FR-LEVEL-01"},{"edge":"Satisfies:forward","node":"Design:FR-LEVEL-01"}],"Impl:FR-LEVEL-01":[{"edge":null,"node":"Impl:FR-LEVEL-01"}],"Req:FR-LEVEL-01":[{"edge":null,"node":"Impl:FR-LEVEL-01"},{"edge":"Satisfies:forward","node":"Req:FR-LEVEL-01"}],"Test:FR-LEVEL-01":[{"edge":null,"node":"Test:FR-LEVEL-01"}]},"requirements":[{"file":"docs/PRD.md","id":"FR-LEVEL-01","line":135}],"tests":[{"file":"spec-engine/src/runtime/level.rs","function":"round_trip_serde","id":"FR-LEVEL-01","line":46}],"unmapped":[]}
```

- Impacted IDs collected: ['FR-LEVEL-01', 'FR-LEVEL-01', 'FR-LEVEL-01']

### 4. tools/call srcdump (dir=spec-engine/src/runtime, mode=signatures, format=ndjson, tokenizer=cl100k_base)
Response isError=False
```text
{"files":4,"generated":"2026-11-25T14:52:56.250606864+00:00","mode":"signatures","root":"/home/da578/Documents/Projects/Rust/asep/spec-engine/src/runtime","schema":"srcdump.v1","total_tokens_est":0,"version":1}
{"type":"file","path":"budget.rs","lang":"rust","lines":0,"tokens_est":890,"content":""}
{"type":"file","path":"level.rs","lang":"rust","lines":0,"tokens_est":532,"content":""}
{"type":"file","path":"mod.rs","lang":"rust","lines":0,"tokens_est":52,"content":""}
{"type":"file","path":"react.rs","lang":"rust","lines":0,"tokens_est":1056,"content":""}

... (truncated for display)
```

### 5. tools/call asep_agent_send (body=bundle object)
Response isError=False
```json
{
  "content": [
    {
      "text": "{\"id\":\"msg-1795618376265-2bea8f01fd8ed8fe\",\"path\":\"messages/msg-1795618376265-2bea8f01fd8ed8fe.json\"}",
      "type": "text"
    }
  ],
  "isError": false
}
```

- Message id returned: msg-1795618376265-2bea8f01fd8ed8fe

### 6. tools/call asep_agent_read (id=returned id)
Response isError=False
```json
{"bundle":"# Context Bundle — MCP Validation (criterion 4.5)\n\n## Changed path\nspec-engine/src/runtime/level.rs\n\n## Impacted node IDs (from asep_affected)\n### Impls (1)\n- FR-LEVEL-01 — spec-engine/src/runtime/level.rs:1 (function=None)\n### Requirements (1)\n- FR-LEVEL-01 — docs/PRD.md:135 (function=None)\n### Tests (1)\n- FR-LEVEL-01 — spec-engine/src/runtime/level.rs:46 (function=round_trip_serde)\n### ADRs (0)\n- (none)\n### Unmapped (0)\n- (none)\n\n## Source excerpts (from srcdump signatures mode)\n```\n{\"files\":4,\"generated\":\"2026-11-25T14:52:56.250606864+00:00\",\"mode\":\"signatures\",\"root\":\"/home/da578/Documents/Projects/Rust/asep/spec-engine/src/runtime\",\"schema\":\"srcdump.v1\",\"total_tokens_est\":0,\"version\":1}\n{\"type\":\"file\",\"path\":\"budget.rs\",\"lang\":\"rust\",\"lines\":0,\"tokens_est\":890,\"content\":\"\"}\n{\"type\":\"file\",\"path\":\"level.rs\",\"lang\":\"rust\",\"lines\":0,\"tokens_est\":532,\"content\":\"\"}\n{\"type\":\"file\",\"path\":\"mod.rs\",\"lang\":\"rust\",\"lines\":0,\"tokens_est\":52,\"content\":\"\"}\n{\"type\":\"file\",\"path\":\"react.rs\",\"lang\":\"rust\",\"lines\":0,\"tokens_est\":1056,\"content\":\"\"}\n```\n\n","commits":[],"concerns":[],"kind":"TASK_REPORT","source":"context-spike","status":"sent","tests":{"exit_code":0,"ref":"spec-engine/src/runtime/level.rs"}}
```

## Result
- Bundle carried through asep_agent_send body: OK
- Read back via asep_agent_read: OK (id=msg-1795618376265-2bea8f01fd8ed8fe)
- Read body equals sent body: True

## Caveat
This validates MCP transport/consumption of the compiled bundle text through the
existing `asep mcp` JSON-RPC stdio tools (`asep_affected`, `srcdump`,
`asep_agent_send`, `asep_agent_read`). It does NOT validate in-runtime context
injection into a native OpenCode client, which was not available as a callable
tool in this session (see docs/MCP_PILOT.md caveats).
