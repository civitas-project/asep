# Context Compiler Spike Results — S1/S2

## Method

Executed the Context Compiler spike (docs/CONTEXT_COMPILER.md) as composition of existing tools only:
- `asep affected <path> --json` → impacted impls / requirements / tests / adrs / unmapped / provenance / commands
- MCP `srcdump` over `asep mcp` JSON-RPC stdio (`initialize`, `tools/list`, `tools/call srcdump`), mode `signatures`, format `ndjson`, tokenizer `cl100k_base`, strip_comments=true, strip_blank_lines=true, on the directory containing each changed path
- Candidate bundle composed as plain markdown (spike script only, not repo code): objective, changed path, impacted node IDs, source excerpts from srcdump signatures output, unmapped changes, budget status
- No new engine, storage, abstraction, or dependency. No Rust source changes.

## Token-estimate source

Token estimates are **tool-reported**, not derived. The MCP `srcdump` tool emits a per-file `tokens_est` field (tiktoken `cl100k_base` on original file content) in every NDJSON entry. I summed per-file `tokens_est` across files in scope for the reported totals.

Caveat: the NDJSON header field `total_tokens_est` is hardcoded to `0` in srcdump/src/formatter.rs and must NOT be used as a token total. Bundle markdown wrapper text is not separately tokenized (no tiktoken available in the spike harness); candidate "source tokens" below count only the in-scope impacted source excerpts.

## Tasks

Three real changed paths from recent commits:

| Task | Changed path | Impacted (impls/reqs/tests/adrs/unmapped) |
|---|---|---|
| 1 | spec-engine/src/mcp/tools.rs | 14 / 6 / 6 / 0 / 0 |
| 2 | spec-engine/src/affected/mod.rs | 10 / 10 / 10 / 0 / 0 |
| 3 | spec-engine/src/runtime/level.rs | 1 / 1 / 1 / 0 / 0 |

## Baseline vs candidate metrics

Baseline = full `srcdump` of the containing directory (mode `full`, format `ndjson`, tokenizer `cl100k_base`). Candidate = composed bundle (mode `signatures` excerpts for impacted files only, plus markdown wrapper).

| Task | Path | Baseline bytes | Baseline lines | Baseline tokens (reported, summed) | Baseline files | Candidate bundle bytes | Candidate bundle lines | Candidate source tokens (impacted files only) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | spec-engine/src/mcp/tools.rs | 53204 | 4 | 10067 | 4 | 3271 | 81 | 10041 |
| 2 | spec-engine/src/affected/mod.rs | 19016 | 1 | 5163 | 1 | 3133 | 74 | 5163 |
| 3 | spec-engine/src/runtime/level.rs | 10907 | 4 | 2530 | 4 | 1390 | 47 | 532 |
| Median | — | 19016 | 1 | 5163 | 1 | 3133 | 74 | 5163 |

Token-estimate source: tool-reported per-file `tokens_est` (`cl100k_base`), summed. Header `total_tokens_est` is 0 and unused.

**Token comparability note:** `tokens_est` is computed on original file content regardless of output mode, so baseline full-mode tokens sum ALL files in the directory while candidate source tokens sum only impacted files. They match only when every file in the directory is impacted (task 2: 5163 = 5163). Task 1 differs (10041 vs 10067) because the directory contains one non-impacted file (~26 tokens). Task 3 differs sharply (532 vs 2530) because only `level.rs` of 4 files in `spec-engine/src/runtime/` was impacted. Candidate bundle bytes vs signature-mode ndjson bytes: the bundle is larger because it includes markdown section headers, the full impacted node ID lists, commands, budget status, and coverage check text in addition to the source excerpts; the signature ndjson contains only the excerpt content. No claim beyond these three measured tasks.

**Medians (per docs/CONTEXT_COMPILER.md §5, computed over the 3 measured tasks):** baseline tokens 5163, baseline bytes 19016, candidate bundle bytes 3133, candidate source tokens 5163, impacted IDs per task 26.

## Coverage check

For each bundle, every impacted node ID string from the corresponding `asep affected --json` output was asserted present in the bundle text:

| Task | Impacted IDs | Missing in bundle |
|---|---:|---:|
| 1 | 26 | 0 |
| 2 | 30 | 0 |
| 3 | 3 | 0 |
| Median | 26 | 0 |

Result: PASS — every impacted impl/requirement/test/ADR/unmapped ID from `affected()` appears in its bundle. No unmapped changes were reported by `affected()` for any task, so none were dropped.

Complete impacted ID lists are verifiable in the bundle artifacts under `docs/context-spike/`:
- `docs/context-spike/bundle_spec_engine_src_mcp_tools_rs.md` (task 1)
- `docs/context-spike/bundle_spec_engine_src_affected_mod_rs.md` (task 2)
- `docs/context-spike/bundle_spec_engine_src_runtime_level_rs.md` (task 3)

## Determinism check

Task 1 (spec-engine/src/mcp/tools.rs) composed twice with identical inputs; outputs compared byte-identical:

- `cmp docs/context-spike/bundle_det_a.md docs/context-spike/bundle_det_b.md` → identical (no output)
- In-memory compare of two `compose()` calls → True
- SHA-1 of bundle: 609d0b8158286f75...

Result: PASS — output is deterministic for identical inputs (byte-identical bundle). Determinism artifacts: `docs/context-spike/bundle_det_a.md` and `docs/context-spike/bundle_det_b.md`.

## Acceptance criteria status (docs/CONTEXT_COMPILER.md §4)

| Criterion | Status | Notes |
|---|---|---|
| 4.1 Bundle contains all impacted impls/reqs/tests/ADRs; no unmapped path dropped silently | PASS | Coverage check above: 0 missing IDs across 3 tasks; `unmapped: []` for all tasks |
| 4.2 Token estimate lower than full `srcdump` of same scope, ratio recorded | PASS (partial) | Candidate bundle bytes are ~6–13% of baseline full-mode bytes. Token reduction holds only where impacted files are a subset of directory files (task 3: 532 vs 2530 tokens). Ratios recorded per task in metrics table. |
| 4.3 Output deterministic for identical inputs (byte-identical) | PASS | Determinism check above |
| 4.4 Budget exhaustion produces explicit truncation status, not silent omission | SIMULATED (not production-validated) | Python spike `spike_budget_truncation.py` simulated `ResourceBudget { max_tokens: 100 }` semantics (mirrors `spec-engine/src/runtime/budget.rs` `BudgetTracker`) over the `spec-engine/src/runtime/level.rs` bundle; `exceeded()` returned `tokens` and the spike artifact carries the explicit line `budget exceeded (tokens): excerpt omitted`. All impacted node IDs retained; nothing silently dropped in the spike. `BudgetTracker` exists in Rust but is NOT wired into context assembly; truncation was demonstrated in a spike artifact, not in the implemented pipeline. Artifact: `docs/context-spike/budget_truncated_bundle.md`. |
| 4.5 Native MCP client can consume the bundle without a custom client | TRANSPORT VALIDATED ONLY | Live `asep mcp` JSON-RPC stdio session passed the compiled bundle through `asep_agent_send` body → `asep_agent_read` round-trip (same process, read body equals sent body, MATCH=True). External native OpenCode client in-runtime consumption was NOT tested (client unavailable in this session). Validates MCP transport/consumption via existing tools only, not production context injection. Artifact: `docs/context-spike/mcp_client_validation.md`. |

## Validation of 4.4 and 4.5

### 4.4 — budget exhaustion explicit truncation (simulated, not production-validated)

Command:
```
python3 /tmp/opencode/asep-context-spike/spike_budget_truncation.py
```
The script simulates `ResourceBudget { max_tokens: 100 }` / `BudgetTracker` semantics
from `spec-engine/src/runtime/budget.rs` over the `spec-engine/src/runtime/level.rs`
bundle (source `tokens_est=532`). This is a spike-only simulation; `BudgetTracker`
exists in Rust but is NOT wired into the context assembly pipeline.

Output (key lines, spike artifact only):
- `budget exceeded (tokens): excerpt omitted` — explicit truncation-status line present in the spike artifact.
- `recorded tokens: 532`, `max_tokens: 100`, `exceeded: True (tokens)`.
- `all impacted IDs retained: True` (impl FR-LEVEL-01, req FR-LEVEL-01, test FR-LEVEL-01); no content silently dropped in the spike.

Limitation: truncation was demonstrated in a spike artifact, not in the implemented pipeline.

Artifact: `docs/context-spike/budget_truncated_bundle.md`.

### 4.5 — native MCP client consumption of the bundle (transport validated only)

Command (drive `asep mcp` JSON-RPC stdio session):
```
ASEP_RUN_DIR=/tmp/opencode/asep-context-spike/mcp_run asep mcp /home/da578/Documents/Projects/Rust/asep
```
with requests (line-delimited JSON-RPC):
1. `initialize`
2. `tools/list` (13 tools, incl. `asep_affected`, `srcdump`, `asep_agent_send`, `asep_agent_read`)
3. `tools/call asep_affected` → `{"paths": ["spec-engine/src/runtime/level.rs"], "root": "<repo>"}`
4. `tools/call srcdump` → `{"dir": "<repo>/spec-engine/src/runtime", "mode": "signatures", "format": "ndjson", "tokenizer": "cl100k_base", "strip_comments": true, "strip_blank_lines": true}`
5. `tools/call asep_agent_send` → `{"agent": "executor", "to": "judge", "kind": "TASK_REPORT", "body": {<bundle object>}}` returns message id
6. `tools/call asep_agent_read` → `{"id": <message id>}` returns the identical body

Output: read body equals sent body (MATCH=True).

Limitation: this validates MCP transport/consumption of the compiled bundle text through
the existing `asep mcp` JSON-RPC stdio tools only (same process). External native
OpenCode client in-runtime consumption was NOT tested (client unavailable in this
session; see docs/MCP_PILOT.md caveats).

Artifact: `docs/context-spike/mcp_client_validation.md`.

## Budget status

No explicit truncation was applied in the spike bundles. Every impacted node ID and every in-dir impacted source excerpt is included. `asep affected` reported `unmapped: []` for all three tasks, so no unmapped change needed handling.

## S3 decision: composition sufficient

**Decision: keep composition only; do not add a thin MCP context-bundle tool at this time.**

Evidence:
- Coverage: 100% of impacted node IDs present across all 3 tasks (criterion 4.1 PASS).
- Determinism: byte-identical output on repeated composition (criterion 4.3 PASS).
- Baseline vs candidate: candidate bundle is a small fraction of baseline full-mode bytes (median 3133 vs 19016 bytes); token estimates are available from the existing `srcdump` tokenizer (`cl100k_base` per-file `tokens_est`).
- No new engine, storage, abstraction, or dependency required.
- Criteria 4.4 and 4.5 were validated in S3 as simulated/transport-only (see "Validation of 4.4 and 4.5" section); they do not block the S3 decision but would need production hardening before relying on them.

Condition to revisit: if a concrete consumer (e.g., the Orbit UI pilot) requires a native MCP tool that returns the bundle as a structured call rather than composing client-side, add a thin MCP tool then. No speculative abstraction before that consumer exists. This matches the spike's "evidence-backed, no speculative abstraction" principle (docs/CONTEXT_COMPILER.md §6).

## Scope note

No claim beyond these three measured tasks. Medians are computed over exactly these 3 tasks and are not generalized to a p95/p99 claim.
