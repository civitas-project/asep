# Context Compiler — workload-driven spike (Layer 3, Knowledge)

> Status: design/spike plan — no implementation yet. Internal consumer: ASEP via MCP client. External consumer candidate: Orbit UI project. Source: user direction, RFC v0.2 §5–§8.

## 1. Redundancy audit

| Candidate | Existing capability | Location | Gap |
|---|---|---|---|
| Source indexing | symbol indexer with modes full/symbols-only/summary/signatures, extension filters, gitignore | `mcp/tools.rs` `srcidx`, `srcidx/src/lib.rs` | none for first spike |
| Source dump | token-efficient dump with tokenizer estimation (`cl100k_base` etc.), NDJSON | `mcp/tools.rs` `srcdump`, `srcdump/src/config.rs` | none for first spike |
| Impact selection | transitive `affected()` with typed edges, ADR/provenance/unmapped | `spec-engine/src/affected/mod.rs` | none |
| Traceability | `build_relation_graph`, coverage rules, `@rel` edges | `spec-engine/src/graph/builder.rs` | none |
| Resource limit | `ResourceBudget` / `BudgetTracker` (tokens, tool calls, cost, latency) | `spec-engine/src/runtime/budget.rs` | config exists; not yet wired into context assembly |
| MCP transport | 13 tools including `asep_affected`, `asep_check`, `srcidx`, `srcdump` | `spec-engine/src/mcp/tools.rs` | no context-bundle tool yet |

Conclusion: the inputs and filters already exist. A Context Compiler can be implemented initially as **composition of existing tools**, not a new engine, storage, or dependency.

## 2. Contract (spike)

- **Input**: task objective, changed paths, optional budget (max tokens / tool calls), caller-supplied trace id.
- **Process**: 
  1. resolve impact via `affected()` (impls, requirements, tests, ADRs, provenance, unmapped);
  2. resolve traceability closure via `build_relation_graph`;
  3. select source excerpts via `srcdump`/`srcidx` modes `signatures`/`summary` with tokenizer estimate;
  4. enforce `ResourceBudget` (stop before exceeding; report truncation explicitly).
- **Output**: deterministic context bundle with sections (objective, changed paths, impacted nodes, relevant source excerpts, open requirements, unmapped changes, budget status). Every item carries its source tool and identifier.
- **Non-goal**: no model reasoning storage, no vector database, no embedding, no new CLI binary.

## 3. Ownership & dependency

| Item | Owner | Notes |
|---|---|---|
| Repository artifacts (PRD, TDD, TRACEABILITY, ADR) | ASEP / repository | source of truth per AD-D06 |
| Runtime working context | native agent runtime | never duplicated |
| Context bundle assembly | ASEP composition layer (spike: script or MCP tool) | stateless; derived on demand |
| Budget decision | caller via `ResourceBudget` | explicit truncation, never silent |

Dependencies: existing crates/tools only. Zero new dependencies initial target.

## 4. Acceptance criteria (spike)

1. For 3 representative ASEP tasks, the bundle contains all impacted impls/requirements/tests/ADRs from `asep_affected` and no unmapped path is dropped silently.
2. Token estimate (from existing tokenizer) is lower than a full `srcdump` of the same scope, with the ratio recorded; no claim beyond measured tasks.
3. Output is deterministic for identical inputs (byte-identical bundle).
4. Budget exhaustion produces explicit truncation status, not silent omission.
5. Native MCP client can consume the bundle without a custom client (validated in MCP.3-style pilot).

## 5. Empirical validation

- Baseline A: full `srcdump` of the affected scope.
- Candidate B: compiled bundle via steps above.
- Record tokenizer estimate, tool call count, wall time, and whether all impacted nodes were present.
- Repeat at least 3 tasks; report median only. No p95 claim.

## 6. Spike plan (exploratory, before production hardening)

| Step | Deliverable | Verification |
|---|---|---|
| S1 | Compose bundle from existing tool outputs on one ASEP task (no new Rust module) | manual diff of impacted-node coverage |
| S2 | Repeat on 3 tasks; record baseline vs candidate metrics | `docs/CONTEXT_COMPILER_RESULTS.md` |
| S3 | Decision: keep composition only, or add a thin MCP tool if consumer requires it | evidence-backed, no speculative abstraction |

## 7. Downstream capabilities (same template, deferred until this spike concludes)

- **Failure Learning**: collect failure evidence from Agent Observability + Judge verdicts; audit evidence quality before any learning mechanism (user direction).
- **CoALA Memory**: persist engineering knowledge across sessions without duplicating native runtime memory (AD-D06 policy).
- **Visual/CV**: pilot with `https://github.com/dzul-x64/orbit` as external UI consumer (AD-D11 reopening).
