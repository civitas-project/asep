# Contributor Guidelines & Execution Pipeline

All contributions follow a deterministic execution pipeline to guarantee quality, traceability, and architectural contract compliance.

---

## 1. Development Environment Setup

- **Prerequisites:** Rust `1.80.0+` (Edition 2021), `git`, `cargo`.
- **Optional Tools:** `plantuml` and `graphviz` (for local PlantUML diagram rendering).
- **Workspace Verification:**
  ```bash
  cd spec-engine
  cargo test
  ```

---

## 2. Git Topology & Branch Naming Conventions

ASEP enforces a **Dual-Branch Integration Model** (`main` + `dev`) using pure Conventional Commits:

### Permanent Base Branches
- **`main` (Axiomatic Trunk):** Production-ready code. Protected; merges only from `release/<version>` via Release PRs. Enforces **zero `@req:*:DRAFT` tags** (`asep check --deny-drafts`) and 100% passing 5-Gate CI.
- **`dev` (Integration Trunk):** Active development trunk. All working branches originate from `dev` and merge back into `dev`.

### Dual-Speed Working Branches (Branch off `dev`)
Branch names follow Conventional Commits and map to the active SDLC mode:
- **Velocity Mode (RAD / Fast Track):**
  - **Branch:** `feat/<topic>` (example: `feat/vector-search`)
  - **PRD Tag:** `@req:FR-[MOD]-[ID]:DRAFT`
  - **Commits:** `feat(draft): scaffold initial vector search`
  - **Verification:** `asep check --mode velocity` (draft tags & docstring warnings are non-blocking).
- **Axiomatic Mode (Hardening / Rigorous Track):**
  - **Bug Fixes:** `fix/<topic>` (example: `fix/token-refresh-race`)
  - **Performance:** `perf/<topic>` (example: `perf/zero-alloc-buffer`)
  - **Refactoring:** `refactor/<topic>` (example: `refactor/isolate-storage`)
  - **Documentation:** `docs/<topic>` (example: `docs/governance-readme`)
  - **Tests:** `test/<topic>` (example: `test/harness-rss-bounds`)
  - **Maintenance:** `chore/<topic>` (example: `chore/update-dependencies`)
  - **Release:** `release/<version>` (example: `release/1.1.0`), cut from `dev`, PR to `main`, then tag `v<version>` on `main`. Back-merge `main` into `dev` only when `git rev-list --count dev..main` is greater than 0.
  - **In-Place Hardening:** On the existing `feat/<topic>` branch before merging into `dev`, graduate `@req:*:DRAFT` to `@req:*:MUST`, write failing TDD assertions (`// @test:FR-*`), and verify via `asep check --mode axiomatic`.

---

<!-- @gov:CONTRIB-VERIFY -->
## 3. Five-Gate Local Verification Pipeline

Contributors must execute the 5-gate local verification pipeline with **Exit Code 0** before submitting a Pull Request:

```bash
cd spec-engine

# Gate 1: Code Formatting
cargo fmt --check

# Gate 2: Strict Linter
cargo clippy -- -D warnings

# Gate 3: Unit & Integration Test Suite
cargo test --all-targets

# Gate 4: Doc-Tests & Contract Examples
cargo test --doc

# Gate 5: Traceability & Spec Compliance
cargo run --bin asep -- check .
```

Continuous integration pipelines will automatically reject PRs that fail formatting, linting, doc-tests, or any unit/integration test.

---

## 4. Commit Standards

Commit messages follow Conventional Commits format and reference traceability tags where applicable:

```text
feat([scope]): [imperative action description] [@req:FR-[MODULE]-[ID]]

- Explanation of operational behavior change
- Invariant or performance implications
```

Examples:
- `feat(pack): auto-detect all modern agentic AI software and default-check installed agents [@req:FR-PACK-TARGETS-01]`
- `fix(lexer): prevent buffer overrun on tags exceeding 64KB [@lim:LIM-MEM-01]`

---

## 5. Pull Request Lifecycle

1. Open a Pull Request targeting `dev`. Only `release/<version>` PRs target `main` (see section 2).
2. Ensure automated CI checks pass completely with Exit Code 0.
3. Verify that all functional requirements marked `MUST` contain matching `@impl` and `@test` tags.
4. PRs require review and approval from at least one repository maintainer prior to merge.
