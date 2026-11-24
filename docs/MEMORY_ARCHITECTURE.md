# Memory Architecture (P2, documentation-first)

> Principle: **Memory architecture is fundamentally an information ownership and lifecycle problem, not a storage technology problem.**  
> Source: `ASEP_Architectural_Decisions.md` §2, AD-D06. Status: design document, no implementation yet.

## 1. Ownership model (native-first)

Separate durable engineering state from runtime execution state.

| ASEP / repository — engineering state | Native runtime — execution state |
|---|---|
| Requirements and acceptance criteria | Working context |
| Decisions and rationale | Conversation history |
| Acceptance evidence | Tool execution state |
| Task checkpoints | Agent lifecycle state |

- The **repository is the source of truth** for durable engineering state.
- The **runtime is the source of truth** for its own execution state.
- ASEP persists only engineering knowledge needed across sessions/runtimes that native facilities do not reliably preserve.
- Do not introduce a vector database, custom memory manager, semantic retrieval engine, or context compression engine by default.

## 2. Information tiers

| Tier | Owner | Lifetime | Examples |
|---|---|---|---|
| Agent | Agent | Per task | Consumed/produced information, intermediate reasoning |
| Native runtime | Runtime | Per session | Working context, conversation history, tool observations, session state |
| ASEP (engineering memory) | Repository | Cross-session | Architectural decisions, engineering constraints, acceptance evidence, task checkpoints |
| Repository artifacts | Repository | Persistent | PRD, TDD, TRACEABILITY, ADR, evidence, source |

## 3. Eight design aspects (contracts, not components)

| Aspect | Fundamental question |
|---|---|
| Classification | What information must be preserved? |
| Ownership | Who is responsible for that information? |
| Persistence | When must it be stored? |
| Retrieval | When must it be loaded back? |
| Invalidation | When is it no longer valid? |
| Consistency | How to handle conflicting information? |
| Recovery | How to continue after context loss? |
| Portability | How does it survive a runtime change? |

## 4. P2.0 — Capability audit (inventory)

Inventory native memory capabilities and existing ASEP storage before designing anything new.

### Native runtime (to be audited per runtime)
- Working context window size and eviction policy.
- Conversation history persistence across sessions.
- Tool observation retention.
- Session state (checkpoint/resume) support.
- Compaction/session hooks (AD-D07).

### Existing ASEP storage (already present)
- `docs/PRD.md`, `docs/TDD.md`, `docs/TRACEABILITY.md` — requirements/design/relations.
- `docs/adr/` — architectural decisions.
- `.spec-engine/evidence/semantic/` — acceptance evidence.
- `.spec-engine/trace/` — observability NDJSON (IN-01).
- `docs/DOUBT.md` — doubt tracker.
- `MessageStore` (file-based envelopes) — agent messages.

### Gap hypothesis (to verify, not assume)
- Native runtimes may already preserve enough context; ASEP's durable engineering state (requirements, decisions, evidence) is the likely unique value.

## 5. P2.1 — Memory classification

Classify information by **function and lifetime**, not by storage technology.

| Class | Lifetime | Owner | Examples |
|---|---|---|---|
| Transient | Within one action | Agent | Intermediate calculation, scratchpad |
| Session | Within one session | Native runtime | Working context, conversation, tool observations |
| Durable engineering | Cross-session | ASEP/repository | Requirements, design, decisions, evidence, checkpoints |
| Permanent | Indefinite | Repository | Released code, accepted ADRs |

Rule: an item is promoted to a higher lifetime class only when a concrete need (recovery, audit, portability) requires it.

## 6. P2.2 — Ownership contract

- **Native runtime owns** execution state: working context, conversation, tool observations, session state, agent lifecycle.
- **ASEP/repository owns** durable engineering state: requirements, design, decisions, acceptance evidence, task checkpoints, doubt tracker.
- **Boundary rule:** ASEP never duplicates runtime execution state; it persists only engineering artifacts that native facilities do not reliably preserve.
- **Source of truth:** repository for engineering state; runtime for execution state. Conflicts resolved in favor of the designated owner.
- **Portability:** engineering state must be readable without the originating runtime (plain Markdown/JSON/TOML).

## 7. P2.3 — Memory lifecycle (outline)

| Operation | Trigger | Owner | Mechanism |
|---|---|---|---|
| Persistence | Task checkpoint / session end / explicit close | ASEP | Write to repository artifacts (existing files) |
| Retrieval | Session start / task dispatch | ASEP | Read relevant artifacts (PRD, TDD, TRACEABILITY, ADR, evidence) |
| Invalidation | Requirement changed / decision superseded / evidence stale | ASEP | Update artifact + traceability relation |
| Recovery | Context loss / new runtime | ASEP | Re-read repository artifacts; no runtime state assumed |
| Consistency | Conflicting information detected | Judge | Flag as blocking finding; Leader resolves |

No new storage engine; reuse existing files and `MessageStore`.

## 8. P2.4 — Gap analysis

After P2.0–P2.3, identify gaps that existing mechanisms cannot satisfy. A gap is valid only if:
1. A reproducible failure to preserve/recover necessary engineering state using repository artifacts and native runtime facilities (AD-D06 reopening criteria).
2. No existing artifact or native capability covers it.
3. The benefit is measurable.

## 9. P2.5 — Conditional implementation

Implement only proven gaps. Default outcome: **no new memory engine**; existing artifacts suffice.

### Reopening criteria (AD-D06)
A reproducible failure to preserve or recover necessary engineering state using repository artifacts and native runtime facilities.

## 10. Constraints

- Documentation-first; no new Rust modules, dependencies, or runtimes in this phase.
- Additive only; no breaking change to existing CLI, MCP, schema (IN-01-COMPAT precedent).
- Native ReAct execution retained (AD-D01).
- Deferred items (AD-D05, AD-D07, AD-D11, AD-D12) remain deferred; use real projects as evidence source.