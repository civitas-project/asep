# Technical Design Document (TDD)

<!-- @design:FR-CLI-01 -->
## FR-CLI-01 Scanner
`spec-engine/src/scanner/mod.rs` walks the target root with `ignore::WalkBuilder` (gitignore, global gitignore, `.git exclude`, `.asepignore`, `.git` optional), reads each recognized source file line by line, and parses traceability tags with `scanner::lexer::parse_tags_from_line`. Code files (`rs`, `go`, `ts`, `js`, `py`, `cpp`, `c`, `h`, `hpp`, `java`, `cs`) emit `@impl`/`@test` tags only; `@req` declarations are skipped in code files so requirements live in spec/markdown files. Results are collected into `ScannedTag` records with relative paths and 1-indexed line numbers.

<!-- @design:FR-CLI-02 -->
## FR-CLI-02 Check command
`spec-engine/src/runner.rs` implements `asep check`: it scans project tags, builds the relation graph (`graph::builder::build_relation_graph`) and the trace graph (`build_trace_graph`), evaluates verification (`verification::evaluate`: structural = closed graph, behavioral = `.spec-engine/behavioral.json::passed`, semantic = evidence per tier policy), audits docstring contracts (`auditor::audit_project_docstrings`), checks installed skill drift, and writes `.spec-engine/verification.json`. Exit codes come from `policy::exit_codes()`: Pass=0, Fail=1, Unproven=3, combined deterministically from gate results.

<!-- @design:FR-CLI-03 -->
## FR-CLI-03 Project init, ADR, diagrams
`spec-engine/src/init/mod.rs::init_asep_project` creates `docs/`, `docs/adr/`, `docs/diagrams/rendered/`, `docs/invariants/` and writes a baseline `docs/PRD.md` only when absent. `spec-engine/src/adr/mod.rs::create_new_adr` scaffolds a numbered Rich ADR under `docs/adr/`. `spec-engine/src/diagram/mod.rs::render_plantuml_diagrams` compiles `.puml` files into SVG assets via PlantUML. The CLI exposes these as `asep init`, `asep adr new`, and `asep render` (runner.rs Commands::Init/Adr/Render).

<!-- @design:FR-CLI-04 -->
## FR-CLI-04 Docstring auditor
`spec-engine/src/auditor/doc_contracts.rs::audit_doc_contracts` accumulates Rust doc comment blocks preceding each function and applies the docstring contract: parroting (docstring starts with the function identifier), phantom sections (`# Panics/Errors/Invariants: None ...`), and infallible error sections, each reported with 1-indexed line numbers. `audit_project_docstrings` walks the tree for `.rs` files, skipping tests/, benches/, examples/, templates/, skeletons/ directories. Anti-slop patterns are enforced as blocking violations outside T0 rigor.

<!-- @design:FR-CONFIG-01 -->
## FR-CONFIG-01 Persistent configuration
`spec-engine/src/config/settings.rs` defines `EngineConfig` with `EditorSettings` (preference: Ask/Terminal/Gui, custom_command), an optional `rigor` tier string, and a `CapabilityProfile` (file_read, file_write, terminal, mcp, subagents, web, context_window). `load_config` reads `.spec-engine/config.toml` via TOML and falls back to defaults when missing or corrupt; `save_config` creates the directory tree and writes pretty TOML. The skill `config` subcommand reads/writes editor preference through these functions.

<!-- @design:FR-MCP-01 -->
## FR-MCP-01 JSON-RPC typed protocol
`spec-engine/src/mcp/protocol.rs` defines typed `JsonRpcRequest` (jsonrpc, id, method, params), `JsonRpcResponse` (jsonrpc, id, result, error), and `JsonRpcError` (code, message, data) structs with serde serialization. `spec-engine/src/mcp/server.rs::process_jsonrpc_request` parses each stdin line into `JsonRpcRequest`, routes methods (`initialize`, `tools/list`, `tools/call`, `ping`), and emits typed `JsonRpcResponse` strings (parse error -32700, method not found -32601, execution error -32603); notifications (null id) receive no response. `run_mcp_server` runs the stdio loop.

<!-- @design:FR-MCP-02 -->
## FR-MCP-02 MCP tools
`spec-engine/src/mcp/tools.rs::get_tools_list` publishes six tools with JSON Schema inputSchemas: `asep_check`, `spec_check` (legacy alias), `adr_new`, `render_diagrams`, `srcidx`, `srcdump`. `execute_tool` dispatches by name, validates required arguments (e.g. `title` for `adr_new`), maps string parameters to strongly typed config (`OutputMode`, `OutputFormat`), and returns MCP `content`/`isError` envelopes or an `Unknown tool` error.

<!-- @design:FR-PACK-01 -->
## FR-PACK-01 Agent presets and embedded skills
`spec-engine/src/pack/manifest.rs` generates `plugin.json` and `mcp_config.json` manifests, merges the `asep` MCP server entry into `opencode.json` without clobbering existing keys, and `scaffold_opencode_agents` installs Lead/Executor/Judge/Explorer presets whose permission rules derive from `CapabilityProfile` (deny-all ceiling plus capability allows; read-only agents get extra deny rules). `spec-engine/src/pack/embedded.rs` holds canonical `SKILL.md` text for each embedded skill, returned by `get_embedded_skills`.

<!-- @design:FR-PACK-02 -->
## FR-PACK-02 Agent detection and plugin install
`spec-engine/src/pack/targets.rs` defines `TargetAgent` (Antigravity, OpenCode, ClaudeCode, Cursor, Windsurf, Continue, RooCode, Copilot, Goose, Zed, Aider) with `is_detected` probing each agent's config/skills directory and `resolve_target_paths` for local or global scopes. `spec-engine/src/pack/installer.rs::run_pack_installer` resolves scope and target agents interactively or headlessly, confirms targets, filters embedded skills, then writes per-agent artifacts: `plugin.json` + `mcp_config.json` + `skills/*/SKILL.md` for Antigravity; skills + `opencode.json` merge + agent presets for OpenCode; `SKILL.md` for ClaudeCode/Goose; `.mdc`/`.md` rules for Cursor/Windsurf/Continue/RooCode/Zed/Aider; and `skills/<id>.md` for Copilot.

<!-- @design:FR-SKILL-01 -->
## FR-SKILL-01 Skill inspection and drift detection
`spec-engine/src/skill/inspector.rs::inspect_skills` reads every `SKILL.md` in the skills dir, parses YAML frontmatter (name, description, version, depends), matches each copy against `get_embedded_skills()` by id or directory, and classifies it as Builtin (content identical), Modified (content differs), or Custom (no embedded original), plus Missing entries for embedded skills absent locally. `detect_skill_drift` returns `SkillDrift` records for modified copies only; missing skills are not reported as drift.

<!-- @design:FR-SKILL-02 -->
## FR-SKILL-02 Skill CRUD and import/export
`spec-engine/src/skill/manager.rs` implements pure file operations on `SKILL.md` documents: `add_skill`/`add_skill_full` create a skill directory with frontmatter and body, normalizing the id (lowercase, spaces to dashes); `delete_skill` removes the skill directory; `export_skill` copies `SKILL.md` to a destination; `import_skill` reads an external markdown file, requires a valid `name:` frontmatter field, derives the id from it, and writes the file into the skills directory. State is preserved on disk as individual skill files.

<!-- @design:FR-INIT-01 -->
<!-- @design:FR-VERIFY-01 -->
## FR-VERIFY-01 Verification engine
`spec-engine/src/verification/mod.rs::evaluate` computes `structural` from `graph.is_closed` (no error-severity coverage violation), `behavioral` from `.spec-engine/behavioral.json::passed` (missing file = false + skipped), and `semantic` from `evidence/semantic/<id>.md` for every `Req` node (valid schema, `verdict == ACCEPTED`, skipped when the tier does not require semantic evidence per `policy::load_policy`). Status is `Fail` when structural or behavioral is false, `Unproven` when semantic evidence is required but missing, otherwise `Pass`.

## FR-INIT-01 Baseline project init
`spec-engine/src/init/mod.rs::init_asep_project` (already declared in source) creates the canonical documentation tree (`docs/adr`, `docs/diagrams/rendered`, `docs/invariants`) and writes `docs/PRD.md` containing `<!-- @req:FR-INIT-01:MUST Baseline project specification -->` only when no PRD exists yet. Invoked by the `asep init` CLI command (runner.rs Commands::Init).

<!-- @design:FR-ANTISLOP-05 -->
## FR-ANTISLOP-05 No `else` — guard clauses only
`spec-engine/policy/asep-policy.toml` gains `[[antislop_rules]] id='AS-CODE-ELSE', domain='code', severity='Error', pattern='\belse\b', message='Else branch forbidden; use early return (guard clause)'`. `spec-engine/src/auditor/anti_slop.rs` adds `AntiSlopViolationKind::ElseBranch { token: String }`, `scan_else(violations, path, cleaned_lines)` over comment-stripped code lines, and a `message_for` arm. A dedicated `scan_else` (rather than a generic regex over raw lines) is required because code-domain canonical rules currently run only on comments; the scanner must run on the stripped code body. The existing gate logic in `runner.rs:177-208` and `ci/mod.rs:179-195` already fails on `Severity::Error`, so no gate change is needed.

<!-- @design:FR-ANTISLOP-06 -->
## FR-ANTISLOP-06 No nested `if`
`AS-CODE-NESTING` becomes `severity='Error'`, `max_nesting=0`. The value is redefined as the **number of enclosing `if` levels allowed**: `scan_nesting` must flag an `if` when `enclosing_ifs > max_nesting`, so 0 forbids every `if` inside another `if` while a top-level `if` stays legal (guard clauses require top-level `if cond { return }`). The reported `depth` stays `enclosing_ifs + 1` for continuity with existing tests. Change the fallback `unwrap_or(3)` to `unwrap_or(0)` and update the doc comment. Keep the brace-stack algorithm and `ExcessiveNesting` kind; update the message to `Nested if forbidden; use early return (guard clause)`.

<!-- @design:FR-ANTISLOP-09 -->
## FR-ANTISLOP-09 Non-suppressible strict rules
`scan_nesting` currently skips line numbers in `allowed_lines` (populated by `asep:allow-antislop`) and `scan_else` must initially be wired the same way. Change both scanners to ignore `allowed_lines` for these two rules only; all other anti-slop rules keep the existing opt-out. This makes requirements 05 and 06 absolute as requested.

## Implementation plan (ordered; one file or one function per unit)

### Task 1 — Policy declaration
- File: `spec-engine/policy/asep-policy.toml`
- Symbol: `AS-CODE-NESTING` (severity Error, max_nesting 0), new `AS-CODE-ELSE` (Error)
- Acceptance: `load_policy()` parses the file; `code_rule("AS-CODE-ELSE")` returns an Error-severity rule; nesting rule has `max_nesting = 0`.
- Verification: `cargo test -p asep --lib --offline policy::tests` then the command emitted by `cargo run -q -p asep --bin asep -- affected spec-engine/policy/asep-policy.toml`.

### Task 2 — Auditor scanners
- File: `spec-engine/src/auditor/anti_slop.rs`
- Symbols: `AntiSlopViolationKind::ElseBranch`, `scan_else`, `message_for`, `scan_nesting` (ignore `allowed_lines`; flag when `enclosing_ifs > max_nesting`; default `unwrap_or(0)`; update doc comment)
- Acceptance: unit tests prove `else` and `else if` are flagged; an `if` inside an `if` is flagged at depth 2 while a single top-level `if` and sequential top-level `if`s are NOT flagged; comment text and string literals containing `else` are not flagged; `asep:allow-antislop` does not suppress these two rules but still suppresses AI-buzzword and narration rules.
- Verification: `cargo test -p asep --lib --offline auditor::anti_slop::tests` plus `asep affected spec-engine/src/auditor/anti_slop.rs`.

### Task 3 — Gate behavior test
- File: `spec-engine/tests/cli_test.rs`
- Symbol: new test `check_fails_on_guard_clause_violations`
- Acceptance: a temp project containing a `.rs` file with one `else` and one nested `if` makes `asep check --rigor T1` exit non-zero; the same project with guard clauses exits 0.
- Verification: `cargo test -p asep --test cli_test --offline`.

### Task 4..N — Self-compliance refactor (one unit per file)
- Files (audited, 35 files / 212 `else` tokens): top offenders `spec-engine/src/runner.rs` (48), `skill/inspector.rs` (13), `skill/menu.rs` (12), `agent_protocol/mod.rs` (11), `pack/manifest.rs` (10), `auditor/anti_slop.rs` (10), `pack/installer.rs` (9), `ci/mod.rs` (9), then the remaining 27 files. Nested `if` sites are found per file by the new scanner.
- Symbol: each file's control-flow sites, converted to sequential guard clauses (`if cond { return/continue/break }`).
- Acceptance: the file has zero `else` tokens and zero nested `if`; public behavior and existing tests unchanged; no `asep:allow-antislop` added for these rules.
- Verification: `cargo run -q -p asep --bin asep -- affected <changed-file>` and run its derived commands.

### Final Task — Whole-branch compliance gate
- Acceptance: `asep check . --rigor T1` exits 0 with the new strict policy in force; zero violations of `AS-CODE-ELSE` and `AS-CODE-NESTING` in audited files.
- Verification: `cargo run -q -p asep --bin asep -- affected <all-changed-paths>`, `cargo clippy -p asep --offline -- -D warnings`, `cargo run -q -p asep --bin asep -- check . --rigor T1`.

## Known consequences of this plan
- The self-compliance phase is the bulk of the work: 35 files, 212 `else` tokens plus nested `if` sites. It is split into one executor unit per file; each unit is independently verified.
- Making the rules Error-severity means intermediate commits will fail `asep check . --rigor T1` until the cleanup phase finishes. The final gate is the acceptance point.
- `.asepignore` already excludes `spec-engine/tests/`, `srcidx/tests/`, `vendor/`, and `harness/e2e/vendor-loom/`, so third-party and fixture code is out of scope.
