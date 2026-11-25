# MCP.4 — Gap Analysis (Consumer View)

> Sources: `docs/MCP_AUDIT.md`, `docs/MCP_PILOT.md`, `FR-MCP-IMPACT-01`, `FR-MCP-IMPACT-02`.  
> No source changes. No custom MCP client introduced.

## 1. Consumer requirements

| Requirement | What the consumer needs |
|---|---|
| Discovery | Enumerate available MCP tools and schemas so an agent can know what impact and trace capabilities exist before calling them. |
| Impact analysis | Given changed paths, determine affected implementations, requirements, tests, ADRs, unmapped changes, provenance, and related commands. |
| Verification | Run traceability checks over the repo or scoped paths and confirm requirements are satisfied by impls/tests. |
| Acceptance | Record scoped acceptance of changes against a commit within defined boundaries. |
| Trace/reporting | Produce trace reports (status/report/purge surface) and link requirement → impl → test → ADR for auditors. |

## 2. Mapping to existing MCP tools

| Consumer need | MCP tool | Coverage |
|---|---|---|
| Discovery | `tools/list` (server) | 13 tools returned in pilot; schemas typed. |
| Traceability check / verification | `asep_check` | Exposes `asep check` via MCP; covers requirement → impl → test verification. |
| Impact analysis | `asep_affected` | Returns `impls`, `requirements`, `tests`, `adrs`, `unmapped`, `provenance`, `commands` for changed paths; reuses `affected()` and `AffectedReport`. |
| Source inspection | `srcdump`, `srcidx` | Source dump and index for deep-dive after impact report. |
| Acceptance | `asep_accept_scoped` | Scoped acceptance of a commit within boundaries. |
| Trace/reporting | `asep_trace_report` | Trace reports for audit. |
| ADR scaffold | `adr_new` | Creates ADRs referenced by impact output. |
| Diagrams | `render_diagrams` | Renders diagrams from existing data; no graph export. |
| Agent messaging | `asep_agent_send/recv/read/ack` | Agent-to-agent messaging within MCP session. |

Also present but out of pilot scope: `spec_check` (legacy alias), status/purge trace subcommands not exposed, `asep ci`/`harness`/`skill` not exposed.

## 3. Gap verdict

Existing tools **sufficiently satisfy** the pilot consumer for discovery, impact analysis, verification, acceptance, and trace/reporting. No demonstrated need for full graph topology export.

Evidence from pilot (`docs/MCP_PILOT.md`):

- `tools/list` returned **13 tools**, including `asep_affected` (audit counted 12 pre-pilot; pilot confirmed 13 after additive tool).
- `asep_affected` with `paths=["src/lib.rs"]` returned success with all seven fields present: `impls`, `requirements`, `tests`, `adrs`, `unmapped`, `provenance`, `commands`; impls=1, requirements=1, tests=1.
- Empty `paths` → JSON-RPC execution error `-32603`, message `'paths' must not be empty`. Schema validation works.
- `initialize` accepted client `protocolVersion: 2025-06-18`; server replied `2024-11-05` with no negotiation and no rejection. Version mismatch silently tolerated.
- Pilot used a **direct JSON-RPC stdio client** (`asep mcp < requests.ndjson > responses.ndjson`), not OpenCode's built-in MCP client. Native client discovery UI, permission prompts, and structured content rendering were not exercised.
- 4 JSON-RPC calls total (1 init, 1 list, 2 invokes). Token usage and latency not exposed/measured.

No source changes required. `asep_affected` was added by reusing `affected()` + `AffectedReport` (additive: 12 → 13 tools), matching FR-MCP-IMPACT-01 and FR-MCP-IMPACT-02.

## 4. Decision

- **P3.3 (graph export): DEFERRED.** Reopening criterion: a demonstrable consumer need for full graph topology that cannot be obtained from `asep_check` / `asep_affected` reports.
- **Custom MCP client: not introduced.** Native OpenCode MCP client via `opencode.json` remains the intended path; AX-01 / AD-D01 preserved.

## 5. Caveats

- No token or latency measured from a real agent session.
- Native OpenCode MCP client not exercised in pilot; behavior beyond `2024-11-05` unverified.
- Protocol version mismatch (`2024-11-05` server vs `2025-06-18` requested) silently tolerated; real client compatibility unverified.
