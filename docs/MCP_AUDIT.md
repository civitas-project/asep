# MCP.1 — Capability Audit

> Audit-first step of `MCP Impact Integration & Client Pilot`. No code changes. Sources: `spec-engine/src/mcp/`, `spec-engine/src/cli.rs`, `opencode.json`.

## 1. Existing MCP capabilities

| Capability | Tool | Source |
|---|---|---|
| Traceability check | `asep_check` | `mcp/tools.rs:29` |
| Legacy check alias | `spec_check` | `mcp/tools.rs:54` |
| ADR scaffold | `adr_new` | `mcp/tools.rs:79` |
| Diagram render | `render_diagrams` | `mcp/tools.rs:97` |
| Source index | `srcidx` | `mcp/tools.rs:110` |
| Source dump | `srcdump` | `mcp/tools.rs:158` |
| Scoped acceptance | `asep_accept_scoped` | `mcp/tools.rs:226` |
| Trace report | `asep_trace_report` | `mcp/tools.rs:244` |
| Agent messaging | `asep_agent_send/recv/read/ack` | `mcp/tools.rs:262-358` |

Total: **12 tools**. Dispatch: `execute_tool` match at `mcp/tools.rs:392`, unknown tool error at `:977`.

## 2. CLI → MCP gaps (relevant to pilot)

| CLI command | MCP exposure | Note |
|---|---|---|
| `asep affected <paths> [--json]` | **missing** | Direct gap for MCP.2 |
| `asep check` | `asep_check` | covered |
| `asep trace status/report/purge` | report only | status/purge not exposed |
| `asep ci` | not exposed | not needed for pilot |
| `asep harness` | not exposed | not needed |
| `asep skill` | not exposed | not needed |

## 3. Protocol & client compatibility

- `initialize` returns a fixed `protocolVersion: "2024-11-05"` (`mcp/server.rs:32-42`). Client-provided version is **not negotiated**; server does not reject mismatches.
- Capabilities advertised: `tools` only (`server.rs:37`). No resources/prompts.
- JSON-RPC methods: `initialize`, `tools/list`, `tools/call`, `ping`, notifications (null id → no response), parse error `-32700`, method not found `-32601`, execution error `-32603`.
- Native client config: `opencode.json` wires MCP via `mcp.servers.asep` (v2 shape). No custom client currently needed.

## 4. Audit conclusion

- The MCP server already exposes the core engineering tools with typed schemas.
- The single relevant gap for the pilot is **`asep_affected`**, which can be added by reusing `AffectedReport` and the existing `affected()` pipeline — no separate impact engine.
- Protocol version is advertised but not negotiated; acceptable for the existing native client, record as a caveat for MCP.3.
- No new dependencies needed for MCP.2.

## 5. Constraints carried forward

- Reuse native MCP clients; custom client only if MCP.3 finds an unmet requirement.
- Preserve AD-D01 (no duplicated runtime capability).
- P3.3 graph export remains DEFERRED until MCP.4 demonstrates need for full topology.
