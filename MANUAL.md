# ASEP Manual: Architecture, Operations, and Engineering Reference

<!-- @gov:MANUAL-HEADER -->
> **Framework:** Diátaxis (Tutorials, How-Tos, Explanations, Reference) + Progressive Disclosure<br>
> **Standard:** ISO/IEC/IEEE 26514:2022 Systems and Software Engineering Documentation<br>
> **Ecosystem:** Axiomatic Systems Engineering Protocol (ASEP v2.1.0)

---

## Reader Navigation Map

| Your Role / Goal | Recommended Entrypoint | Key Sections |
| :--- | :--- | :--- |
| **New Engineer / Evaluator** | [Part I: Quickstart](#part-i-onboarding-mental-model-and-5-minute-quickstart) | SDLC mental model, 5-minute project setup, config tuning |
| **Feature Developer** | [Part II: Engineer Cookbook](#part-ii-engineer-cookbook) | Living requirements, Velocity Mode RAD, Axiomatic hardening |
| **DevOps / CI Maintainer** | [Part III: CLI Operations & Quality Gates](#part-iii-cli-operations-git-topology-and-quality-gates) | Dual-branch topology, 5-Gate CI pipeline, MCP stdio setup |
| **Systems Architect** | [Part IV: Engine Architecture Internals](#part-iv-engine-architecture-and-internal-lifelines) | AST scanner, bipartite closure graph, docstring contract (DbC) auditor |
| **Auditor / Tool Integrator** | [Part V: Technical Reference Appendix](#part-v-technical-reference-appendix) | Tag taxonomy, CLI flags reference, Criterion benchmarks |

---

## Part I: Onboarding, Mental Model, and 5-Minute Quickstart

### 1. The Core Mental Model
The Axiomatic Systems Engineering Protocol (ASEP) treats software requirements, architectural designs, production implementations, and verification suites as a single, mathematically closed bipartite graph. 

Instead of isolating specifications in external issue trackers or wiki pages that drift away from code:
- **Living Specifications:** Requirements exist directly inside version-controlled Markdown artifacts (`docs/PRD.md`) tagged with ISO 29148 metadata (`<!-- @req:FR-ID:PRIORITY Title -->`).
- **Closed-Loop Traceability:** Every mandatory requirement marked `MUST` must possess a 1:1 corresponding design mapping (`@design`), production interface (`// @impl`), and passing automated test (`// @test`).
- **Dual-Track SDLC:** ASEP balances rapid exploratory prototyping with extreme programming rigor through two discrete tracks:
  - **Track 1 (Velocity Mode):** Rapid discovery allowing in-progress exploratory requirements (`@req:*:DRAFT`) and relaxed docstring audits.
  - **Track 2 (Axiomatic Mode):** Zero-tolerance release gate requiring 100% graph closure, strict Meyer Design by Contract (DbC), and `--deny-drafts` enforcement.

### 2. Five-Minute Runnable Quickstart

Follow these steps to initialize and verify a project under ASEP:

```bash
# 1. Initialize repository scaffold
asep init .

# 2. Author requirement in docs/PRD.md
cat << 'EOF' > docs/PRD.md
# Product Requirements Document

<!-- \@req:FR-PAY-01:MUST Atomic balance deduction -->
## 1. Payment Settlement
The engine must atomically verify and deduct available balances.
EOF

# 3. Create design record in docs/TDD.md
cat << 'EOF' > docs/TDD.md
# Technical Design Document

<!-- \@design:FR-PAY-01 -->
## 1. Settlement Interface
Atomic deduction executed through Compare-And-Swap (CAS) state machine.
EOF

# 4. Implement interface in src/lib.rs
cat << 'EOF' > src/lib.rs
// \@impl:FR-PAY-01
pub fn deduct_balance(balance: &mut u64, amount: u64) -> Result<(), &'static str> {
    if *balance < amount {
        return Err("insufficient funds");
    }
    *balance -= amount;
    Ok(())
}
EOF

# 5. Write automated verification test in tests/payment_test.rs
cat << 'EOF' > tests/payment_test.rs
// \@test:FR-PAY-01
#[test]
fn test_deduct_balance() {
    let mut balance = 100u64;
    assert!(deduct_balance(&mut balance, 40).is_ok());
    assert_eq!(balance, 60);
    assert!(deduct_balance(&mut balance, 70).is_err());
}
EOF

# 6. Execute deterministic verification gate
asep check .
```

Expected verifiable output:
```text
============================================================
              ASEP SDLC Traceability Report                 
============================================================
Discovered 1 requirements across workspace.
------------------------------------------------------------
Summary (mode: axiomatic): 1 closed MUST | 0 broken MUST | 0 pending SHOULD | 0 DRAFT | 0 docstring contract violations

✔ Gate Result: PASSED (Exit Code 0)
```

### 3. Configuration and Ignore Patterns
Customize project traversal and editor preferences using persistent configuration files:
- **Ignore Rules (`.asepignore`):** Place `.asepignore` in the workspace root to exclude template directories, mock fixtures, or non-production generators (syntax matches `.gitignore`).
- **Editor Preferences (`.spec-engine/config.toml`):** Configure interaction modes for opening generated ADRs or diagrams:
  ```toml
  [editor]
  preference = "terminal" # Options: "ask", "terminal", "gui"
  custom_command = "vim"
  ```

---

## Part II: Engineer Cookbook

### Recipe 1: Authoring Living Requirements (ISO 29148 Grammar)
Requirements must adhere strictly to active modal verbs defining observable behavioral bounds:
- **Mandatory Requirements:**
  ```markdown
  <!-- \@req:FR-AUTH-01:MUST Constant-time password comparison -->
  The authentication service shall compare credential digests using constant-time evaluation.
  ```
- **Optional / Phase 2 Requirements:**
  ```markdown
  <!-- \@req:FR-NOTIF-02:SHOULD Multi-channel webhook notifications -->
  The event dispatcher should dispatch webhook payloads across retry queues.
  ```

### Recipe 2: Rapid Prototyping in Velocity Mode
During initial architectural discovery, enforce speed over completeness:
1. Tag exploratory requirements with `:DRAFT`:
   ```markdown
   <!-- \@req:FR-EXPLORE-01:DRAFT Streaming telemetry subscriber -->
   ```
2. Run local verification with `--mode velocity`:
   ```bash
   asep check . --mode velocity
   ```
   Active draft tags produce non-blocking informative notices without halting pipelines (Exit Code 0).

### Recipe 3: Hardening to Axiomatic Mode (In-Place Graduation)
Before merging feature branches into integration trunks, harden prototypes:
1. Promote draft tag to mandatory status: change `:DRAFT` to `:MUST`.
2. Add the corresponding `@design:FR-ID` in architecture models or ADRs.
3. Annotate public functions with `// @impl:FR-ID`.
4. Add automated test coverage with `// @test:FR-ID`.
5. Verify zero lingering drafts:
   ```bash
   asep check . --deny-drafts
   ```

### Recipe 4: Packaging and Distributing Agent Skills
Package the canonical ASEP skills across supported AI coding agent platforms:
```bash
# Install skills into current workspace (.agents/plugins/asep)
asep pack --local --agent antigravity -y

# Install skills globally into user configuration directory (~/.gemini/config/plugins/asep)
asep pack --global --agent antigravity -y

# Export individual skill to portable Markdown
asep skill export code-contracts ./contracts_skill.md
```

---

## Part III: CLI Operations, Git Topology, and Quality Gates

### 1. Dual-Branch Git Topology
ASEP mandates a two-trunk branching strategy:

```text
       (hotfix/*)
           ▲
           │
main  ────●────────────────────────●───────── (Axiomatic Trunk: zero-draft, strict CI)
           ▲                      ▲
           │ (harden / merge)     │ (release PR)
dev   ─────●───────●──────────────●────────── (Integration Trunk: drafts permitted)
                   ▲
                   │
         (feat/*, fix/*, perf/*)
```

- **`main` (Axiomatic Trunk):** Stable release branch. Direct commits are prohibited. All requirements must be closed `MUST` with 0 active drafts (`--deny-drafts`).
- **`dev` (Integration Trunk):** Primary collaboration trunk. Fast-track velocity merges containing `:DRAFT` tags are permitted for integrated staging.
- **Feature Branches:** Branched exclusively from `dev` using standard Conventional Commits (`feat/FR-[ID]`, `fix/FR-[ID]`, `perf/FR-[ID]`, `refactor/FR-[ID]`).

### 2. The 5-Gate CI Pipeline
All pull requests targeting `main` must pass automated validation with **Exit Code 0**:

```bash
# Gate 1: Symmetrical Traceability and Contract Gate
asep check . --deny-drafts

# Gate 2: Static Type & Memory Safety
cargo check --all-targets

# Gate 3: Zero Warnings Policy
cargo clippy --all-targets -- -D warnings

# Gate 4: Automated Test Suites
cargo test --all-targets

# Gate 5: Deterministic Code Formatting
cargo fmt --check
```

### 3. Model Context Protocol (MCP) Integration
Integrate `asep` into MCP-compliant IDEs and agent harnesses using stdio transport:
```bash
asep mcp
```
Exposes two canonical tools:
- `spec_check`: Runs comprehensive traceability and docstring contract audits over targeted directories with mode configuration.
- `adr_new`: Deterministically scaffolds sequentially numbered Architecture Decision Records.

---

## Part IV: Engine Architecture and Internal Lifelines

### 1. Subsystem Decomposition
The `spec-engine` core consists of five decoupled subsystems:
1. **AST Scanner & Lexer (`src/scanner/`):** Traverses the repository respecting `.gitignore` and `.asepignore`. Uses compiled regular expressions to extract structured `TagOccurrence` tokens while filtering string literals inside code files.
2. **Traceability Graph Builder (`src/graph/`):** Evaluates bipartite relations across requirements, designs, implementations, and test assertions. Resolves closure state without heap allocations on non-broken paths.
3. **Docstring Auditor (`src/auditor/`):** Evaluates public Rust items against Meyer DbC, flagging signature parroting and phantom docstring sections (`# Panics: None`, `# Errors: None`).
4. **Packaging Core (`src/pack/`):** Multi-agent installer generating validated `plugin.json` manifests and distributing canonical Markdown skills.
5. **MCP Server (`src/mcp/`):** Lightweight JSON-RPC 2.0 stdio server compliant with the Model Context Protocol.

### 2. Traceability Graph Resolution Lifeline

```text
[ Source Files / Markdown ]
           │
           ▼
┌───────────────────────┐
│     WalkBuilder       │ (Applies .gitignore and .asepignore)
└──────────┬────────────┘
           │
           ▼
┌───────────────────────┐
│      AST Lexer        │ (Extracts @req, @design, @impl, @test)
└──────────┬────────────┘
           │
           ▼
┌───────────────────────┐
│  build_trace_graph()  │ (Partitions into MUST, SHOULD, DRAFT)
└──────────┬────────────┘
           │
     ┌─────┴──────────────┐
     ▼                    ▼
[ is_closed: true ]   [ must_broken > 0 ]
  Exit Code 0           Exit Code 1
```

### 3. Concurrency and Scalability Standards (ARCH-11)

**Physical core targeting.** Compute, AST parsing, regex scanning, and SIMD pipelines target physical CPU cores (`num_cpus::get_physical()`). Logical threads (SMT/hyper-threads) are reserved strictly for I/O concurrency and latency hiding.

**Partitioned shared-nothing concurrency.** Per-thread accumulation buffers with final-phase reduction are preferred over shared mutable atomics to eliminate MESI cache-coherence bus invalidations.

**Scalability budget.** Parallel speedup is governed by Amdahl's Law with serial fraction $s \le 0.05$. Gustafson's weak scaling governs batch processing workloads.

---

## Part V: Technical Reference Appendix

### Appendix A: Traceability Metadata Taxonomy

| Tag Format | File Location | Engineering Purpose |
| :--- | :--- | :--- |
| `<!-- \@req:...:MUST -->` | `PRD.md` | Mandatory requirement; must be closed by `@impl` and `@test` |
| `<!-- \@req:...:SHOULD -->` | `PRD.md` | Optional requirement; warns in standard check, fails in `--strict` |
| `<!-- \@req:...:DRAFT -->` | `PRD.md` | Exploratory prototype; non-blocking in Velocity Mode, denied by `--deny-drafts` |
| `<!-- \@design:... -->` | `TDD.md` / `ADR.md` | Technical design and architectural interface mapping |
| `// \@impl:...` | Source code files | Production domain implementation boundary |
| `// \@test:...` | Test files (`tests/*`) | Automated test assertion verifying requirement closure |
| `\@adr:...` | ADR markdown files | Architectural decision record reference index |
| `<!-- \@gov:... -->` | Governance files | Repository governance policy markers and security SLAs |
| `\@lim:...` | Specifications | Hard operational bounds and system capacity ceilings |

### Appendix B: CLI Flag Reference

- `asep check [PATH]`:
  - `--mode <velocity|axiomatic>`: Selects active verification track (defaults to axiomatic).
  - `--deny-drafts`: Rejects any lingering `:DRAFT` tags with Exit Code 1.
  - `--strict`: Treats pending `SHOULD` requirements as hard pipeline failures.
  - `--json`: Emits raw machine-readable JSON output for automated CI processing.
- `asep pack`:
  - `--local`: Installs plugin in current workspace directory.
  - `--global`: Installs plugin in global user agent directory.
  - `--agent <antigravity|opencode|claude|cursor|zed>`: Targets specific AI coding assistant.
  - `--skills <list>`: Selective comma-separated skill filter.
- `asep adr new <TITLE>`: Scaffolds a new sequentially numbered ADR.
- `asep diagram <PATH>`: Compiles `.puml` PlantUML files into vector `.svg` diagrams.
- `asep mcp`: Boots the stdio Model Context Protocol daemon.

### Appendix C: System Capacity Limits
- `@lim:LIM-SCAN-01`: Directory traversal strictly respects `.gitignore` and `.asepignore`. Ignored build directories (`target/`, `node_modules/`) are never scanned.
- `@lim:LIM-MEM-01`: Line parsing buffer capped at 64 KB per line to protect against memory exhaustion from oversized binary lines.

### Appendix D: Micro-Benchmark Telemetry
Evaluated on calibrated Linux x86_64 baseline hardware (8 cores @ 3.2 GHz, 32 GB RAM, NVMe PCIe 4.0 storage) via Criterion:
- **AST Tag Lexing (10,000 lines):** 4.8 ms (p50), 5.1 ms (p99), Zero heap leaks.
- **Traceability Graph Resolution (500 relations):** 1.9 ms (p50), 2.1 ms (p99), Linear $O(V+E)$ space.
- **Docstring Contract Parsing (100 modules):** 0.9 ms (p50), 1.0 ms (p99), Bounded stack buffers.
