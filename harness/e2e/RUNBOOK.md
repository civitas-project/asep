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

## P0 entry point

`harness/e2e/scripts/new-run.sh <run-id>` [`RUN_ROOT=<dir>`] — prints the run directory path and writes `manifest.json` (see script header for field contract).
