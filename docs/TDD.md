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

<!-- @design:FR-ANTISLOP-07 -->
## FR-ANTISLOP-07 Policy-driven enforcement
`spec-engine/src/auditor/anti_slop.rs::code_rule` reads `AS-CODE-NESTING` and `AS-CODE-ELSE` from `spec-engine/policy/asep-policy.toml` through the cached compiled-rule table. Severity and message come from the policy (Error fallback for `AS-CODE-ELSE`), so changing severity or thresholds in the policy changes `asep check`/`asep ci` behavior without recompiling gate logic in `runner.rs` or `ci/mod.rs`.

<!-- @design:FR-ANTISLOP-08 -->
## FR-ANTISLOP-08 Self-compliance
Every audited source file in the repository is refactored to sequential guard clauses (top-level `if cond { return/continue/break }`), removing all `else` tokens and all nested `if`. No `asep:allow-antislop` marker is used for these two rules. `.asepignore` keeps `spec-engine/tests/`, `srcidx/tests/`, `**/benches/**`, `vendor/`, and `harness/e2e/vendor-loom/` out of audit scope.

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
- File: `spec-engine/tests/integration_check_test.rs` (reuses the existing `write_t2_closed_project` helper and T1 anti-slop test pattern at line ~365)
- Symbol: new test `check_fails_on_guard_clause_violations`
- Acceptance: a temp project containing a `.rs` file with one `else` and one nested `if` makes `asep check --rigor T1` exit non-zero; the same project with guard clauses exits 0.
- Verification: `cargo test -p asep --test integration_check_test --offline`.

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

## Rust build optimization pilot

<!-- @design:BUILD-PILOT-01 -->
Compare the current profile (A) with `debug = 0` for non-workspace dependencies only in `dev` and `test` (B). Test B with `--config 'profile.dev.package."*".debug=0' --config 'profile.test.package."*".debug=0'`. Workspace code keeps `line-tables-only`; the release profile, incremental compilation, features, linker, and `nightly-2026-11-14` pin stay unchanged. No new runner is needed.

Use a temporary worktree at the same SHA on the repository filesystem; local `/tmp` is tmpfs. Use one experiment artifact directory at the same path for both variants. Clear only that directory when switching variants; leave the main `target/` intact. Record the SHA, toolchain, linker, job count, and test order. Run serially. Stop if free disk space approaches 4 GiB. At the initial audit, the main `target/` used 3.3 GiB and 8.7 GiB was free.

Measure initial and no-change build wall times for context. After warmup, repeat each edit scenario at least three times per variant. Apply identical changes and restore sources outside measurement; rebuild the initial state before each repeat. Record timing samples, medians, observed peak disk usage from `du -s -B1`, and retained disk usage immediately after the same sequence. Sampling can miss brief spikes. Do not report p95 from three samples. Store results outside the experiment artifact directory.

<!-- @design:BUILD-PILOT-02 -->
Adopt only if both edit medians, observed peak disk usage, and retained disk usage decrease, and the timing improvement does not depend on one sample. Repeat the comparison if differences are close to sample variation; keep the current profile if still inconclusive. Compare the same test target inventory and outcomes. The pilot does not run the full suite or change the applicable acceptance gate.

### Pilot work order

| Order | File and symbol | Acceptance criteria | Verification command |
|---|---|---|---|
| 1. Measure function edit | `spec-engine/src/harness/e2e.rs`, `now_rfc3339` | Temporarily change `duration.as_secs()` to `duration.as_secs().saturating_add(1)` in the experiment worktree. Both variants build the same target. Record each repeat and restore the file. | `cargo build -p asep --bin asep --offline --locked --timings` |
| 2. Measure test edit and disk | `spec-engine/tests/harness_e2e_test.rs`, `test_new_run_writes_manifest` | Temporarily replace the seed presence assertion with a check for seed `883735528`. Both variants use the same tests and outcomes. Record time, observed peak, and retained size for steps 1–2. Restore the file. | `cargo test -p asep --test harness_e2e_test --offline --locked -- --list`; `cargo test -p asep --test harness_e2e_test --offline --locked` |
| 3. Adopt if it passes | `Cargo.toml`, `profile.dev.package."*".debug` and `profile.test.package."*".debug` | Add only these two overrides. Do not change global config, tests, or gates. Leave the manifest unchanged if the candidate fails. | Run `asep affected Cargo.toml` and its suggested commands. If empty, record the missing graph mapping; run `cargo metadata --offline --locked --no-deps --format-version 1` and repeat the pilot target test without override flags in the experiment directory. |

<!-- @impl:BUILD-PILOT-01 -->
The pilot ran at `446a0656` with 4 jobs in both orders: A then B, and B then A. Each run measured three function edits and three test edits. Test edit wall time covered compilation via `--list` and test execution. Disk sampling ran every 1.5 seconds; one sample in the second baseline failed when a temporary file disappeared.

<!-- @impl:BUILD-PILOT-02 -->
Both orders met the adoption criteria. The candidate became two overrides in `Cargo.toml`. Reference targets were not reached. The acceptance gate stayed unchanged.

## Rust build optimization follow-up

<!-- @design:BUILD-NEXT-01 -->
Initial audit: `target/debug/build` used 1,205,305,344 bytes. Regular file blocks deduplicated by inode accounted for 919.0 MiB of compilation unit output, 203.9 MiB of build script executables and metadata, 20.0 MiB of build script output, and 6.6 MiB of other files. Grammar `.a` and `.o` files were not the main contributor. Four no-edit pilot builds reported no recompilation. This mixed-cache snapshot neither establishes the new profile size nor proves that every command is free from invalidation.

<!-- @design:BUILD-NEXT-02 -->
Start with one version duplication: `srcdump` uses TOML 0.8.23, while ASEP already resolves TOML 1.2.1 through its `1.1.8` version constraint. Compare compilation units, not just `cargo tree` lines. Do not unify `syn` across versions or change the TLS backend in this round.

<!-- @design:BUILD-NEXT-03 -->
Run all tests offline with the pinned nightly, the pilot debug profile, one experiment cache on the repository filesystem, and at least 4 GiB free. Do not clear the main cache. For structural A/B comparisons, start with an empty experiment directory at the same path. Measure the initial build, three identical edit rebuilds, and observed peak and retained disk usage. For job counts and queue behavior, match cache state before each trial and reverse the order. Record each process's wall time, total completion time, Cargo lock messages, and peak and retained disk usage. Do not generalize job count results to other machines. The full suite and acceptance gate stay unchanged.

### Work order

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| 1. Audit cache | `target/debug/build`, unit directories and build script output | Classify blocks without double-counting hard links. List the largest components and distinguish old artifacts from active output when evidence permits. Delete nothing. | `du -s -B1 target/debug/build`; inode inventory; warm no-op pilot log |
| 2a. Merge one group | `spec-engine/tests/lexer_test.rs`, `scanner` module | Copy `test_scan_project_tags_respects_gitignore` from `scanner_test` without changing its fixture or assertions. Keep the five lexer cases. | `cargo test -p asep --test lexer_test --offline --locked` |
| 2b. Remove duplicate target | `spec-engine/tests/scanner_test.rs`, `scanner_test` target | Delete only after all six cases are listed and pass. Compare the two old targets against the new one. Adopt if cost drops and edit feedback does not slow beyond sample variation. | `cargo test -p asep --test lexer_test --offline --locked -- --list`; `cargo metadata --offline --locked --no-deps --format-version 1`; A/B benchmark |
| 3a. Pin config behavior | `srcdump/tests/config_test.rs`, `Config::from_file` | Test a valid config fixture and invalid TOML before changing the parser version. | `cargo test -p srcdump --test config_test --offline --locked` |
| 3b. Unify versions | `Cargo.toml`, `workspace.dependencies.toml` | Match ASEP's version constraint without changing other features. | `cargo metadata --offline --format-version 1` in the experiment worktree |
| 3c. Update resolution | `Cargo.lock`, TOML 0.8 family | Resolve offline without unrelated upgrades. Adopt if the old units disappear, config tests pass, and feedback and disk usage do not regress. | `cargo tree -p asep -d --offline --locked`; `cargo test -p asep --test lexer_test --unit-graph -Z unstable-options --offline --locked`; `cargo test -p srcdump --test config_test --offline --locked` |
| 4. Test job counts and queues | `spec-engine/tests/lexer_test.rs`, `lexer_test` target; `Cargo.toml`, unchanged `build.jobs` setting | Compare 2 versus 4 jobs for an initial build and a rebuild after the same edit. Compare identical serial and concurrent build/test pairs on the experiment cache. Separate lock wait from compilation where logs support it. Report findings without changing defaults. | `cargo build -p asep --bin asep --offline --locked --jobs <2-or-4> --timings`; `cargo test -p asep --test lexer_test --offline --locked --jobs <2-or-4>` |

For each change, run `asep affected <changed-paths>` first. If it returns no commands or names nonexistent Cargo targets, record that and use the targeted checks in the table. Review each unit's diff and evidence before continuing.

<!-- @impl:BUILD-NEXT-01 -->
The audit is complete. Consolidating `scanner_test` into `lexer_test` was not adopted. The valid round saved 19.9 MiB of retained artifacts (2.7%), but every candidate edit sample was slower than every baseline sample. Both targets remain separate in the main worktree.

<!-- @impl:BUILD-NEXT-02 -->
TOML unification removed six old compilation units without adding any. It was not adopted: in one round, the initial build was faster and disk use was lower, but every candidate edit sample was slower than every baseline sample. The parser version and main lockfile stayed unchanged; the three `srcdump` config tests were retained.

<!-- @impl:BUILD-NEXT-03 -->
On this machine, 4 jobs were faster in one cold-cache comparison. The three edit samples per job count overlapped. Run builds and tests serially when sharing a target directory: concurrent runs hit lock waits without a consistent total time improvement. Cargo defaults stayed unchanged.

## Full Rust P0: leader and local acceptance

<!-- @design:FR-LEADER-01 -->
Keep four agents. `leader` sets scope and reports review results, not acceptance. `executor` supplies the snapshot identity or diff hash, command, exit code, and output location. `judge` stays read-only and reviews the diff and evidence identifying the snapshot and worktree. `explorer` may use `read`/`grep` when `srcdump` is unavailable. Apply the new names in repository configuration and installer presets. Back up global config and change it only on this machine, preserving models and unrelated permissions.

<!-- @design:FR-LEADER-02 -->
Switch directly to `Role::Leader` and the wire value `leader`. Bump the envelope and schema versions from 1 to 2. Reject old versions and the value `lead`. Do not rewrite or migrate old messages. Kind rules, ACK permissions, and message status meanings stay unchanged.

<!-- @design:FR-ACCEPT-01 -->
Initial audit: `MANUAL.md` listed gates 1–5, while the workflow router and CLI used 0–4 with different checks. `verification::evaluate` accepts a boolean from `behavioral.json` without binding it to a snapshot. Keep `asep check` and `asep ci` as local feedback, not acceptance evidence. P0 proposes `asep accept` for one Git commit, with a clean snapshot prepared by the runner and policy and reference tests from an approved source. Build and test the candidate under a separate OS identity or isolation boundary with no write access to the runner, policy, reference tests, or evidence store. The runner records each gate result, commit, fingerprint of tested inputs, toolchain, and policy identity. Do not accept workspace JSON as evidence. Do not issue `ACCEPTED` if permission separation cannot be enforced.

GitHub Actions for merge authorization is deferred. Do not enable or replace required checks or change branch protections. P0 verifies snapshots locally only. Compiling a benchmark does not prove performance.

### Proposed implementation order

Run `asep affected <changed-paths>` for each unit. Use targeted checks in the table if the graph does not map the change.

| Unit | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| 1. Role contract | `spec-engine/tests/agent_protocol_test.rs`, `spec-engine/tests/mcp_test.rs`, `spec-engine/tests/integration_check_test.rs`; role and envelope fixtures and legacy-version cases | New names are valid; old names and envelopes are rejected. Keep ACK and invalid-version diagnostic coverage. | `cargo test -p asep --offline --locked --test agent_protocol_test --test mcp_test --test integration_check_test` |
| 2. Hard cutover | `spec-engine/src/agent_protocol/mod.rs` (`Role`, `ENVELOPE_VERSION`, `MessageStore::ack`), `spec-engine/policy/agent-protocol.toml` (`version`, `roles`), `spec-engine/src/mcp/tools.rs` (`mcp_role`, tool descriptions) | Rust, schema, and MCP agree without aliases. All three must agree before closing the unit. | Unit 1 test command. |
| 3. Presets and terminology | `spec-engine/src/pack/manifest.rs` (`scaffold_opencode_agents`), `spec-engine/tests/targets_test.rs` (preset tests), `opencode.json` and `~/.config/opencode/opencode.json` (`agents`, `default_agent`); role documentation in `workflow-router/skill/SKILL.md` and `harness/e2e/RUNBOOK.md` | New presets use `leader`; global config selects it without changing models or permissions. Add no agent. | `cargo test -p asep --offline --locked --test targets_test`; validate both JSON configs and OpenCode agent selection. |
| 4. Gate contract | `MANUAL.md` (pipeline), `spec-engine/src/ci/mod.rs` (`gate_name`, `run_ci_pipeline`) | Reconcile numbers, commands, and mandatory versus N/A outcomes before wiring acceptance. Skipping a mandatory gate cannot count as a pass. | `cargo test -p asep --offline --locked --test ci_and_formal_test` |
| 5. Offline acceptance | `spec-engine/src/acceptance.rs` (proposed `evaluate_acceptance`), `spec-engine/src/cli.rs` (`Commands::Accept`), `spec-engine/src/runner.rs` (dispatch), `spec-engine/tests/acceptance_test.rs` | Evidence applies only to the tested snapshot. Test failed or skipped gates, missing, stale, or forged evidence, input changes, and missing isolation. | `cargo test -p asep --offline --locked --test acceptance_test`; test without network and verify write denial from the candidate process identity. |
| 6. Remove workflow from P0 | `.github/workflows/asep-merge.yml` (`asep-merge`), `opencode.json` (`agents.leader.system`), `spec-engine/src/pack/manifest.rs` (`scaffold_opencode_agents`), `workflow-router/skill/SKILL.md` (role protocol), `MANUAL.md` (pipeline) | Delete the undeployed draft workflow. Prompts and docs do not name GitHub Actions as an active gate. Check global config and update `agents.leader.system` only if it claims Actions are active. Leave rulesets, permissions, and models unchanged. | `test ! -e .github/workflows/asep-merge.yml`; validate JSON config; `cargo test -p asep --offline --locked --test targets_test`; `git diff --check` |
| 7. Close local acceptance | `spec-engine/src/acceptance/runner.rs` (`accept_commit`), `spec-engine/tests/acceptance_test.rs` (success and rejection evidence), `docs/TRACEABILITY.md` (`FR-ACCEPT-01`) | Run a controlled Git fixture on a provisioned Linux runner without network. A valid snapshot passes mandatory gates and yields protected evidence. A wrong snapshot, failed gate, or untrusted environment yields no accepted evidence. Do not close the requirement before obtaining success evidence. | `cargo test -p asep --offline --locked --lib acceptance::`; `cargo test -p asep --offline --locked --test acceptance_test`; test `asep accept <full-commit>` on the provisioned fixture; `asep check . --rigor T1` |

Split multi-file rows into single-file or single-symbol units before delegation. P0 makes no other agent API changes and adds no ENI, persistent state machine, or crate.

## ASEP 3.0.0-alpha release

<!-- @design:FR-REL-03 -->
Bump the major version because `Role::Lead`, wire value `lead`, and envelope v1 are no longer compatible. Stay on alpha. Keep the P0 and version commits separate and retain the local build optimization commit. Exclude PDFs, `SUGEST.md`, and local logs. The original publication plan pushed the same SHA to `release/3.0.0-alpha` on both remotes without force-pushing, merging, or tagging. The approved history replacement below applies only to that release branch.

<!-- @design:FR-REL-04 -->
Global OpenCode runs `asep mcp` from `PATH`, selecting `1.0.0-alpha`. Back up and atomically replace `~/.cargo/bin/asep` with the binary built from the verified release commit. Use its absolute path in `mcp.asep.command` so OpenCode cannot select another binary. Preserve `args`, default `leader`, models, providers, and permissions. A user MCP installation does not provision the trusted runner.

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| 1. Commit P0 | Reviewed P0 diff, `HEAD` and `refs/heads/release/3.0.0-alpha` | Compare both remotes; preserve the GitHub `dev` baseline and local commits without overwriting remote branches in the initial push. Commit only reviewed P0 files. | `git diff --check`; `git merge-base --is-ancestor`; `asep affected <changed-paths>`; `asep check . --rigor T1` |
| 2. Bump Rust | `spec-engine/Cargo.toml`, `package.version`; `Cargo.lock`, `asep` package (separate unit) | Both use `3.0.0-alpha`; other dependency versions stay unchanged. | `cargo metadata --offline --locked --no-deps --format-version 1`; inspect lockfile diff |
| 3. Bump Dart | `spec-engine/dart/pubspec.yaml`, `version` | Version is `3.0.0-alpha`. | Check the YAML `version` field |
| 4. Changelog | `CHANGELOG.md`, `3.0.0-alpha` section | Record the breaking rename, protocol v2, offline acceptance, and build optimization. Do not claim merge or deployment. | `git diff --check`; inspect changelog section |
| 5. Verify and push | `docs/TRACEABILITY.md`, `FR-REL-03`/`FR-REL-04` evidence; release refs on both remotes | Record actual evidence. Commit the version separately. Both remotes point to the same release SHA. | `asep affected <changed-paths>`; targeted tests; `cargo build -p asep --offline --locked --bin asep`; `asep --version`; `asep check . --rigor T1`; `git ls-remote` |
| 6. Synchronize global OpenCode | `~/.cargo/bin/asep` (binary); `~/.config/opencode/opencode.json`, `mcp.asep.command` (separate unit) | The installed binary matches the release build. Only the ASEP MCP target changes in the config. | Compare binary hashes; run `--version` and MCP `initialize` via the configured command; compare JSON before and after |

### English release documentation and commit replacement

<!-- @design:FR-REL-05 -->
Recreate the three release commits from `446a0656` with their existing English subjects and intent. Keep the old tip as a local recovery ref. Translate Indonesian additions to English without changing runtime behavior, benchmark results, or acceptance criteria. Replace only the unmerged release branch, using an explicit lease against each remote's previously verified tip. Do not rewrite `main`, `dev`, or tags. If either lease fails, stop and report the mismatch.

| Unit | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| 1 | `docs/PRD.md`, build optimization, P0, and release sections | English prose; existing requirements and IDs preserved. | `git diff --check -- docs/PRD.md`; review translated sections against the old tip. |
| 2 | `docs/TDD.md`, build optimization, P0, and release sections | English prose; numerical results, commands, and deployment limits preserved. | `git diff --check -- docs/TDD.md`; compare commands and measurements with the old tip. |
| 3 | `docs/TRACEABILITY.md`, build, P0, and release evidence sections | English evidence and notes; graph links and measured results preserved. | `asep affected docs/TRACEABILITY.md`; `git diff --check -- docs/TRACEABILITY.md`; `asep check . --rigor T1`. |
| 4 | `CHANGELOG.md`, `3.0.0-alpha` section | English and accurate; no claim of merge or tagged release. | `git diff --check -- CHANGELOG.md`; inspect release section and commit subjects. |
| 5 | `refs/heads/release/3.0.0-alpha` and release PR text | Recreated commits contain no unintended file changes; both remote release refs match the new tip; PR text is English. | Compare old and new tip trees, run targeted checks only, check both refs with `git ls-remote`, and inspect the draft PR. |

## Capability-Aware Delegation & Ambient Skill Distribution

### Phase M — MCP capability routing (first)

<!-- @design:FR-CAP-01 -->
Capability registry: `CapabilityProfile` in `spec-engine/src/config/settings.rs` gains `supported_media: Vec<String>` and `tool_domains: Vec<String>` (config-driven, validated by `asep check`). No hardcoded enum; unknown entries are accepted but warned. Default profile includes `text`, `code`.

<!-- @design:FR-CAP-02 -->
MCP delegation schema: extend `asep_agent_send` payload (in `spec-engine/src/mcp/tools.rs` and `spec-engine/src/agent/protocol.rs` if present) with optional fields `required_capabilities`, `media`, `domain`, `tool_filter`, `delegation_threshold`. Serde `#[serde(default)]` keeps backward compat. Validation in `execute_tool`.

<!-- @design:FR-CAP-03 -->
Capability guard: executor (and any worker) checks task's `required_capabilities`/`media` against its profile before running; mismatch returns structured `BLOCKED: requires <capability>` without executing. Implemented in `spec-engine/src/agent/executor_guard.rs` (new) or existing executor handler.

<!-- @design:FR-CAP-04 -->
Single-agent default + threshold: leader prompt/rules and `CapabilityProfile.delegation_threshold` (default e.g. 3 tool calls) define when to delegate. Implemented as config + documented rule; no code change beyond config field, enforced by agent behavior + check.

<!-- @design:FR-CAP-05 -->
Tool filtering + trajectory: worker loads only tools in `tool_filter` (or domain default). TASK_REPORT includes `tool_call_count: u32` and optional `duplicate_calls`. Added to report schema in `spec-engine/src/agent/protocol.rs`.

<!-- @design:FR-CAP-06 -->
Trajectory review: judge flags duplicate/unnecessary tool calls as blocking when evidence shows waste. Documented in judge prompt (opencode.json) + `asep check` optional gate.

<!-- @design:FR-CAP-07 -->
Capability-domain workers: `scaffold_opencode_agents` in `spec-engine/src/pack/manifest.rs` generates `code-worker`, `git-worker`, `media-worker`, `verifier` presets with supported_media/tool_domains from CapabilityProfile.

<!-- @design:FR-CAP-08 -->
Media routing: router/leader selects worker whose supported_media covers task media; text-only agents never receive image/pdf/audio/video tasks. Enforced by capability guard (FR-CAP-03) + documented routing rule.

### Phase A — Ambient skill distribution (after Phase M)

<!-- @design:FR-AMB-01 -->
Ambient init: `init_asep_project` in `spec-engine/src/init/mod.rs` (or new `spec-engine/src/init/ambient.rs`) injects pointer block into AGENTS.md/CLAUDE.md/GEMINI.md and scaffolds flat `skills/` with canonical SKILL.md. Idempotent.

<!-- @design:FR-AMB-02 -->
Pack provisioning-only: remove skill-copy logic from `spec-engine/src/pack/installer.rs` (hard remove); keep role/permission/MCP wiring. Update `run_pack_installer`.

<!-- @design:FR-AMB-03 -->
Bootstrap injection: add `using-asep` skill + harness plugin entry (Pi extension style) generated by pack for supported harnesses. Embedded in `spec-engine/src/pack/embedded.rs`.

<!-- @design:FR-AMB-04 -->
Description-first + token budget: update all `*/skill/SKILL.md` frontmatter; `asep check` validates description starts with `Use when...` and word count budget.

<!-- @design:FR-AMB-05 -->
Skill capability metadata: SKILL.md frontmatter gains `requires: [<capability>]`; router skips unavailable. Parsed in `spec-engine/src/skill/inspector.rs` or new parser.

<!-- @design:FR-AMB-06 -->
Ambient governance gate: `asep check` validates pointer block presence, flat skills dir, description-first, token budget. New checks in `spec-engine/src/runner.rs` / auditor.

### Ordered implementation plan (Phase M first, then Phase A)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| M1 | `spec-engine/src/config/settings.rs`, `CapabilityProfile` (add `supported_media`, `tool_domains`, `delegation_threshold`) | Fields serialize/deserialize; defaults include text/code; invalid types rejected. | `cargo test -p asep spec-engine` settings tests; `asep check . --rigor T1` |
| M2 | `spec-engine/src/mcp/tools.rs`, `asep_agent_send` schema + handler | New fields optional with defaults; validation rejects malformed; backward compat with old payloads. | `cargo test -p asep mcp` tests; `asep affected spec-engine/src/mcp/tools.rs` |
| M3 | `spec-engine/src/agent/executor_guard.rs` (new) or executor handler, capability guard | Mismatch returns `BLOCKED: requires <cap>`; no execution; match proceeds. | Unit tests for guard; `asep affected` |
| M4 | `spec-engine/src/pack/manifest.rs`, `scaffold_opencode_agents` | Generates code/git/media/verifier presets from CapabilityProfile. | `asep pack --dry-run` or targeted tests; `asep affected` |
| M5 | `spec-engine/src/agent/protocol.rs`, TASK_REPORT (`tool_call_count`, `duplicate_calls`) | Fields serialized; judge can read. | Tests; `asep affected` |
| M6 | Judge prompt in `opencode.json` (project + global) + docs | Judge flags duplicate/unnecessary tool calls as blocking. | Manual review; `asep check . --rigor T1` |
| M7 | `spec-engine/src/init/mod.rs` + `spec-engine/src/init/ambient.rs` (new) | Pointer block injected idempotently; flat `skills/` scaffolded. | Integration test; `asep affected` |
| M8 | `spec-engine/src/pack/installer.rs`, `run_pack_installer` | Skill copy removed; provisioning only; no fallback. | Tests; `asep check . --rigor T1` |
| M9 | `spec-engine/src/pack/embedded.rs` + harness plugin entries | `using-asep` bootstrap skill embedded and installable. | `asep pack` dry-run; tests |
| M10 | `*/skill/SKILL.md` frontmatter + `spec-engine/src/skill/inspector.rs` | Description-first + token budget + `requires` parsed. | `asep check . --rigor T1`; `asep affected` |
| M11 | `spec-engine/src/runner.rs` + auditor, ambient governance gate | Pointer block, flat skills, description-first, budget validated. | `asep check . --rigor T1` |
| M12 | Verification & reconfig | Run `asep pack` to apply latest config after Phase M and after Phase A; restart MCP. | `asep --version`; MCP `initialize`; `asep check . --rigor T1` |

After Phase M complete: restart with latest config. After Phase A complete: restart again. Full suite reserved for merge to main.
