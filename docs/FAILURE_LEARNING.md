# Failure Learning — workload-driven design (Layer 3, Knowledge)

> Status: design before evidence audit — no learning mechanism yet. Internal consumer: ASEP leader/executor/judge loop. Source: user direction.

## 1. Redundancy audit

| Existing capability | Location | Reuse |
|---|---|---|
| Agent Observability (IN-01) | `spec-engine/src/observability.rs` | events, spans, cost metadata, file evidence, `TraceReport.failures`, `skipped`, metrics with denominators |
| Judge verdict contract | `opencode.json` (judge agent) | structured `verdict`, `blocking[]` with path/line/evidence, `non_blocking`, `missing_evidence` |
| Scoped acceptance | `spec-engine/src/acceptance_scoped.rs` | evidence files with ACCEPTED verdict, trusted runner only |
| Traceability graph | `spec-engine/src/graph/builder.rs` | links failures to requirements via existing relations |
| MCP tools | `spec-engine/src/mcp/tools.rs` | `asep_trace_report`, `asep_check`, `asep_affected` |

Conclusion: failure **evidence collection already exists**. No new store, engine, or dependency is justified before auditing evidence quality.

## 2. Contract (evidence-first)

- **Input**: trace events with failures list, judge verdicts with blocking findings, scoped acceptance results, verification status.
- **Evidence record** (derived, not stored in a new subsystem): failure ID, source (trace event / judge verdict / gate), path:line or tool:step reference, requirement ID when resolvable, verdict status, timestamp, and provenance chain from `TraceReport` / `AffectedReport`.
- **Quality gate** before learning: each failure must have (a) concrete reference, (b) measurable outcome, (c) reproducible verification command or recorded inability to reproduce, (d) no self-certification (judge advisory only; trusted runner grants acceptance).
- **Output**: a curated failure catalogue in repository Markdown, with links to existing evidence files; no new database.

## 3. Ownership & dependency

| Item | Owner |
|---|---|
| Raw trace events | TraceCollector / observability (existing) |
| Verdicts | Judge agent (advisory) |
| Acceptance status | trusted local runner only |
| Curated catalogue | repository documentation, maintained by leader |
| Learning rules | proposed as documentation/test changes, not automatic policy edits |

Dependencies: existing only. Zero new dependencies.

## 4. Acceptance criteria (spike)

1. Collect failure evidence from at least 3 real sessions/runs (including the MCP pilot and SPSC reconstructions) without adding instrumentation.
2. Audit each record against the quality gate; classify as usable, incomplete, or rejected with reason.
3. Map usable failures to requirements via existing traceability graph; report unresolved mappings explicitly.
4. Propose at most 3 candidate learnings (test, doc, or rule) with evidence citation; do not auto-apply.
5. Demonstrate that no new store or dependency was needed.

## 5. Empirical validation

- Count failures collected, usable, rejected, and mapped.
- Record which source (observability vs judge vs gate) supplied the decisive evidence.
- No claim that learning improved outcomes until a controlled before/after run exists.

## 6. Spike plan

| Step | Deliverable | Verification |
|---|---|---|
| L1 | Evidence inventory from existing logs/artifacts | `asep check --rigor T1` unchanged |
| L2 | Quality audit table with classifications | document review |
| L3 | Candidate learnings with citations, no auto-apply | document review |

Downstream: CoALA Memory and Visual/CV follow after this evidence audit, per user sequence.
