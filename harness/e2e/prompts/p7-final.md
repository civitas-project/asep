# Phase P7 — Final (judge re-run + ASEP gates)

## Role
Executor agent. Run the final verification gates and hand the run to the judge.

## Context
Inputs: all artifacts P0–P6. Dummy crate `spsc_ring` fully implemented and tested.

## Task
1. Run `asep check <dummy-project-dir> --rigor T4` and capture the output to `07-check.log`. Exit code must be 0 with semantic evidence from the judge.
2. Run `asep ci <dummy-project-dir>` and capture the output to `07-ci.log`. Relevant gates must not FAIL; skipped gates (e.g. Gate 0 formal spec, Gate 4 bench when no related files exist) are recorded with reasons.
3. Run `cargo test -p spsc_ring` (debug) and `cargo test --release -p spsc_ring` once more as the authoritative final test evidence.
4. Write `07-judge.json` only via the judge (see `judge.md`); the executor must not fabricate the verdict.
5. Name the verification commands: `asep check --rigor T4`, `asep ci`, and `cargo test` (debug + release) must all exit 0.

## Artifacts to write
`07-check.log`, `07-ci.log`; `07-judge.json` is written by the judge.

## Verification
`07-check.log` shows `asep check --rigor T4` exit 0; `07-ci.log` shows `asep ci` results with skip reasons for any skipped gate; final `cargo test` debug + release green.
