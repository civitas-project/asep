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

### Phase A — Ambient skill distribution (after Phase M)

<!-- @req:FR-AMB-01:MUST `asep init` shall inject an ambient pointer block into AGENTS.md/CLAUDE.md/GEMINI.md and scaffold a flat `skills/` directory with canonical SKILL.md files, so skills are discoverable without `asep pack`. -->
<!-- @req:FR-AMB-02:MUST `asep pack` shall become provisioning-only (agent roles, permissions, MCP wiring); skill file copying shall be removed (hard remove, no fallback). -->
<!-- @req:FR-AMB-03:MUST A bootstrap skill shall be auto-injected at session start for supported harnesses (superpowers-style), activating ambient skill discovery. -->
<!-- @req:FR-AMB-04:MUST Each SKILL.md shall use description-first discovery (`Use when...`) with a token budget (frequently-loaded skills <= 200 words; descriptions <= 500 chars). -->
<!-- @req:FR-AMB-05:MUST Skills may declare `requires: [<capability>]` metadata; the router shall skip skills whose required capabilities are unavailable to the active agent/model. -->
<!-- @req:FR-AMB-06:MUST `asep check` shall validate ambient skill structure (pointer block presence, flat skills directory, description-first frontmatter, token budget) as part of governance. -->
