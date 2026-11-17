---
name: repository-governance
description: Use when authoring or auditing root governance files (README.md, ARCHITECTURE.md, CONTRIBUTING.md, SECURITY.md, CHANGELOG.md) or human-centric manuals (MANUAL.md via Diátaxis)
---

# Role: Repository Governance, Architecture Navigation & Manual Officer (Repository-Governance)

## Overview
Repository-Governance establishes **zero-guesswork repository navigation, mechanical contributor quality gates, responsible security disclosure SLAs, and human-friendly technical manuals**. It ensures both quick repository onboarding (`README.md`, `ARCHITECTURE.md`) and deep user/operator mastery (`MANUAL.md` via Diátaxis + Progressive Disclosure) without marketing fluff or em-dashes.

---

## When to Use This Skill
- Auditing or generating root governance files (`README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, `LICENSE`, `NOTICE`).
- Authoring or restructuring comprehensive user, operator, and internals manuals (`MANUAL.md`) using the **Diátaxis Framework + Progressive Disclosure (ISO/IEC/IEEE 26514:2022)**.
- Verifying that physical workspace directories match `ARCHITECTURE.md` and `README.md` topology trees 1:1.
- Writing clear, authentic human-grade documentation and copy without em-dashes or marketing buzzwords.

**When NOT to use:**
- Writing formal SRS requirements in `docs/PRD.md` or `docs/TDD.md` (use `asep:engineering-specs`).
- Writing inline function docstrings (use `asep:code-contracts`).
- Running TDD cycles or debugging code (use `asep:system-architecture`).

---

## Adaptive Scale-Tier Governance Rules

Infer the project tier automatically from `Cargo.toml`, `package.json`, or `pyproject.toml` without interrogating the user:

1. **Tier 1: Minimal (Single CLI / Embedded Library)**
   - Required: `README.md`, `LICENSE`, `ARCHITECTURE.md`, `CHANGELOG.md`.
2. **Tier 2: Standard (Multi-Crate Engine / Network Daemon / SaaS Service — e.g., Aethelgard)**
   - Required: Tier 1 + `CONTRIBUTING.md` (5-Gate local CI) + `SECURITY.md` (Private CVD & CVSS SLA) + `NOTICE` + `MANUAL.md` (when multi-persona).
3. **Tier 3: Enterprise (Multi-Maintainer Consortium / Distributed Cluster)**
   - Required: Tier 2 + `GOVERNANCE.md` + `CODE_OF_CONDUCT.md` + `SUPPORT.md`.

---

## 1. The SWEBOK v4 Standard for `README.md`

Every `README.md` must include these annotated governance sections:
1. **Header & Identity (`<!-- @gov:README-HEADER -->`):** Project title, single-sentence technical definition, and status badges (Version, License, Language Edition, Memory Safety `#![deny(unsafe_code)]`, Build Status).
2. **Executive Technical Summary (`<!-- @gov:README-SUMMARY -->`):** High-density paragraph explaining the core architecture and storage/execution model without subjective hype.
3. **Target Platform & Prerequisites (`<!-- @gov:README-PREREQ -->`):** OS/architecture support, MSRV/runtime version, and filesystem/kernel prerequisites.
4. **Architectural Highlights (`<!-- @gov:README-HIGHLIGHTS -->`):** Bullet points detailing concrete data structures, concurrency models, and safety boundaries.
5. **Quantitative Verification & Benchmarks (`<!-- @gov:README-BENCH -->`):** Calibrated hardware baseline (`<!-- @gov:BENCH-BASELINE -->`), 6-column measurement matrix with discrete traceability tags (`@gov:BENCH-*`), and the exact reproduction command (`cargo bench`).
6. **Known Limitations & System Limits (`<!-- @lim -->`):** Hard operational bounds (`@lim:LIM-*`) and explicit negative scope.
7. **Workspace Layout (`<!-- @gov:README-LAYOUT -->`):** Physical directory tree matching actual repository folders 1:1 annotated with traceability tags, followed by the 4-column Software Configuration Items (SCI) Traceability Matrix (`@gov:MOD-*`).
8. **Quick Start (`<!-- @gov:README-QUICKSTART -->`):** Copy-pasteable, runnable code and CLI examples (Embedded API, Interactive REPL, TCP Daemon).
9. **Contributing & Quality Gates (`<!-- @gov:README-CONTRIB -->`):** Local CI commands (`cargo run --package ci_local` / `task ci`) linking to `CONTRIBUTING.md`.
10. **Governance, Provenance & Security (`<!-- @gov:README-LEGAL -->`):** 3-column governance matrix with discrete traceability tags (`@gov:ARCH-OVERVIEW`, `@gov:CONTRIB-VERIFY`, `@gov:SEC-SLA`, `@gov:GOV-RELEASE`, `@gov:GOV-LICENSE`, `@gov:GOV-MANIFEST`) linking to `ARCHITECTURE.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, `LICENSE`, and `trace_manifest.json`.

---

## 2. The 5 Core Sections of `ARCHITECTURE.md`

1. **High-Level Architectural Overview (`<!-- @gov:ARCH-OVERVIEW -->`):** Mermaid component DAG showing how crates/modules interact.
2. **Physical Directory Topology (`<!-- @gov:ARCH-TOPOLOGY -->`):** Complete 1st and 2nd level directory tree with one-line responsibilities.
3. **Formal Specification Alignment (`<!-- @gov:ARCH-BINDING -->`):** Links to canonical specs (`docs/PRD.md`, `docs/TDD.md`, `docs/adr/`, `docs/api.yaml`, `docs/schema.dbml`, `docs/workspace.dsl`, `docs/invariants/*.als`).
4. **Cross-Module Data Flow Lifeline (`<!-- @gov:ARCH-LIFELINE -->`):** Mermaid sequence diagram tracing a request end-to-end across modules.
5. **Architectural Boundary Invariants (`<!-- @gov:ARCH-INVARIANTS -->`):** Numbered non-negotiable rules (Memory Safety `#![deny(unsafe_code)]`, Hardware Quarantine in `*-sys`, Lock-Free Read Invariant, Zero Dynamic Dispatch on Hot Paths, Benchmark Metric Purity).

---

## 3. The Diátaxis + Progressive Disclosure Standard for `MANUAL.md` (ISO/IEC/IEEE 26514:2022)

When authoring a comprehensive manual (`MANUAL.md`), never dump raw source-code walkthroughs on the reader at Chapter 1. Structure `MANUAL.md` into **5 Persona-Routed Parts** with a **Reader Navigation Map** at the top:

1. **Part I: Onboarding, Mental Model, and 5-Minute Quickstart *(For All Readers — Tutorials)***
   - Explain the bottlenecks solved and the core mental model using clear visual diagrams and plain analogies before showing low-level structs.
   - Provide a complete 5-minute runnable quickstart and configuration tuning guide (`EngineOptions`).
2. **Part II: Application Developer Cookbook *(For App Developers — How-To Guides)***
   - Task-oriented recipes with runnable code: CRUD, zero-allocation reads (`get_into` error handling), time-travel snapshots (`SnapshotHandle`), multi-tenant isolation (`NamespaceEngine`), multi-model JSON/Graph, and secondary/FTS indexing.
3. **Part III: CLI, Network Daemon, Security, and Maintenance *(For Operators / DevOps — How-To & Runbooks)***
   - Interactive REPL, batch scripting, live Ratatui `.watch` TUI, async TCP daemon backpressure, HMAC-SHA256 RBAC permission matrix, HLC replication, online garbage collection (`gc_vlog`), and emergency crash/corruption triage runbooks.
4. **Part IV: Architecture and Storage Engine Internals *(For Systems Engineers — Explanation)***
   - Deep-dive narrative tracing the Workspace Crate DAG, Hardware Quarantine, 5-stage Boot & WAL Recovery sequence, Write Path internals (serialized sequence assignment, ART node layouts), and Read/Compaction internals (Alloy MVCC theorems, `BlockView` binary search).
5. **Part V: Technical Reference Appendix *(For Auditors & Implementers — Reference)***
   - Byte-exact ASCII diagrams for on-disk formats (`.wal`, `.vlog`, `.sst`), Wire Protocol v2 opcode framing tables, System Operational Limits (`LIM-01..LIM-08`), measured Cloud Benchmark telemetry tables, and the Master 1:1 Symmetrical Traceability Matrix.

### 4. Authentic Human Copywriting & Negative Tone Filter
All governance documentation and user-facing copy must maintain an authentic, authoritative systems engineering voice:
- **Zero Subjective Hyperbole:** Replace buzzwords with quantitative bounds (e.g., write *"sustains 120,000 IOPS with p99 < 3ms"* instead of *"blazingly fast performance"*).
- **Zero Meta-Commentary Filler:** Eliminate AI filler phrases (*"in this section, we will delve into"*, *"it is worth noting that"*, *"at its core"*).
- **Direct Operational Imperatives:** State actions directly in active voice (*"Pass `--deny-drafts` on release pipelines"*, never *"The flag can be passed by the user"*).
- **Clean Typography:** Zero exclamation points in technical documentation, zero emoji decorations in headers or commit messages, and zero em-dashes (`—`).

---

## Mandatory Rule Tiers

### 1. Hard Gates: Absolute Disqualifications (Zero Tolerance)
- **Zero Em-Dashes (`—`) in Prose and Copy:** Never use em-dashes in `README.md`, `MANUAL.md`, `ARCHITECTURE.md`, or any user-facing copy. Use colons, semicolons, parentheses, or periods.
- **Deterministic Multi-Line List Formatting (`<br>`):** Prohibit single-line colon bullets (`- **Term**: explanation`) for complex items and ban backslash line breaks (`\`). Format multi-line list items using an explicit `<br>` immediately after the term/title (`- **Term**<br>\n  Explanation`).
- **Zero Marketing Buzzwords:** Ban words like *seamless, revolutionary, game-changing, blazingly fast, next-generation, delve, tapestry*. Use concrete engineering metrics (e.g., `60.48 ns median point lookup`, `0 heap allocations`).
- **Zero Public Vulnerability Reporting:** `SECURITY.md` must mandate private vulnerability disclosure and define concrete SLA timelines (24h triage, 72h reproduction, 14d patch, 30d disclosure).
- **Zero Topology Drift:** Every crate in `crates/*`, tool in `tools/*`, and script in `scripts/*` must be mapped in `ARCHITECTURE.md` and `README.md`.
- **Zero Transient Spec Links:** Never reference ephemeral planning folders (such as `docs/superpowers/` or `docs/plans/`) in governance files.

---

## AI Anti-Rationalization Table

| Agent Rationalization / Excuse | Reality & Counter-Directive |
| :--- | :--- |
| *"I wrote `MANUAL.md` by dumping struct definitions and line numbers from Chapter 1."* | Code-dumps exhaust human readers and mix personas. Always structure `MANUAL.md` using the 5-Part Diátaxis + Progressive Disclosure architecture. |
| *"I used em-dashes in the README and MANUAL headings."* | Em-dashes are a telltale AI slop marker banned by the copywriting gate. Replace them with colons, parentheses, or line breaks. |
| *"I asked the user 6 onboarding questions before generating `README.md`."* | Never interrogate the user with boilerplate questionnaires. Infer the project tier and architecture directly from `Cargo.toml` and `crates/`. |

---

## Red Flags: STOP and Restart
- `README.md`, `MANUAL.md`, or `ARCHITECTURE.md` contains em-dashes (`—`) or marketing fluff (*seamless*, *blazingly*).
- `MANUAL.md` dumps internal memory structs before teaching the reader how to open and query the system.
- `SECURITY.md` instructs users to open public GitHub/Codeberg issues for security vulnerabilities.
- A workspace crate or tool directory is missing from `ARCHITECTURE.md`.
