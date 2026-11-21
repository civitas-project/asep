---
name: engineering-specs
description: Use when authoring or auditing PRD/TDD specifications, ISO 29148 requirements grammar, Architecture-as-Code contracts, vector SVG diagrams, or cloud benchmark reports
---

# Role: Principal Technical Author & Systems Specification Auditor (Engineering-Specs)

## Overview
Engineering-Specs enforces mathematical precision, deterministic SRS grammar (ISO/IEC/IEEE 29148:2018), Architecture-as-Code contracts, and zero-slop technical writing across **engineering specifications and research benchmarks**. It ensures every requirement is computable, bounded, and verifiable.

---

## When to Use This Skill
- Authoring or auditing Product Requirement Documents (`docs/PRD.md`), Technical Design Documents (`docs/TDD.md`), Use Cases (`docs/USE_CASES.md`), Runbooks (`docs/runbooks/*.md`), and ADRs (`docs/adr/*.md`).
- Formulating functional requirements with mathematical invariants, happy paths, and explicit failure modes.
- Authoring Architecture-as-Code contracts: DBML schemas (`docs/schema.dbml`), OpenAPI wire contracts (`docs/api.yaml`), Structurizr C4 models (`docs/workspace.dsl`), Statecharts (`docs/statecharts/*.json`), and Alloy proofs (`docs/invariants/*.als`).
- Rendering formal PlantUML diagrams (`docs/diagrams/*.puml`) to vector SVG (`docs/diagrams/rendered/*.svg`) and macro Mermaid flowcharts.
- Orchestrating reproducible cloud benchmarks (`scripts/kaggle_bench.py`, `scripts/cloud_runner.py`) and generating publication-grade Typst tables (`*.typ`) and vector SVG charts (`scripts/formatter.py`).

**When NOT to use:**
- Orchestrating TDD cycles, debugging root causes, or managing lock hierarchies (use `asep:system-architecture`).
- Writing inline source code docstrings (use `asep:code-contracts`).
- Writing user-facing onboarding manuals (`MANUAL.md`) or root `README.md` files (use `asep:repository-governance`).

---

## The Engineering Authoring Standard

### 0. Pre-Specification Discovery & Interactive Brainstorming
Before authoring or modifying formal requirements in `docs/PRD.md`:
1. **Interactive Operational Interview:** When requirements or architecture are ambiguous, interview the user directly to clarify operational goals, throughput/latency budgets, and runtime constraints.
2. **Multi-Option Trade-Off Exploration:** Formulate 2 to 3 distinct implementation alternatives (e.g., Option 1: Minimal/Direct, Option 2: Modular/Extensible, Option 3: Lock-Free High-Throughput).
3. **Comparative Evaluation Matrix:** Present trade-offs across Complexity, Latency, Memory Footprint, and Maintenance Overheads.
4. **Explicit Alignment:** Obtain user consensus on the chosen approach before freezing identifiers into `@req\:FR-*`.

### 1. Canonical SRS Requirement Anatomy (ISO/IEC/IEEE 29148:2018)
Every requirement block in `docs/PRD.md` must include:
1. **Traceability Header & Lifecycle Status:**
   - **`@req\:FR-[MOD]-[ID]:DRAFT` (Velocity Mode / RAD):** Rapid exploration, PoC, and mass feature development. Non-blocking in `asep check --mode velocity`.
   - **`@req\:FR-[MOD]-[ID]:MUST` (Axiomatic Mode / Production):** Hardened requirement enforcing 1:1 symmetrical closure across `@design`, `// @impl`, and `// @test`.
   - **`@req\:FR-[MOD]-[ID]:SHOULD` (Backlog):** Non-blocking secondary requirement.
   - Example: `<!-- [@req\:FR-[MOD]-[ID]:DRAFT] [@criticality:high|low] -->`
2. **Active-Voice Statement:**
   $$\langle\text{System Subject}\rangle + \langle\text{must} \mid \text{shall}\rangle + \langle\text{Active Verb}\rangle + \langle\text{Object}\rangle + [\text{Quantitative Constraint}]$$
   - **Pass:** `The NamespaceEngine must isolate multi-tenant storage operations using 4-byte big-endian tenant key prefixes and enforce bounded range scan termination within the tenant keyspace.`
   - **Fail:** `Multi-tenant data should be seamlessly isolated by the system.`
3. **Mathematical Invariant:** Explicit LaTeX bounds, complexity ceilings, or state equations (e.g., $$len(key) \le 256\text{ bytes}$$).
4. **Happy Path:** Numbered deterministic execution steps.
5. **Edge Cases and Failure Modes:** Explicit error variants returned for every boundary violation (e.g., `EngineError::KeyTooLarge`).

### 2. Architecture-as-Code Binding
Specifications must not rely on prose alone; bind `@design\:FR-*` tags into computable artifacts:
- **Relational / Catalog Schema:** `docs/schema.dbml`
- **Network / Wire Protocol Contract:** `docs/api.yaml` (using `x-asep-design: "FR-*"`)
- **Macro C4 Topology:** `docs/workspace.dsl`
- **Lifecycle State Machine:** `docs/statecharts/engine.json`
- **Formal Logic Invariant:** `docs/invariants/*.als`

### 3. Dual-Visual Diagram Routing & OKLCH Token Palette
- **Formal UML (PlantUML in `docs/diagrams/*.puml`):** Compile **exclusively to vector SVG (`docs/diagrams/rendered/*.svg`)** for Use Case, Activity, Sequence BCE (Denert 1991), and Class diagrams.
- **Raster Prohibition:** Never commit `.png`, `.jpg`, or `.webp` files to `docs/diagrams/rendered/`.
- **Web-Native Macro Flows (Mermaid in Markdown):** Use for high-level architecture overviews and sequence lifelines inside Markdown files.

#### Standardized PlantUML OKLCH / sRGB Palette (WCAG AAA Text Contrast)
All PlantUML diagrams must declare explicit `skinparam` blocks mapped from `asep:design-tokens` against the dark canvas (`#1E1E2E`):

| Token Name | Semantic Role | OKLCH $(L, C, H)$ | `fallbackHex` | Contrast vs `#1E1E2E` |
| :--- | :--- | :--- | :--- | :--- |
| `surface-canvas` | Diagram root background | `oklch(0.23, 0.03, 275)` | `#1E1E2E` | 1.00:1 (Base) |
| `surface-crust` | System boundary container | `oklch(0.17, 0.02, 275)` | `#11111B` | 12.97:1 (Text AAA) |
| `surface-mantle` | Node & participant fill | `oklch(0.20, 0.02, 275)` | `#181825` | 12.14:1 (Text AAA) |
| `surface-overlay` | Actor & stereotype fill | `oklch(0.32, 0.03, 275)` | `#313244` | 8.69:1 (Text AAA) |
| `surface-highlight` | Lifeline activation bar | `oklch(0.40, 0.03, 275)` | `#45475A` | 1.80:1 (Bar fill) |
| `text-primary` | Default font & labels | `oklch(0.88, 0.03, 275)` | `#CDD6F4` | 11.34:1 (AAA) |
| `accent-actor` | Actor border & arrows | `oklch(0.76, 0.12, 248)` | `#89B4FA` | 7.79:1 (AAA) |
| `accent-boundary` | Boundary & activity border | `oklch(0.86, 0.14, 142)` | `#A6E3A1` | 11.03:1 (AAA) |
| `accent-control` | Control & diamond border | `oklch(0.90, 0.11, 88)` | `#F9E2AF` | 12.91:1 (AAA) |
| `accent-entity` | Entity & data model border | `oklch(0.76, 0.16, 298)` | `#CBA6F7` | 8.07:1 (AAA) |
| `accent-database` | Database storage border | `oklch(0.80, 0.13, 50)` | `#FAB387` | 9.27:1 (AAA) |

### 4. Reproducible Cloud Benchmarking & Publication Pipeline (`FR-BENCH-*`)
When benchmarking or preparing empirical performance telemetry:
1. **Isolated Cloud Execution (`scripts/kaggle_bench.py` & `scripts/cloud_runner.py`):** Execute Criterion suites on reproducible remote hardware, recording CPU model, core count, RAM, and OS kernel in `benchmark_summary.json`.
2. **Statistical Metric Purity:** Parse only `base/estimates.json` and `new/estimates.json`; strictly ignore relative delta directories (`change/estimates.json`) to prevent uncalibrated metric corruption.
3. **Publication Formatting (`scripts/formatter.py`):**
   - Generate vector SVG charts (`figures/*.svg`) with $\ge 30\%$ vertical y-axis headroom (`max_val * 1.30`) and stripped top/right spines so data annotations never clip.
   - Generate native Typst tables (`tables/*.typ`) with `#let format_ns(val)` dynamic unit conversion (`ns`, `µs`, `ms`).
   - Extract artifacts into `benchmarks/results/<YYYY-MM-DD_HH-MM-SS>/` and update the `benchmarks/results/latest` symlink atomically.

---

## Mandatory Rule Tiers

### 1. Hard Gates: Absolute Disqualifications (Zero Tolerance)
- **Zero AI Slop Vocabulary Blacklist:** Prohibit buzzwords, filler, and poetic metaphors (see the zero-tolerance slop vocabulary list in policy).
- **Zero Em-Dashes in Specifications:** Do not use em-dashes in specification prose; use colons, semicolons, parentheses, or separate sentences.
- **Deterministic Multi-Line List Formatting (`<br>`):** Prohibit single-line colon bullets (`- **Title**: description`) for complex items and ban backslash line breaks (`\`). Format multi-line list items using an explicit `<br>` immediately after the title (`- **Title**<br>\n  Description`).
- **No Passive-Voice Actor Omission:** Every requirement must name the exact subsystem or actor performing the action.
- **Zero Raster Diagram Assets:** Only `.svg` files are permitted in `docs/diagrams/rendered/`.
- **Zero Unstyled BCE Stereotypes:** Sequence diagrams must explicitly style `Actor`, `Boundary`, `Control`, `Entity`, and `Database` skinparams alongside `#45475A` activation bars.
- **Criterion Delta Quarantine:** Benchmark parsers must never ingest `change/estimates.json` as latency values.

---

## AI Anti-Rationalization Table

| Agent Rationalization / Excuse | Reality & Counter-Directive |
| :--- | :--- |
| *"I used 'seamlessly' to make the architecture overview flow better."* | "Seamlessly" hides missing error-handling contracts. State the exact protocol, timeout, or fallback behavior. |
| *"I only set `ParticipantBorderColor` in PlantUML."* | PlantUML renders unstyled `boundary`, `control`, and `entity` stereotypes with jarring default colors unless each stereotype skinparam is explicitly declared. |
| *"I parsed all `estimates.json` files in `target/criterion/` with `rglob`."* | Criterion writes relative percentage deltas into `change/estimates.json`. Ingesting `change/` corrupts nanosecond latency tables with negative fractions. |
| *"I used an em-dash to connect two clauses in the PRD."* | Em-dashes are banned by the prose hygiene gate. Use a colon, semicolon, or period. |

---

## Red Flags: STOP and Restart
- A requirement statement lacks a clear system subject, active verb, or failure mode.
- The text contains blacklisted slop vocabulary or em-dashes.
- A raster image (`.png`, `.jpg`) is generated instead of a vector `.svg`.
- A benchmark chart clips bar labels at the top border due to missing 30% y-axis headroom.
