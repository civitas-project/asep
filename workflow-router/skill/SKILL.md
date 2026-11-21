---
name: workflow-router
description: Use at the start of any task to route intent, sequence phases, and orchestrate skills across the ASEP (Axiomatic Systems Engineering Protocol) ecosystem
version: 2.2.0
depends: [system-architecture, engineering-specs, code-contracts, repository-governance, design-tokens, spec-engine]
---

# Workflow Router: Master Agent Dispatch & Skill Orchestrator

## Overview

Workflow Router is the unified decision engine for selecting, sequencing, and executing engineering skills across the **ASEP (Axiomatic Systems Engineering Protocol)** ecosystem. It combines **Spec-Driven Extreme Programming (FXP)** with **Design by Contract (DbC)**, **Formal Verification**, and **Zero-Slop Craftsmanship** using five self-contained domain skills plus MCP tools from spec-engine:

1. **`asep:system-architecture`:** 2-Phase SDLC orchestration (Phase 1 Exploratory Spike vs. Phase 2 Production TDD), vertical slice planning, systematic root-cause debugging, Rich ADRs, Alloy/TLA+ formal proofs, and the 5-Gate CI verification pipeline.
2. **`asep:engineering-specs`:** ISO/IEC/IEEE 29148 requirements grammar (`PRD.md`), technical designs (`TDD.md`), Architecture-as-Code (`schema.dbml`, `api.yaml`, `workspace.dsl`), vector SVG PlantUML/Mermaid diagrams, zero-slop technical prose, and reproducible cloud benchmark publication (`kaggle_bench.py` + Typst).
3. **`asep:code-contracts`:** Low-level function docstrings, strict conditionality (zero ghost sections, zero signature parroting), memory ownership (`Zero-Allocation`, `Caller-Owned`), concurrency lock hierarchies, and doctest verification.
4. **`asep:repository-governance`:** SWEBOK v4 `README.md`, 5-part `ARCHITECTURE.md`, Diátaxis + Progressive Disclosure `MANUAL.md` (ISO/IEC/IEEE 26514:2022), `CONTRIBUTING.md`, private CVD `SECURITY.md`, and authentic human copywriting.
5. **`asep:design-tokens`:** W3C DTCG / KDL design token contracts (`.tokens.kdl`), perceptual OKLCH color math with sRGB `fallbackHex`, mandatory unit invariance, WCAG AA/AAA contrast, interactive state modifiers, and 44px ergonomic tap targets.
6. **`asep:spec-engine` (MCP):** Semantic source indexer (`srcidx`), token-efficient source dumper (`srcdump`), traceability validator (`asep_check`), ADR scaffolder (`adr_new`), diagram renderer (`render_diagrams`).

---

## When to Use This Skill

- At the start of every task or user request to determine the exact execution path.
- When facing new features or technical unknowns to decide between **Phase 1: Exploratory Spike** and **Phase 2: Production TDD**.
- When diagnosing bugs, test failures, or performance regressions before touching production code.
- When coordinating multi-step tasks across specifications, implementation, documentation, and quality gates.

---

## The Master Dispatch Matrix

Use this matrix to route any engineering intent exclusively to internal ASEP skills:

| Intent / Trigger Situation | Primary Skill | Supporting Skills | First Mandatory Action |
| :--- | :--- | :--- | :--- |
| **Rapid prototyping, PoC, or mass feature exploration** | `asep:system-architecture` | `asep:engineering-specs` | Enter **Velocity Mode (Track 1 / RAD)** on branch `feat/<topic>`. Scaffold PRD with `@req\:*:DRAFT` and code with `// \@impl:*`. Run `asep check --mode velocity`. |
| **Graduating a draft feature to production ready** | `asep:system-architecture` | `asep:code-contracts`, `asep:engineering-specs` | Enter **Graduation Gate**: upgrade `@req\:*:DRAFT` to `@req\:*:MUST`, write failing automated unit tests (`@test`), audit DbC contracts, and verify via `asep check --mode axiomatic`. |
| **Defined feature, behavior change, or vertical slice implementation** | `asep:system-architecture` | `asep:engineering-specs`, `asep:code-contracts` | Enter **Axiomatic Mode (Track 2 / Production TDD)**. Update `@req\:*:MUST` and `@design` slice, write failing `@test` (`RED`), then minimal `@impl` (`GREEN`). |
| **Core invariant, lock-free concurrency, or state machine (`@criticality:high`)** | `asep:system-architecture` | `asep:engineering-specs` | Author formal model in `docs/invariants/*.als` (Alloy) or `*.tla` (TLA+) and verify zero counterexamples (`UNSAT`). |
| **Bug, test failure, crash, corruption, or unexpected behavior** | `asep:system-architecture` | `asep:code-contracts` | Enter **Systematic Debugging** on branch `fix/<topic>`. Isolate root cause, write a failing regression test reproducing the bug (`RED`), then fix (`GREEN`). |
| **Authoring or auditing PRDs, TDDs, ADRs, or UML/Mermaid diagrams** | `asep:engineering-specs` | `asep:system-architecture`, `asep:design-tokens` | Load document skeleton, enforce ISO 29148 active-voice grammar, and compile diagrams exclusively to vector `.svg`. |
| **Understanding codebase structure, symbol extraction, or cross-referencing** | `asep:system-architecture` | `asep:spec-engine` (MCP: `srcidx`) | Invoke `srcidx` via MCP with `mode: "symbols-only"` or `mode: "signatures"` to extract semantic symbols (functions, structs, traits) across Rust, TypeScript, Python, Dart, Go, PHP, Java, Typst. |
| **Preparing source context for LLM consumption, code archaeology, or token-efficient dumping** | `asep:system-architecture` | `asep:spec-engine` (MCP: `srcdump`) | Invoke `srcdump` via MCP with `mode: "summary"` or `mode: "signatures"`, `format: "ndjson"`, `estimate_tokens: true` to stream token-efficient source for LLM context. |
| **Academic research benchmarking, ablation studies, or paper tables** | `asep:engineering-specs` | `asep:system-architecture` | Execute isolated cloud benchmarks (`scripts/kaggle_bench.py`), verify noise-floor gates, and generate Typst tables and SVG figures. |
| **Writing or auditing function docstrings, error contracts, or unsafe boundaries** | `asep:code-contracts` | `asep:system-architecture` | Apply Strict Conditionality (omit empty sections, eliminate signature parroting, document `# Invariants`, `# Errors`, `# Panics`). |
| **Creating or updating README, MANUAL, ARCHITECTURE, CONTRIBUTING, or SECURITY** | `asep:repository-governance` | `asep:engineering-specs` | Apply SWEBOK v4 10-point README, 5-part `ARCHITECTURE.md`, or Diátaxis + Progressive Disclosure `MANUAL.md`. |
| **Authoring design tokens, UI color palettes, themes, or layout dimensions** | `asep:design-tokens` | `asep:engineering-specs` | Define KDL tokens with explicit units (`px`, `rem`, `ms`), OKLCH coordinates with `fallbackHex`, and WCAG contrast bounds. |
| **Claiming a task, bug fix, or milestone is complete** | `asep:system-architecture` | `asep:code-contracts`, `asep:repository-governance` | Execute the **5-Gate Local CI Pipeline** (`cargo run --package ci_local` or native equivalent) in terminal and inspect real output. |

---

## Operational Workflow Pipeline

```text
                       [User Request Received]
                                  │
                                  ▼
         ┌─────────────────────────────────────────────────┐
         │ 0. WORKING BRANCH (git)                         │
         └────────┬────────────────────────────────────────┘
                  │
                  ▼
         ┌─────────────────────────────────────────────────┐
         │ 1. CLASSIFY INTENT                              │
         └────────┬───────────────────────────────┬────────┘
                  │                               │
                  ▼                               ▼
         ┌─────────────────────────────────────────────────┐
         │ 1.5 SOURCE ANALYSIS (MCP)                       │
         └────────┬───────────────────────────────┬────────┘
                  │                               │
       [New Feature / Change]            [Bug / Test Failure]
                  │                               │
       Need codebase context?            Need root cause context?
       ├── YES ──► Invoke `srcidx`           ├── YES ──► Invoke `srcidx`
       │           `mode: "symbols-only"`    │           `mode: "signatures"`
       │           or `srcdump`              │           or `srcdump`
       │           `mode: "summary"`         │           `mode: "summary"`
       │                 │                   │                 │
       └── NO ◄──────────┘                   └── NO ◄──────────┘
                  │                               │
                  ▼                               ▼
     Is technical feasibility known?    Invoke `asep:system-architecture`
     ├── NO  ──► PHASE 1: SPIKE         (Phase 2B: Systematic Debugging)
     │           • Throwaway prototype    1. Reproduce & find root cause
     │           • Zero @req/@impl tags   2. Write failing regression test (RED)
     │           • Prove feasibility      3. Apply minimal fix (GREEN)
     │                 │
     └── YES ◄─────────┘
          │
          ▼
     PHASE 2A: PRODUCTION TDD (`asep:system-architecture`)
     ├── Step 1: Vertical Slice Spec (`asep:engineering-specs`)
     │           • Add/update `@req\:FR-*:MUST` in `docs/PRD.md`
     │           • Add/update `@design\:FR-*` in `docs/TDD.md` & `docs/adr/`
     │           • If `@criticality:high`, verify Alloy (`.als`) / TLA+ proof
     ├── Step 2: TDD RED
     │           • Write failing test with `// @test\:FR-*`
     │           • Run test runner to confirm expected failure
     ├── Step 3: TDD GREEN
     │           • Implement minimal production logic with `// @impl\:FR-*`
     │           • Enforce `#![deny(unsafe_code)]` outside hardware quarantine
     ├── Step 4: TDD REFACTOR & CONTRACTS
     │           • Clean up duplication
     │           • Apply `asep:code-contracts` (zero parroting, strict conditionality)
     │           • Update `README.md`, `ARCHITECTURE.md`, or `MANUAL.md`
     │             via `asep:repository-governance` if public surface changed
     └── Step 5: 5-GATE VERIFICATION (`ci_local` / `asep_check`)
                 ├── Gate 0: Formal Specs (DBML, OpenAPI, Structurizr, PlantUML SVG)
                 ├── Gate 1: 1:1 Symmetrical Traceability (`asep_check trace`)
                 ├── Gate 2: Zero-Warning Linter (`clippy -D warnings`)
                 ├── Gate 3: Native Test Suite, Doctests & Seeded Fuzz Smoke
                 ├── Gate 4: Continuous Performance & Noise-Floor Budget Gate
                 │
     ┌───────────┘
     └── Step 6: Pull Request
                 • Push the branch and open a PR to `dev`. Only a `release/<version>` branch opens a PR to `main` (Sequencing Rule 7).
                 • Do not merge until Gates 0-4 pass with Exit Code 0
```

---

## Sequencing and Precedence Rules

1. **Architecture & Process Precede Implementation:**
   - Never jump straight into writing production code. Route through `asep:system-architecture` first to establish whether you are in Phase 1 (Spike), Phase 2A (Vertical Slice TDD), or Phase 2B (Systematic Debugging).
2. **Test-First (TDD) is Non-Negotiable in Phase 2:**
   - Every production feature or bug fix requires observing a failing automated test (`RED`) before writing or modifying the implementation (`GREEN`).
3. **1:1 Symmetrical Traceability Closure:**
   - Every `@req\:FR-*:MUST` in `docs/PRD.md` must link to `@design\:FR-*` in `docs/TDD.md`, `// @impl\:FR-*` in production code, and `// @test\:FR-*` in automated tests, verified by `asep_check trace` (`trace_manifest.json`).
4. **Zero-Slop Craftsmanship Across Code and Prose:**
   - **Prose (`asep:engineering-specs` & `asep:repository-governance`):** Zero em-dashes, zero poetic AI metaphors (*weave, beacon, explore*), zero unquantified buzzwords (*smooth, blazingly fast*), zero single-line colon bullets (`- **Title**: text`); use `<br>` for multi-line list separation.
   - **Code (`asep:code-contracts`):** Zero signature-parroting comments, zero phantom docstring sections (`# Panics: None`).
   - **Design (`asep:design-tokens`):** Zero dimensionless numbers, zero unclipped wide-gamut colors without `fallbackHex`.
5. **Empirical Terminal Verification Before Completion:**
   - Never declare a task finished based on mental execution. Always run the verification command in the terminal and inspect the actual output.
6. **Sovereign & Self-Contained Ecosystem:**
   - All engineering capabilities (interactive brainstorming, requirements elicitation, task slicing, TDD, systematic debugging, comment hygiene, human copywriting, and accessible design tokens) are natively integrated into the 6 internal `asep:*` skills. External plugins are never required or invoked within ASEP-governed repositories.
7. **Git Topology & Branching:**
   - Every change lives on a branch cut from `dev`. Never commit directly to `dev` or `main`.
   - Branch prefix follows the commit type:
     - `feat/<topic>`: new behavior (`feat:` commits)
     - `fix/<topic>`: bug fixes (`fix:` commits)
     - `perf/<topic>`: performance work (`perf:` commits)
     - `refactor/<topic>`: restructuring without behavior change (`refactor:` commits)
     - `docs/<topic>`: documentation only (`docs:` commits)
     - `test/<topic>`: tests only (`test:` commits)
     - `chore/<topic>`: maintenance and tooling (`chore:` commits)
     - `release/<version>`: release cut from `dev` (`chore(release):` commits)
   - Merge to `dev` only through a Pull Request. Only `release/<version>` may open a PR to `main`.
   - Release procedure:
     1. Cut `release/<version>` from `dev` (example: `release/1.1.0`).
     2. Allowed on the release branch: version bump, changelog, and small fixes. No new features.
     3. Open a PR from `release/<version>` to `main`. This is the only PR that targets `main`.
     4. After the merge, create an annotated tag `v<version>` on `main` (example: `v1.1.0`).
     5. Run `git rev-list --count dev..main`. If the count is 0, stop. If it is greater than 0, back-merge: cut `chore/back-merge-<version>` from `dev`, merge `main` into it, and open a PR to `dev`.

---

## MCP Tool Invocation Pattern

Agents operating under workflow-router **must** invoke spec-engine MCP tools for source analysis tasks. The pattern:

### Discovery
1. Workflow-router loads `spec-engine` skill (declared in `depends`)
2. MCP server exposes `srcidx` and `srcdump` tools via stdio
3. Agent calls tools through workflow-router's MCP client interface

### Invocation Examples

**Symbol extraction (srcidx):**
```json
{
  "tool": "srcidx",
  "arguments": {
    "dir": "/path/to/repo",
    "mode": "symbols-only",
    "include_ext": ["rs", "ts", "py"],
    "strip_comments": true,
    "include_symbols": true
  }
}
```

**Token-efficient dump (srcdump):**
```json
{
  "tool": "srcdump",
  "arguments": {
    "dir": "/path/to/repo",
    "mode": "summary",
    "format": "ndjson",
    "tokenizer": "cl100k_base",
    "estimate_tokens": true,
    "max_lines": 500,
    "summary_threshold": 1000
  }
}
```

### When to Use Which

| Task | Tool | Mode | Why |
|------|------|------|-----|
| Find all public APIs | `srcidx` | `symbols-only` | Minimal token cost, semantic symbols only |
| Understand call graph | `srcidx` | `signatures` | Function signatures + types |
| LLM context prep | `srcdump` | `summary` | Token-budgeted, truncated large files |
| Full code archaeology | `srcdump` | `full` | Complete source with metadata |
| Token counting | `srcdump` | `signatures` + `estimate_tokens` | Pre-flight context budget |

### Integration with Traceability

- `srcidx` output can feed `@design` tag generation (symbol locations → design references)
- `srcdump` output can feed `@impl` tag verification (source context → implementation coverage)
- Both tools respect `.gitignore` and `.asepignore` by default

---

## AI Anti-Rationalization Table

| Agent Rationalization | Reality & Routing Directive |
| :--- | :--- |
| *"This change is trivial, so I do not need to check the router."* | Unrouted "trivial" edits cause traceability drift and broken contracts. Always classify the task via the Master Dispatch Matrix. |
| *"I will write the implementation first and add the test right after."* | Tests written after implementation are biased and never verified to catch failures. Follow `asep:system-architecture` Phase 2 TDD (`RED` then `GREEN`). |
| *"I can fix this bug immediately by changing this line."* | Guessing fixes masks deeper invariant bugs. Follow Phase 2B Systematic Debugging: locate root cause and write a failing regression test first. |
| *"I need to write a big 50-page specification before writing any code."* | Spec-Driven XP uses incremental vertical slices. Specify and implement one requirement slice (`FR-[MOD]-[ID]`) at a time. |
| *"I mentally verified that the tests and traceability pass."* | Mental execution is hallucination-prone. Run `asep_check trace` and the native test suite in the terminal. |

---

## Appendix: Lead-Judge-Executor Protocol (ARCH-06)

Bounded decomposition cycle for the executor-judge pipeline:

1. **Lead (primary mode):** Decomposes the task into bounded, verifiable slices. Owns the spec, the `@req`/`@design` tags, and the failing test. Defines done criteria and the evidence the judge must check: task requirements, produced diff, test results.
2. **Executor (subagent mode):** Implements the minimal change to pass the slice. Stays within bounds set by the lead; no scope creep, no unrelated refactors.
3. **Judge (subagent mode, read-only):** Verifies executor output against the task, the diff, and test evidence. Read-only: it may not modify files. Passes the slice or returns it to the executor with specific failures.
4. **Explorer (subagent mode):** Fast codebase exploration for symbol extraction and context gathering; feeds findings back to the lead.

Cycle: lead plans, executor implements, judge verifies, repeat until every slice passes. The judge's read-only gate prevents unverified code from being accepted.

`opencode.json` presets (scaffolded by `asep::pack::manifest::scaffold_opencode_agents`):

- `lead`: description "Orchestrator for the bounded executor-judge workflow.", mode "primary"
- `executor`: description "Fast coding executor in a bounded executor-judge pipeline.", mode "subagent"
- `judge`: description "Read-only reviewer verifying executor output against task, diff, and test evidence.", mode "subagent"
- `explorer`: description "Fast explorer subagent for codebase exploration.", mode "subagent"

---

## Requirement and Plan Phase (Lead-owned)

For any new feature, the lead performs two phases before any executor is dispatched:

1. **Requirement phase (brainstorming).** If the request leaves something material unsettled, ask the user ONE decisive question. Write the agreed behavior into `docs/PRD.md` as `@req:*:MUST` requirements tied to observable behavior. Do not invent requirements to make a gate green.
2. **Plan phase.** Write the design and the ordered implementation plan into `docs/TDD.md`: `@design:` entries, then per-task files/symbols, acceptance criteria, and verification commands. Stop and get explicit user approval before dispatching any executor.

Execution then follows the Lead-Judge-Executor Protocol above: one bounded task per executor, fresh judge review of each diff, and fresh verification by the controller.

Hard rules:
- `docs/PRD.md`, `docs/TDD.md`, and `docs/TRACEABILITY.md` are the only specification artifacts.
- Never create .superpowers/ or docs/superpowers/, and never add a new skill folder for this pattern.
- The lead owns PRD/TDD; executors never edit them.
- A task without a file path, a symbol, acceptance criteria, and a verification command is not delegable.

## Targeted Verification

Run `asep affected <changed-paths>` before dispatching verification. It reads the traceability graph and returns the exact test files, requirements, and commands to run. Use that output as the verification command; do not run the full suite by default. Full suite is reserved for merges to `main` or an explicit user request.

## Canonical Anti-Slop Rules

Canonical anti-slop rules live in `spec-engine/policy/asep-policy.toml`; skills reference rule IDs and never restate their patterns. This skill routes anti-slop checks by domain to `asep:engineering-specs` (AS-DESIGN-VAGUE, AS-DESIGN-TBD, AS-DOCS-PLACEHOLDER, AS-DOCS-WORDY), `asep:code-contracts` (AS-CODE-BANNER, AS-CODE-NARRATION, AS-CODE-BUZZWORD, AS-CODE-BOXDRAW, AS-CODE-NESTING), `asep:repository-governance` (AS-COPY-FILLER, AS-COPY-HYPE, AS-DOCS-WORDY, AS-COPY-BUZZWORD), and `asep:design-tokens` (AS-COPY-HYPE, AS-DOCS-PLACEHOLDER). `asep check --rigor T1` reports Warning rules and blocks on Error rules outside T0; `--strict` promotes Warnings. Intentional fixtures may use the inline marker `asep:allow-antislop`.

## Canonical Rule References

Canonical ASEP policy lives in `spec-engine/policy/asep-policy.toml`: coverage rules `COV-01`..`COV-06`, rigor tiers `Tier T0`..`Tier T4`, and exit codes `0` (pass), `1` (fail), `2` (config I/O), `3` (unproven). Skills reference rules by ID; they never restate them.

This skill applies COV-01`..`COV-06 (all coverage rules) and Tier T0..T4 (all tiers); unproven results surface as exit 3.

Existing agents and top-level keys in `opencode.json` are preserved; missing defaults are inserted only when absent.
