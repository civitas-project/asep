# Product Requirements Document (PRD)

<!-- @req:FR-CLI-01:MUST The scanner shall extract traceability tags from project files while honoring ignore rules. -->
<!-- @req:FR-CLI-02:MUST The check command shall evaluate structural, behavioral and semantic verification and report a deterministic exit code. -->
<!-- @req:FR-CLI-03:MUST The CLI shall initialize projects, create ADRs and render PlantUML diagrams. -->
<!-- @req:FR-CLI-04:MUST The auditor shall detect docstring contract violations and anti-slop patterns. -->
<!-- @req:FR-CONFIG-01:MUST The engine shall persist and load editor preference, rigor and capability configuration. -->
<!-- @req:FR-MCP-01:MUST The MCP server shall process JSON-RPC requests using typed protocol structures. -->
<!-- @req:FR-MCP-02:MUST The MCP server shall expose check, spec_check, srcdump and srcidx tools with validated schemas. -->
<!-- @req:FR-PACK-01:MUST The pack module shall generate agent presets and embed canonical skill text. -->
<!-- @req:FR-PACK-02:MUST The installer shall detect target agents and install plugin artifacts for them. -->
<!-- @req:FR-SKILL-01:MUST The skill subsystem shall inspect installed skills and detect drift from embedded originals. -->
<!-- @req:FR-SKILL-02:MUST The skill manager shall add, delete, import and export skills while preserving state. -->
<!-- @req:FR-VERIFY-01:MUST The verification engine shall derive PASS, FAIL or UNPROVEN from graph coverage, behavioral report and semantic evidence. -->
<!-- @req:FR-ANTISLOP-05:MUST The anti-slop auditor shall emit an Error-severity violation for every `else` token in audited source files, and `asep check --rigor T1` shall exit non-zero whenever such a violation is present. -->
<!-- @req:FR-ANTISLOP-06:MUST The anti-slop auditor shall emit an Error-severity ExcessiveNesting violation for any `if` nested inside another `if` (depth >= 2) in audited source files, and `asep check --rigor T1` shall exit non-zero whenever such a violation is present. -->
<!-- @req:FR-ANTISLOP-07:MUST Both rules shall be declared in `spec-engine/policy/asep-policy.toml` with `severity = 'Error'` — `AS-CODE-NESTING.max_nesting = 0` (zero nesting levels allowed; top-level `if` remains permitted for guard clauses) and a new `AS-CODE-ELSE` rule — so that gate behavior is policy-driven rather than hardcoded. -->
<!-- @req:FR-ANTISLOP-08:MUST The repository itself shall contain no `else` token and no nested `if` in audited source files, so that `asep check . --rigor T1` exits 0 without relying on inline opt-outs for these two rules. -->
<!-- @req:FR-ANTISLOP-09:MUST The `asep:allow-antislop` inline marker shall not suppress `AS-CODE-ELSE` or `AS-CODE-NESTING` violations; every other anti-slop rule shall keep its existing opt-out behavior. -->

## Pilot optimasi build Rust

<!-- @req:BUILD-PILOT-01:MUST The pilot shall compare edit-to-feedback time after a private function edit and an integration test edit, plus sampled peak and retained build storage, against the current ASEP baseline on the same pinned toolchain. -->
<!-- @req:BUILD-PILOT-02:MUST The candidate shall be adopted only if feedback is consistently faster and build storage consistently smaller, with the same scoped test outcomes, no reduction in acceptance scope, and the main build cache left intact. -->

Target acuan, bukan syarat lulus: median feedback edit fungsi turun 40%, disk puncak turun 30%, dan artefak tersisa turun 40%.

## Lanjutan optimasi build Rust

<!-- @req:BUILD-NEXT-01:MUST Classify allocated bytes under target/debug/build without deleting the main cache. Measure consolidation of one integration-test group and preserve its test cases, assertions, and outcomes. Adopt it only if it reduces build cost without slowing edit feedback beyond observed variation. -->
<!-- @req:BUILD-NEXT-02:MUST Measure actual Cargo dependency build units before and after removing a redundant variant. Keep supported inputs, features, and targeted tests intact. -->
<!-- @req:BUILD-NEXT-03:MUST Compare two versus four Cargo jobs and concurrent versus serial Cargo commands on the same isolated target. Record wall time, lock waits, and disk use without changing global defaults. -->

## Capability-Aware Delegation & Ambient Skill Distribution (Track: agent-topology)

### Phase M — MCP capability routing (first)

<!-- @req:FR-CAP-01:MUST CapabilityProfile shall declare an extensible `supported_media` list (text, image, pdf, audio, video, code, data, and future entries) and a `tool_domains` list; these shall be config-driven, not a hardcoded enum, and validated by `asep check`. -->
<!-- @req:FR-CAP-02:MUST The MCP agent messaging protocol shall extend `asep_agent_send` payloads with optional `required_capabilities`, `media`, `domain`, `tool_filter`, and `delegation_threshold` fields; existing callers without these fields shall continue to work (additive, no break). -->
<!-- @req:FR-CAP-03:MUST A task whose required capabilities are not covered by the receiving agent shall be rejected with a structured `BLOCKED: requires <capability>` result rather than guessed or partially executed. -->
<!-- @req:FR-CAP-04:MUST The leader shall apply a single-agent default: a task completable within the configured delegation threshold (tool calls) shall be handled by the current agent without delegation. -->
<!-- @req:FR-CAP-05:MUST Each executor shall load only the tools relevant to the task's domain (tool filtering) and report `tool_call_count` in its TASK_REPORT for trajectory review. -->
<!-- @req:FR-CAP-06:MUST The judge shall flag duplicate or unnecessary tool calls as a blocking issue when trajectory evidence shows waste. -->
<!-- @req:FR-CAP-07:MUST Agent presets generated by `asep pack` shall include capability-domain workers (code, git, media, verify) whose supported_media and tool_domains derive from CapabilityProfile. -->
<!-- @req:FR-CAP-08:MUST Media-tagged tasks shall be routed only to agents whose supported_media includes the required media; text-only agents shall never receive image/pdf/audio/video tasks. -->
<!-- @req:FR-CAP-09:MUST `PackOptions` shall not expose a `skills_filter` field; skill selection belongs to ambient init, not provisioning. -->

### Scoped non-root acceptance

<!-- @req:FR-ACC-02:MUST A trusted local runner shall operate without root: policy and evidence shall be user-owned (0600/0700), and candidate code shall execute inside unprivileged bubblewrap with the snapshot read-only and policy/evidence not mounted into the sandbox. -->
<!-- @req:FR-ACC-03:MUST After all scoped gates pass, the runner shall write `evidence/semantic/<REQ-ID>.md` with `verdict: ACCEPTED` for every `@req:*:MUST`; `asep check` shall accept `--evidence-dir <path>` and read semantic evidence from that path. -->
<!-- @req:FR-ACC-04:MUST The acceptance Tests gate shall run only tests derived from `asep affected <changed-paths>`; `cargo test --workspace --all-targets` shall not be a mandatory acceptance gate. BenchPreflight shall be opt-in. -->
<!-- @req:FR-ACC-05:MUST `asep check` shall resolve the semantic evidence directory from `.spec-engine/config.toml` (`evidence_dir`) when `--evidence-dir` is absent, so `asep check --rigor T2` is proven after scoped acceptance without extra flags. -->
<!-- @req:FR-MCP-03:MUST The MCP server shall expose `asep_accept_scoped` with a validated schema (`commit`, optional `dir`) that invokes the scoped non-root acceptance runner and returns the evidence directory. -->
<!-- @req:FR-INT-01:MUST OpenCode leader/executor presets and documentation shall instruct agentic verification via `asep check --rigor T2` followed by `asep accept-scoped <commit>`; the agent shall never grant ACCEPTED itself. -->

### Phase A — Ambient skill distribution (after Phase M)

<!-- @req:FR-AMB-01:MUST `asep init` shall inject an ambient pointer block into AGENTS.md/CLAUDE.md/GEMINI.md and scaffold a flat `skills/` directory with canonical SKILL.md files, so skills are discoverable without `asep pack`. -->
<!-- @req:FR-AMB-02:MUST `asep pack` shall become provisioning-only (agent roles, permissions, MCP wiring); skill file copying shall be removed (hard remove, no fallback). -->
<!-- @req:FR-AMB-03:MUST A bootstrap skill shall be auto-injected at session start for supported harnesses (superpowers-style), activating ambient skill discovery. -->
<!-- @req:FR-AMB-04:MUST Each SKILL.md shall use description-first discovery (`Use when...`) with a token budget (frequently-loaded skills <= 200 words; descriptions <= 500 chars). -->
<!-- @req:FR-AMB-05:MUST Skills may declare `requires: [<capability>]` metadata; the router shall skip skills whose required capabilities are unavailable to the active agent/model. -->
<!-- @req:FR-AMB-06:MUST `asep check` shall validate ambient skill structure (pointer block presence, flat skills directory, description-first frontmatter, token budget) as part of governance. -->

## Leader protocol

<!-- @req:FR-LEADER-01:MUST OpenCode presets and generated presets shall use leader as the primary role name. -->
<!-- @req:FR-LEADER-02:MUST The role enum, wire value, and MCP tools shall use Leader/leader; the former lead value shall be rejected. -->

## IN-01 — Agent Observability Protocol (Layer 05)

Sumber: `ASEP_Architecture_Expansion_Proposal_Catalog.pdf` hal. 15. Review verdict **REVISE** (hal. 17): arah diterima, tapi shared event/trace envelope dari langkah A dilebur sebagai unit pertama sebelum konsumen dibangun. Status ini berarti desain disetujui untuk langkah berikutnya, bukan fitur sudah jadi.

<!-- @req:IN-01-CONTRACT:MUST A versioned event/trace envelope shall carry schema_version, trace_id, optional parent_id, task_id, a monotonic seq, a wall timestamp and a monotonic duration; trace_id, parent_id and task_id shall be explicit caller inputs and shall never be inferred from filenames, message IDs, tool output or the wall timestamp. -->
<!-- @req:IN-01-COMPAT:MUST Adding the envelope shall not change the existing Envelope, GateResult, CheckJsonReport, MCP tool schemas or `.spec-engine` config files, and all existing tests shall pass unchanged. -->
<!-- @req:IN-01-COLLECT:MUST A local collector shall append events as NDJSON (newline-delimited JSON) to a per-run file with bounded buffering and a flush at span end, and telemetry shall produce no file unless explicitly enabled. -->
<!-- @req:IN-01-COST:MUST Cost and token metadata shall be nullable and carry a source of measured, estimated, provider_reported or unavailable together with a missing-fields list; an unavailable value shall never be reported as zero or guessed. -->
<!-- @req:IN-01-REPORT:MUST The CLI shall expose `asep trace status|report` and the MCP server shall expose `asep_trace_report`, and both shall reconcile recorded events against CiReport/CheckJsonReport for the same task and gate. -->
<!-- @req:IN-01-PRIVACY:MUST Only allowlisted fields shall be persisted, secrets shall be redacted before write, and `asep trace purge <trace_id>` shall remove that trace from the log and its derived reports; a secret fixture shall never appear in any trace file. -->
<!-- @req:IN-01-RETENTION:MUST Retention shall be a TTL plus a maximum size with segment rotation and removal of expired segments; absolute append-only shall not be claimed. -->
<!-- @req:IN-01-OVERHEAD:MUST Telemetry shall be configurable as off or on with off as the default; there is no sampled mode. File-change and intent evidence is always recorded when telemetry is on. Overhead shall be measured on the same revision with a documented result before on is adopted as a default. -->
<!-- @req:IN-01-METRIC:MUST Trace reports shall present gate pass rate with severity, reproducibility, tool/token/latency, cost per verified result, recovery fidelity and overhead, each with denominator, skipped list, failures and a pinned baseline, and shall not collapse them into a single score. -->

### File-change evidence (added after review discussion)

<!-- @req:IN-01-FILE-EVIDENCE:MUST The collector shall compute a SHA-256 content hash for each in-scope file at span start and span end, and shall emit an NDJSON file-change event only for files whose hash changed, carrying the path, before/after hashes, task ID and span label. -->
<!-- @req:IN-01-INTENT:MUST A write-ahead intent event carrying the target path and its pre-write hash shall be recorded before a controlled tool modifies a file; file-change and intent events shall always be recorded even when ordinary event recording is off, shall be marked as complete evidence, and reports shall state whether ordinary event coverage is on or off. -->

## P3 — Knowledge Graph & Impact Analysis (RFC v0.2)

Sumber: `ASEP_Adaptive_Architecture_v0.2.md` §5, §6, §8 (P3). Urutan: P3.1 transitive impact (sekarang), P3.2 ADR annotation coverage (terpisah), P3.3 graph export (conditional, tunggu konsumen).

### P3.1 — Transitive Impact Analysis

Reuse `build_relation_graph` (tanpa engine baru, database, atau dependency). Additive terhadap CLI/JSON existing.

<!-- @req:FR-IMPACT-01:MUST `asep affected` shall perform cycle-safe transitive traversal of the relation graph from changed paths, following typed edges (`Satisfies`, `Verifies`, `Informs`, `Constrains`) in both directions. -->
<!-- @req:FR-IMPACT-02:MUST The impact report shall include affected implementations, requirements, tests, and ADRs, each categorized by kind. -->
<!-- @req:FR-IMPACT-03:MUST Each affected item shall carry provenance as an ordered list of node IDs forming the traversal path from a changed node. -->
<!-- @req:FR-IMPACT-04:MUST Changed paths that map to no graph node shall be reported explicitly in an `unmapped` field, not silently ignored. -->
<!-- @req:FR-IMPACT-05:MUST Existing CLI text output and JSON fields (`impls`, `requirements`, `tests`, `commands`) shall remain unchanged; new fields are additive only. -->
<!-- @req:FR-IMPACT-06:MUST The legacy same-ID fallback (no `@rel` tags) shall remain functional. -->
<!-- @req:FR-IMPACT-07:MUST Traversal shall be deterministic (sorted output) and shall not loop on cyclic graphs. -->

### P3.2 — ADR Annotation Coverage

Documentation-first, graph data-quality slice. Reuse existing graph builder, impact traversal, validators. No new engine, dependency, or abstraction.

<!-- @req:FR-ADRCOV-01:MUST When an ADR is a changed path, `asep affected` shall traverse incoming `Informs` edges to report genuinely dependent requirements and implementations, without treating the ADR as a transit hub. -->
<!-- @req:FR-ADRCOV-02:MUST Terminal-sink behavior shall be preserved for ADRs reached during traversal: no outgoing traversal from an ADR, and no incoming traversal from a non-start ADR. -->
<!-- @req:FR-ADRCOV-03:MUST Existing impact report fields, traversal behavior for non-ADR nodes, coverage rules, and legacy fallback shall remain unchanged. -->

### P3.3 — Graph export (conditional)

`asep graph` export (JSON/DOT). Deferred until a real consumer exists.

## Layer 5 — Agent Runtime (RFC v0.2)

Sumber: `ASEP_Adaptive_Architecture_v0.2.md` §3, §5, §6, §8 (P0–P5). Urutan: audit redundansi dulu, lalu Engineer Level, lalu resource budgeting, lalu ReAct contracts. Setiap slice additive; tidak ada engine baru.

### Slice 0 — Redundancy audit (P0)

<!-- @req:FR-AUDIT-01:MUST The repository shall contain `docs/AUDIT.md` enumerating existing agent-runtime mechanisms, their consumers, overlap with native runtime capabilities, and a baseline measurement plan; `asep check --rigor T1` shall pass with it present. -->

### Slice 1 — Engineer level

<!-- @req:FR-LEVEL-01:MUST An `EngineerLevel` enum (L1, L2, L3) shall exist with scope descriptions and serialize/deserialize as `L1`/`L2`/`L3` with round-trip. -->
<!-- @req:FR-LEVEL-02:MUST `CapabilityProfile` shall carry an `engineer_level` field defaulting to L1, loaded and saved through the existing config (additive, default applied when absent). -->
<!-- @req:FR-LEVEL-03:MUST The agent `Envelope` shall carry an optional `level` field (additive, absent defaults to L1) that round-trips through serde without breaking existing envelopes. -->

### Slice 2 — Resource budgeting

<!-- @req:FR-BUDGET-01:MUST A `ResourceBudget` struct shall declare optional `max_tokens`, `max_tool_calls`, `max_cost_usd` and `max_latency_ms` (None meaning unlimited) and serialize/deserialize. -->
<!-- @req:FR-BUDGET-02:MUST A `BudgetTracker` shall accumulate usage and report the first exceeded dimension via `exceeded()` returning `Option<&str>`. -->
<!-- @req:FR-BUDGET-03:MUST Exceeding a budget shall produce a structured `BLOCKED: budget exceeded (<dimension>)` result rather than continue. -->

### Slice 3 — ReAct contracts

<!-- @req:FR-REACT-01:MUST `Action` and `Observation` structs shall represent a tool invocation and its result, linked to `trace_id` and `seq`. -->
<!-- @req:FR-REACT-02:MUST An `ActionContract::validate` shall reject actions missing required fields or lacking capability coverage, returning a structured error. -->
<!-- @req:FR-REACT-03:MUST An `Observation` shall be convertible to a `TraceEvent` reusing the existing collector, without introducing a new store. -->
