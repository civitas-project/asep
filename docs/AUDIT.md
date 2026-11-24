# Audit — ASEP Agent Runtime (Layer 5, P0)

> Baseline audit before new Layer 5 capability work. Goal: avoid duplicating mechanisms the LLM or native runtime already provides (AX-01 non-redundancy).

## Existing mechanisms

| Mechanism | Location | Purpose |
|---|---|---|
| `Role` enum (Leader/Executor/Judge/Explorer) | `spec-engine/src/agent_protocol/mod.rs` | Logical responsibility on the message bus |
| `Kind` enum + `Envelope` | `spec-engine/src/agent_protocol/mod.rs` | Typed agent messages with correlation |
| `MessageStore` (send/recv/read/ack) | `spec-engine/src/agent_protocol/mod.rs` | File-based atomic message bus |
| `CapabilityProfile` | `spec-engine/src/config/settings.rs` | Capability ceiling + media/domains + delegation threshold |
| `covers()` / `missing()` / `supports_media()` | `spec-engine/src/config/settings.rs` | Capability guard before execution |
| `delegation_threshold` | `spec-engine/src/config/settings.rs` | Single-agent default trigger |
| MCP agent tools (`asep_agent_send/recv/read/ack`) | `spec-engine/src/mcp/tools.rs` | JSON-RPC entry points |
| `protocol_schema()` (TOML-driven) | `spec-engine/policy/agent-protocol.toml` | Role/kind validation rules |
| Skill lifecycle (20 submodules) | `spec-engine/src/skill/` | Skill CRUD, drift, install, sync |
| Pack / presets / installer | `spec-engine/src/pack/` | Agent provisioning, permissions |
| `TraceCollector` / `FileChangeTracker` | `spec-engine/src/observability.rs` | Telemetry + file-change evidence |

## Consumers

- Leader orchestrator (this agent) consumes `Role`, `CapabilityProfile`, message bus, pack presets.
- Executor/Judge/Explorer subagents consume agent messages and capability guards.
- MCP clients (OpenCode, etc.) consume the 12 MCP tools.
- `asep check` consumes traceability + config + skill drift.
- `asep pack` consumes `CapabilityProfile` for preset generation.

## Overlap with native runtime

- ReAct reasoning loop: native runtime (OpenCode) already provides reasoning/action/observation; ASEP must NOT rebuild an engine, only add action contracts, budget, evidence (AX-01).
- Memory/context: native runtime has session context; ASEP adds persistent memory only if empirically beneficial (P2).
- Tool execution: native runtime executes tools; ASEP adds filtering + trajectory review (FR-CAP-05/06).
- Verification: native tests already exist; ASEP adds independent judge + scoped acceptance.

## Baseline measurement plan

- Measure single-agent vs delegated task success, defect rate, token/tool usage, latency, rework on identical tasks (RFC §7.3 configs A–D).
- Ablation per capability to isolate benefit (AD-D05).
- No benchmark numbers claimed yet; this audit is the prerequisite (P0 exit criterion).
- Next slice (Engineer Level) must reuse `CapabilityProfile` and `Envelope`, not introduce parallel stores.