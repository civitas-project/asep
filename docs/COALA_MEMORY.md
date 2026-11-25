# CoALA-inspired Memory — workload-driven design (Layer 3, Knowledge)

> Status: exploration — no implementation. Policy: AD-D06 native-first ownership. Source: user direction.

## 1. Redundancy audit

| CoALA class | Native runtime equivalent | ASEP gap |
|---|---|---|
| Working memory | native working context / conversation | none; do not duplicate |
| Episodic memory | session history (runtime-owned) | only durable engineering checkpoints across sessions |
| Semantic memory | repository facts, PRD/TDD/TRACEABILITY/ADR | none; repository is source of truth |
| Procedural memory | skills, prompts, runbooks | none; existing skill system covers it |

Existing ASEP artifacts: PRD, TDD, TRACEABILITY, ADR, DOUBT, evidence files, observability traces, judge verdicts, `MessageStore` envelopes. No vector store, embedding, or custom memory engine exists — and none is justified without a reproducible failure.

## 2. Contract

- **Input**: engineering knowledge needing persistence across sessions/runtimes (decisions, constraints, acceptance evidence, task checkpoints).
- **Storage owner**: repository artifacts (Markdown/JSON/TOML), as already defined in `docs/MEMORY_ARCHITECTURE.md` (withdrawn from production but policy retained).
- **Native runtime owner**: working context, conversation history, tool observations, session state.
- **Boundary rule**: ASEP never writes a second copy of runtime state. Retrieval reads repository artifacts; invalidation is an artifact edit plus traceability update.
- **Portability**: artifacts are plain text, readable by any runtime.

## 3. Ownership & dependency

| Item | Owner |
|---|---|
| Working context | native agent runtime |
| Durable engineering state | ASEP / repository |
| Retrieval trigger | caller (leader) at session/task start |
| Checkpoint write | explicit, at task boundary; not crash-safe unless runtime hook exists (AD-D07) |

Dependencies: zero new dependencies. No new module, store, or engine.

## 4. Acceptance criteria (exploration)

1. Identify one real cross-session recovery scenario in ASEP (e.g. resume after compaction) where repository artifacts alone were sufficient or insufficient.
2. If sufficient, record that no memory subsystem is needed (default outcome).
3. If insufficient, document the exact missing knowledge item, its owner, and why native runtime could not preserve it — this is the AD-D06 reopening criterion.
4. Demonstrate that no duplicate of native working memory was created.

## 5. Empirical validation

- Use existing traces/verdicts and session notes; no new instrumentation.
- Record whether recovery succeeded using repository artifacts only.
- No claim of benefit without a repeated, reproducible scenario.

## 6. Spike plan

| Step | Deliverable | Verification |
|---|---|---|
| M1 | Collect 2 cross-session scenarios from existing work | document evidence |
| M2 | Classify as sufficient / insufficient with owner and gap | document review |
| M3 | Decision: no new memory engine, or reopen AD-D06 with evidence | evidence-backed |

Visual/CV pilot remains last in the user-defined sequence.
