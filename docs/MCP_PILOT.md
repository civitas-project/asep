# MCP.3 — Client Pilot (JSON-RPC stdio)

> Pilot of existing MCP server capabilities using a direct JSON-RPC stdio client.  
> No source changes; native OpenCode MCP client was not available as a callable tool in this session — see caveats.

## Setup

- Pilot project: `/tmp/opencode/asep-mcp3-pilot` with a requirement tag, an impl node, a test node, satisfies/verifies relations.
- Requests: `initialize`, `tools/list`, `tools/call asep_affected` (valid), `tools/call asep_affected` (empty paths).
- Command: `asep mcp < requests.ndjson > responses.ndjson`.

## Results

| Check | Outcome |
|---|---|
| Protocol compatibility | `initialize` accepted with client `protocolVersion: 2025-06-18`; server replied `2024-11-05`. No rejection; no negotiation. |
| Tool discovery | `tools/list` returned **13 tools**, including `asep_affected`. |
| Invocation | `asep_affected` with `paths=["src/lib.rs"]`, `root=<pilot>` returned success. |
| Structured result | Tool returned success (`isError` false); JSON text parsed with all seven fields present (`impls`, `requirements`, `tests`, `adrs`, `unmapped`, `provenance`, `commands`); impls=1, requirements=1, tests=1; adrs/unmapped/provenance/commands counts not captured, presence only. |
| Error handling | Empty `paths` → JSON-RPC error `-32603`, message `'paths' must not be empty`. |
| Overhead | 4 JSON-RPC calls total (1 init, 1 list, 2 invokes). Token usage not exposed by this client. |
| Practical usefulness | `asep_affected` gives the agent exact changed-path → test/requirement impact without reading source; directly useful for scoping verification. |

## Caveats

- This was a **direct stdio JSON-RPC client**, not OpenCode's built-in MCP client. Native client behavior (discovery UI, permission prompts, structured content rendering) was not exercised.
- Protocol version mismatch is silently tolerated; real client compatibility beyond `2024-11-05` is unverified.
- Token/tool-call overhead from a real agent session not measured.
- No custom MCP client introduced (AX-01 / AD-D01 preserved).

## MCP.4 preliminary gap assessment

Existing tools (`asep_check`, `asep_affected`, `asep_accept_scoped`, `asep_trace_report`) cover discovery, impact, verification, acceptance, and reporting. No demonstrated need for full graph topology export. **P3.3 remains DEFERRED** pending a concrete consumer that requires full topology beyond these reports.

## Final native pilot (current binary)

OpenCode v2.4.1 native client: `opencode mcp list` reports `asep connected`.
A native `asep_check` invocation at rigor T1 returned **FAIL**, 129 requirements,
22 broken MUST, exit code 1. Direct CLI `asep check --rigor T1 .` returned the
same verdict and counts (FAIL, 129, 22). Discovery, invocation, error surfacing
and CLI/MCP parity are therefore verified for the current binary.
