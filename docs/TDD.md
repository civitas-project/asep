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
