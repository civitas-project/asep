# ASEP E2E Harness Runbook

Drives one dummy SPSC lock-free ring-buffer run through phases P0–P7.
Dummy project lives under `/tmp/opencode/asep-e2e/<run-id>/`; never inside the ASEP repo.

## Phases and artifacts

| Fase | Action | Artifact |
| --- | --- | --- |
| P0 Bootstrap | create run dir, record seed, `rustc -V`, ASEP commit, copy asep binary path | `manifest.json` |
| P1 Requirement | agent writes PRD with `@req:*:MUST` | `01-prd.md`, `01-judge.json` |
| P2 Design | agent writes TDD plan with `@design:` | `02-tdd.md`, `02-judge.json` |
| P3 TDD | agent writes test, prove RED, lock test hash | `03-tests.diff`, `03-red.log`, `03-hash.txt` |
| P4 Implement | agent implements ring buffer | `04-impl.diff` |
| P5 Bug hunt | seeded stress, deterministic interleave hook, debug + release, loom | `05-findings.md`, `05-loom.log` |
| P6 Optimize | apply optimizations, relative measurement only | `06-optimization.md` |
| P7 Final | judge re-runs test/check; `asep check` rigor T4 + `asep ci` | `07-judge.json`, `07-check.log`, `07-ci.log` |

## Dispatch order

`lead → executor → judge → verifier`

Each phase is dispatched in that order with a prompt containing the brief path and the report path. The judge writes a verdict artifact; the harness then re-runs the tests and checks itself — the judge's claim is not accepted on executor logs alone.

## Hard stop rule

A `FAIL` verdict from the judge, or a failed re-run of `cargo test` / `asep check` by the harness, stops the run. No further phase runs until the finding is resolved: maximum two repair rounds per finding, after which the finding is parked with a written decision. A parked finding does not fail the run, but the final report must state it explicitly.

## Phase prompts and judge contract

One prompt per phase lives in `harness/e2e/prompts/`: `p1-requirements.md`, `p2-design.md`, `p3-tdd.md`, `p4-implement.md`, `p5-bug-hunt.md`, `p6-optimization.md`, `p7-final.md`. Each prompt names the artifact to write and the verification command. `p3-tdd.md` locks the tests via `03-hash.txt`.

The judge contract is `harness/e2e/prompts/judge.md`. The judge writes one verdict JSON per evaluated phase with exactly the keys `verdict`, `blocking`, `non_blocking`, `missing_evidence`, `next_action`. The judge must explicitly state it re-ran `cargo test` and `asep check` (observed exit codes and output excerpts); executor logs alone are not evidence. The harness re-run is authoritative: judge PASS plus a failed harness re-run of `cargo test` / `asep check` equals overall FAIL.

## P0 entry point

`asep harness new-run <run-id>` [`--root <dir>`] — prints the run directory path and writes `manifest.json` (schema_version, run_id, seed, rustc_version, asep_commit, asep_binary, created_at).
`asep harness vendor-loom <run-dir> --offline` — copies the committed vendored loom tree and runs a bounded offline concurrency proof.
`asep harness verify <run-dir> [--json]` — checks artifacts 01–07, verifies the test hash lock, re-runs `cargo test` and `asep check --rigor T4`, and writes `verify.json`.
