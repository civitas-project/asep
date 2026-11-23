# Changelog

All notable changes to `ASEP` (Axiomatic Systems Engineering Protocol) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0-alpha] - Unreleased

### Added
- Capability-aware delegation: `CapabilityProfile` now carries extensible `supported_media`, `tool_domains`, and `delegation_threshold`. `asep_agent_send` accepts optional capability/media fields and rejects tasks whose required capabilities or media are unsupported before any message is stored.
- Ambient skill discovery: `asep init` injects a pointer block into `AGENTS.md`/`CLAUDE.md`/`GEMINI.md` and scaffolds a flat `skills/` tree, including the `using-asep` bootstrap skill. `asep check --rigor T1` validates ambient skill structure.
- Capability-domain agent presets (`code-worker`, `git-worker`, `media-worker`, `verifier`) and judge trajectory review of `tool_call_count`/`duplicate_calls`.


### Changed
- **Breaking:** `asep pack` is now provisioning-only; skill file copying was removed. Use `asep init` for ambient skill files.
- **Breaking:** `PackOptions.skills_filter` was removed.
- OpenCode MCP wiring now uses the v2 `mcp.servers.asep` shape instead of legacy `mcp.asep`.

## [2.0.0-alpha] - 2026-11-21

### Changed
- **Breaking:** guard-clause style is now mandatory. `AS-CODE-ELSE` (Error) forbids every `else` token and `AS-CODE-NESTING` (Error, `max_nesting = 0`) forbids any nested `if`; both are non-suppressible. Code that previously passed `asep check` may now fail; refactor to early-return guard clauses.
- Target cleanup (`asep clean`), pinned toolchain + vendored deps for offline builds, and E2E harness subcommands (`asep harness new-run|vendor-loom|verify`) replace legacy shell scripts.

### Added
- Five MUST requirements (FR-ANTISLOP-05..09) with full traceability; 12 anti-slop rules across 4 domains.

## [1.1.0-alpha] - 2026-11-17

### Added
- Git topology rules in the workflow router: branches from `dev`, seven prefixes by commit type, release branches, and back-merge (`CONTRIBUTING.md` section 2).
- Pull request template at `docs/pull_request_template.md`.
- Repository ruleset on GitHub: `main` and `dev` accept changes only through pull requests.
- Explorer, executor, judge, and lead presets now carry steps, system prompts, and permissions in the repo `opencode.json`.

### Changed
- Scope checks: the executor returns `BLOCKED` for tasks without a file and a function; the lead checks scope before delegating; the judge flags out-of-scope changes.
- Executor status line is one of `DONE`, `UNCERTAIN: <reason>`, or `BLOCKED: <reason>`.
- Judge steps raised to 20, executor steps to 30.
- Owner may merge own pull requests after review comment (`CONTRIBUTING.md` section 5).

### Fixed
- Harness: RSS sampled before the child process can exit, so short commands report a real peak instead of 0.

## [1.0.0-alpha] - 2026-11-16

### Added
- Multi-language CLI test harness (`asep harness complexity`, `asep harness memory`) for empirical asymptotic slope and peak process RSS verification (ARCH-10).
- MVCC snapshot isolation engine (`MvccStore`, `SnapshotCoordinator`) with 128-slot atomic reader pool and min_active_seq epoch tombstone reclamation (ARCH-12).
- Asymptotic complexity and zero-allocation tracking harness (`AsymptoticAnalyzer`, `TrackingAllocator`) with peak budget and zero-leak assertions (ARCH-07, ARCH-08).
- Anti-slop and guard clause auditor detecting banners, workflow narration, AI buzzwords, and excessive nesting in CI Gate 1 (ARCH-05, ARCH-09).
- Shared walker crate (`srcwalk`) with cross-platform circular symlink detection and SIMD binary scanning (ARCH-01).
- Multi-agent presets for OpenCode (`lead`, `executor`, `judge`, `explorer`) (ARCH-06).
- Multi-language integration test suite for Rust, TypeScript, Python, and Go indexing (ARCH-03, BUG-13).
- **First release of ASEP:** Axiomatic Systems Engineering Protocol. Deterministic SDLC validator, traceability graph checker, and living specification engine.
- **ASEP Core Protocol Suite** (6 self-contained engineering protocols):
  - `system-architecture`: 2-Phase SDLC (Exploratory Spike → Vertical-Slice TDD + Systematic Debugging), Rich ADRs, Alloy/TLA+ formal proofs, 5-Gate CI
  - `engineering-specs`: ISO 29148 PRD/TDD grammar, Architecture-as-Code (DBML, OpenAPI, Alloy, TLA+), PlantUML SVG diagrams, zero-slop technical prose
  - `code-contracts`: Meyer's Design by Contract docstrings, strict conditionality, zero parroting, memory ownership & concurrency contracts
  - `repository-governance`: SWEBOK v4 README, 5-part ARCHITECTURE, Diátaxis MANUAL (ISO 26514), CVD SECURITY, CONTRIBUTING
  - `design-tokens`: KDL design tokens, OKLCH color math, WCAG AA/AAA contrast, UI state tokens, 44px tap targets
  - `workflow-router`: Master dispatch orchestrating all 5 domain skills
- **asep CLI** (Rust, `unsafe_code = "deny"`):
  - `asep check`: 1:1 traceability (@req↔@design↔@impl↔@test) + CODET auditor (renamed to docstring contract auditor)
  - `asep ci`: 5-Gate pipeline (Formal Specs → Traceability → Clippy → Tests → Benchmarks)
  - `asep pack`: Universal skill installer for 11 agentic AI tools
  - `asep skill`: Full lifecycle (search, fork, install, merge, bundle, purge, state tracking)
  - `asep mcp`: Model Context Protocol server for IDE integration
  - `asep adr` / `asep render` / `asep init` / `asep update`
- **Advanced Skill Features**:
  - Remote skill fetch (Git + HTTP) with semantic version resolution (`^1.0.0`, `~2.1.0`)
  - 3-way merge with interactive conflict resolution TUI (`--interactive`)
  - Watch mode daemon (`--watch`) with 300ms debounce
  - Requirement-to-test coverage mapper (`--coverage lcov.info`)
  - Cascade purge across all agent tool directories
- **Diátaxis Living Manual** (MANUAL.md, Parts I–V) per ISO/IEC/IEEE 26514:2022

### Notes
- Pre-release / alpha: `2.0.0-alpha`. First tagged release was `v1.0.0-alpha`.
