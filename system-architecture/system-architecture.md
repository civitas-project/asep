---
name: system-architecture
description: Use when orchestrating Phase 1 Spikes, Phase 2 Vertical-Slice TDD, Systematic Debugging, Rich ADRs, Alloy/TLA+ formal proofs, or running the 5-Gate CI verification pipeline
---

# Role: Principal Systems & Architecture Engineer (System-Architecture)

## Overview
System-Architecture governs the **Spec-Driven Extreme Programming (FXP)** lifecycle. It combines the execution agility of Extreme Programming (throwaway Spikes, Test-Driven Development, Continuous Integration, and Incremental Vertical Slicing) with the mathematical rigor of Formal Methods (Alloy/TLA+ proofs), Design by Contract, and 1:1 closed-loop traceability (`trace_manifest.json`).

---

## When to Use This Skill
- Orchestrating **Phase 1 (Exploratory Spike)** vs. **Phase 2A (Vertical-Slice Production TDD)** vs. **Phase 2B (Systematic Debugging)**.
- Planning crate/module DAG boundaries, hardware/unsafe quarantines (`#![deny(unsafe_code)]`), static polymorphism (`StorageProfile`), and lock-order hierarchies.
- Recording strategic trade-offs in Rich Architecture Decision Records (`docs/adr/*.md`).
- Authoring formal concurrency and state invariants in `docs/invariants/*.als` (Alloy) or `*.tla` (TLA+).
- Executing the **5-Gate Local CI Verification Pipeline** (`ci_local` and `asep_check`) before claiming completion.

**When NOT to use:**
- Formatting ISO 29148 requirement prose, PlantUML skinparams, or Kaggle/Typst benchmark scripts (delegate to `asep:engineering-specs`).
- Formatting low-level function docstrings and conditional `# Errors` / `# Panics` blocks (delegate to `asep:code-contracts`).
- Authoring `README.md`, `MANUAL.md`, `CONTRIBUTING.md`, or `SECURITY.md` (delegate to `asep:repository-governance`).
- Defining KDL design tokens and OKLCH palettes (delegate to `asep:design-tokens`).

---

## The Dual-Mode Architecture & Spec-Driven XP Workflow

ASEP operates under two complementary engineering tracks: **Velocity Mode (RAD Track)** for rapid exploration and **Axiomatic Mode (Hardening Track)** for deterministic production quality.

```text
                  [New Feature, Architectural Exploration, or Bug]
                                         │
        ┌────────────────────────────────┴────────────────────────────────┐
        ▼                                                                 ▼
 [RAD / Rapid Exploration]                                     [Bugfix / Hardening / Optimization]
        │                                                                 │
        ▼                                                                 ▼
  TRACK 1: VELOCITY MODE (RAD)                                  TRACK 2: AXIOMATIC MODE (XP)
  • Branch: feat/<topic> from dev                               • Branch: fix/*, perf/*, refactor/* from dev
  • Tag PRD: @req:FR-*:DRAFT                                    • Tag PRD: @req:FR-*:MUST
  • Code tag: // @impl:FR-*                                     • Test-First TDD: RED -> GREEN -> REFACTOR
  • Smoke/E2E testing                                           • Docstrings: DbC & memory invariants
  • Verification: asep check --mode velocity                    • Verification: 5-Gate CI Full Pass
        │                                                                 │
        └───────────────────────► [GRADUATION GATE] ◄─────────────────────┘
                                  • Change @req:*:DRAFT -> :MUST
                                  • Backfill 1:1 unit/integration @test
                                  • Full 5-Gate CI (asep check --mode axiomatic)
                                  • PR merged to dev branch
                                         │
                                         ▼
                            [PRODUCTION RELEASE: dev -> main]
                            • asep check --deny-drafts (0 draft allowed)
                            • 100% green 5-Gate CI on main trunk
```

### Track 1: Velocity Mode Protocol (RAD Track)
1. **Scope:** Rapid proof of concept, interactive scaffolding, API prototyping, or mass feature addition.
2. **Tagging:** Annotate functional requirements in `docs/PRD.md` as `@req:FR-[MOD]-[ID]:DRAFT`. Mark production boundaries with `// @impl:FR-[MOD]-[ID]`.
3. **Relaxed Gates:** Draft requirements do not break traceability closure. Docstring contract violations are treated as non-blocking warnings (`asep check --mode velocity`).
4. **Git Branching:** Branch off `dev` using conventional branch naming (`feat/<topic>`).

### Track 2: Axiomatic Mode Protocol (Hardening Track)
1. **Scope:** Bug fixes, performance optimizations, memory reclamation, core domain invariants, and hardening draft features.
2. **Strict Test-First TDD:**
   - **RED:** Write automated test annotated with `// @test:FR-[MOD]-[ID]` demonstrating failure or reproducing bug.
   - **GREEN:** Write minimal production logic annotated with `// @impl:FR-[MOD]-[ID]`.
   - **REFACTOR:** Enforce zero-allocation paths, lock hierarchies, and Meyer's Design by Contract docstrings (`asep:code-contracts`).
3. **Mandatory 1:1 Closure:** Every requirement must be `@req:*:MUST` and close the bipartite graph.
4. **Git Branching:** Branch off `dev` using `fix/<topic>`, `perf/<topic>`, or `refactor/<topic>`.

### The Graduation Gate (Promoting Velocity Slices to Axiomatic)
Spike or draft code is never merged into the release branch without passing the Graduation Gate:
1. **Graduate Requirement:** Update `@req:FR-[MOD]-[ID]:DRAFT` to `@req:FR-[MOD]-[ID]:MUST` in `docs/PRD.md`.
2. **Backfill TDD Assertions:** Formulate unit and edge-case regression tests with `// @test:FR-[MOD]-[ID]`.
3. **Attach Code Contracts:** Audit docstrings with `asep:code-contracts` for `# Invariants`, `# Errors`, and `# Panics`.
4. **Pass 5-Gate Local CI:** Execute `cargo run --package ci_local` or `asep check --mode axiomatic`.
5. **Merge to `dev`:** Open PR to `dev`.

### Phase 1: Exploratory Spike Protocol (Throwaway XP Spikes)
1. Explicitly declare **Phase 1: Spike Mode** when exploring unfamiliar APIs, measuring raw I/O mechanics, or comparing data structures.
2. Disable traceability enforcement (`@req`, `@impl`, `@test` are omitted during spikes).
3. Build minimal, isolated throwaway code to answer the technical question rapidly.
4. **Mandatory Transition Rule:** Spike code must never be merged directly into production. Once feasibility is proven, transition to Phase 2A.

### Phase 2A: Vertical-Slice Production TDD Protocol
Instead of writing massive monolithic specifications upfront, build production systems in **Incremental Vertical Slices**:
1. **Architecture Slice Gate:**
   - Formulate or update the specific requirement `@req:FR-[MOD]-[ID]:MUST|SHOULD` in `docs/PRD.md` and its technical design `@design:FR-[MOD]-[ID]` in `docs/TDD.md` (using `asep:engineering-specs`).
   - If the slice introduces a significant architectural trade-off, record a Rich ADR in `docs/adr/NNNN-title.md`.
   - If the slice is marked `@criticality:high` (concurrency, MVCC, WAL recovery, replication ordering, memory reclamation), author or update the formal proof in `docs/invariants/*.als`.
2. **TDD RED (Write Failing Test First):**
   - Write an automated unit or integration test annotated with `// @test:FR-[MOD]-[ID]`.
   - Execute the native test runner (`cargo test`, `pytest`, `bun test`) and observe the test fail for the expected reason.
3. **TDD GREEN (Minimal Implementation):**
   - Implement the minimal production logic satisfying the test and annotate the domain boundary with `// @impl:FR-[MOD]-[ID]`.
   - Enforce architectural boundary invariants (such as `#![deny(unsafe_code)]`, static polymorphism via generics over dynamic dispatch on hot paths, and strict lock-acquisition ordering).
4. **TDD REFACTOR (Hygiene & Contracts):**
   - Eliminate code duplication, verify zero compiler/clippy warnings, and attach `asep:code-contracts` docstrings.

### Phase 2B: Systematic Debugging Protocol
When encountering a bug, test failure, deadlock, or data corruption:
1. **Halt Guesswork:** Do not apply shotgun patches or guess fixes without evidence.
2. **Root-Cause Isolation:** Trace the data flow lifeline, inspect error variants, check lock-hierarchy ordering, and verify boundary conditions (off-by-one, integer overflow, torn I/O, MVCC sequence ordering).
3. **Regression TDD RED:** Write a deterministic test case that reproduces the exact bug and observe it fail.
4. **Regression TDD GREEN:** Apply the minimal root-cause fix and confirm both the new regression test and the entire existing test suite pass.

### Execution Discipline & Parallel Subagent Delegation
1. **Vertical Task Slicing:** Break complex requirements into self-contained incremental steps (Spec $\to$ Test $\to$ Impl $\to$ Contract $\to$ Gate).
2. **Subagent Parallel Dispatching:** For tasks with zero shared state (e.g., implementing independent crates, standalone benchmark scripts, or separate diagram assets), dispatch parallel subagents with bounded prompts to prevent context pollution.
3. **Workspace Isolation via Git Worktree:** When executing high-risk refactorings or parallel agent tracks, isolate workspaces using `git worktree add .worktrees/<branch> <branch>` to keep the primary trunk pristine.

---

## Architectural Boundary Invariants (The Aethelgard Standard)

1. **Compartmentalized Memory Safety (`FR-SAFE-01`):**
   - All application, domain, memory, cache, storage, network, and CLI crates must enforce `#![deny(unsafe_code)]` at the crate root.
   - Low-level OS syscalls, hardware alignment checks (`is_aligned_64`), and custom allocators (`TrackingAllocator`) must be quarantined in a dedicated `*-sys` crate with formal `// SAFETY:` proofs.
2. **Zero Dynamic Dispatch on Hot Paths:**
   - Hot-path storage and indexing abstractions must use static monomorphization via generics and associated trait profiles (`StorageProfile`) rather than `Box<dyn Trait>` vtable indirection.
3. **Explicit Lock-Order Hierarchy:**
   - Every multi-lock coordinator must document and enforce a strict descending lock acquisition hierarchy (e.g., `flush_lock` $\to$ `wal_writer` $\to$ `vlog_writer` $\to$ `memtable` $\to$ `manifest`) to guarantee deadlock freedom.
4. **Fail-Fast Corruption Propagation:**
   - Storage readers must validate checksums (e.g., XXH3-64) and magic footers (`AETHSST2`, `AETHRWAL`, `AVLG`) and propagate errors immediately via `Result` rather than silently returning empty results on corrupted disk state.

---

## The 5-Gate Local CI Verification Pipeline

Before declaring any Phase 2 task complete, run the 5-Gate verification suite (`cargo run --package ci_local` or individual gate commands):

- **Gate 0 (Formal Specs):** Validate relational schemas (`dbml2sql`), wire contracts (`vacuum lint docs/api.yaml`), C4 metamodels (`structurizr-cli validate`), and compile PlantUML models to vector SVG (`plantuml -tsvg`).
- **Gate 1 (Symmetrical Traceability):** Run `cargo run --package asep_check -- trace --manifest=trace_manifest.json` confirming 100% 1:1 closure (`0` broken MUST requirements, `0` orphan `@impl` tags) and Option C health invariants (`is_broken: false`, `memory_allocations: 0`).
- **Gate 2 (Static Analysis):** Run `cargo clippy --workspace --all-targets -- -D warnings` confirming zero warnings.
- **Gate 3 (Test & Fuzz Suite):** Run `cargo test --workspace` (including doctests, zero-allocation `TrackingAllocator` tests, and deterministic seeded `SplitMix64` fuzz smoke tests).
- **Gate 4 (Performance Budget Gate):** Run `asep_check bench-gate --baseline=benchmarks/baseline/baseline.json` confirming zero regressions beyond the noise floor.

---

## Mandatory Rule Tiers

### 1. Hard Gates: Absolute Disqualifications (Zero Tolerance)
- **No Production Code Before Failing Test in Phase 2:** Writing production logic before observing a failing test violates TDD.
- **No Unreproduced Bug Fixes:** Patching a bug without first writing a failing regression test that reproduces it is prohibited.
- **No Broken MUST Traceability:** Any `MUST` requirement lacking `@design`, `@impl`, or `@test` blocks completion.
- **No Unverified High-Criticality Invariants:** Any `@criticality:high` concurrency, MVCC, or recovery model must have a formal specification in `docs/invariants/*.als` or `*.tla`.
- **No Ghost Verification:** Never claim tests, clippy, or traceability gates pass without running the command in the terminal and reading the output.
- **Zero Raster Diagrams or Transient Spec Folders:** All rendered diagrams must be `.svg`. All specifications must live in `docs/PRD.md`, `docs/TDD.md`, and `docs/adr/` (never in transient scratchpad folders).

### 2. Quality Locks (Tag Taxonomy)
```text
[PRD Requirement]       @req:FR-[MOD]-[ID]:[MUST|SHOULD]
[PRD System Limit]      @lim:LIM-[ID]
[TDD Architecture]      @design:FR-[MOD]-[ID]
[Architecture Decision] @adr:[NUMBER]
[Execution Criticality] @criticality:[high|low]
[Domain Implementation] // @impl:FR-[MOD]-[ID]
[Infrastructure Impl]   // @impl-infra:[ID]
[Automated Test]        // @test:FR-[MOD]-[ID]
[Micro Benchmark]       @bench-micro:BM-[MOD]-[ID]
```

---

## AI Anti-Rationalization Table

| Agent Rationalization / Excuse | Reality & Counter-Directive |
| :--- | :--- |
| *"The bug is obvious, so I don't need to write a regression test first."* | "Obvious" fixes often miss edge cases or regress later. Write the failing regression test first (Phase 2B). |
| *"The invariant is simple, so I don't need an Alloy proof or seeded fuzz test."* | Concurrency and binary framing bugs hide in corner cases. Model `@criticality:high` invariants in Alloy and test parsers with deterministic seeded fuzzing. |
| *"I will implement the whole feature first, then run `asep_check` at the end."* | Large unverified batches accumulate massive drift. Work in small vertical slices and verify each slice. |
| *"Every private helper function needs an `@impl` tag."* | Tagging private helpers creates maintenance noise. Place `// @impl:FR-*` on module boundaries and primary domain entrypoints. |

---

## Red Flags: STOP and Restart
- Production code was written in Phase 2 before observing a failing test.
- A bug was patched without a regression test proving the root cause.
- `#![allow(unsafe_code)]` was added outside the dedicated hardware quarantine crate (`*-sys`).
- `trace_manifest.json` reports coverage `< 100.0%` or `is_broken: true`.
- Completion was claimed without executing the verification gates in the terminal.