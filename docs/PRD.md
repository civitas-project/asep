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

## Rust build pilot

<!-- @req:BUILD-PILOT-01:MUST The pilot shall compare edit-to-feedback time after a private function edit and an integration test edit, plus sampled peak and retained build storage, against the current ASEP baseline on the same pinned toolchain. -->
<!-- @req:BUILD-PILOT-02:MUST The candidate shall be adopted only if feedback is consistently faster and build storage consistently smaller, with the same scoped test outcomes, no reduction in acceptance scope, and the main build cache left intact. -->

Reference targets, not pass criteria: reduce median function-edit feedback time by 40%, peak disk usage by 30%, and retained artifacts by 40%.

## Rust build follow-up

<!-- @req:BUILD-NEXT-01:MUST Classify allocated bytes under target/debug/build without deleting the main cache. Measure consolidation of one integration-test group and preserve its test cases, assertions, and outcomes. Adopt it only if it reduces build cost without slowing edit feedback beyond observed variation. -->
<!-- @req:BUILD-NEXT-02:MUST Measure actual Cargo dependency build units before and after removing a redundant variant. Keep supported inputs, features, and targeted tests intact. -->
<!-- @req:BUILD-NEXT-03:MUST Compare two versus four Cargo jobs and concurrent versus serial Cargo commands on the same isolated target. Record wall time, lock waits, and disk use without changing global defaults. -->
