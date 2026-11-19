# Phase P5 — Bug hunt (stress + loom)

## Role
Executor agent. Hunt for real bugs in the implemented ring buffer.

## Context
Inputs: `04-impl.diff` (P4). Stress fixture: 100_000 operations per configuration, seed from `manifest.json`, `thread::yield_now()` interleave hook, 30 s watchdog per configuration (hang/livelock = FAIL).

## Task
1. Run the stress fixture under debug (`cargo test -p spsc_ring --test stress_test`) and release (`cargo test --release -p spsc_ring --test stress_test`). Log both runs.
2. Run loom systematically: `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test` with a 60 s timeout per test; capture output to `05-loom.log`. Loom failure or timeout = FAIL.
3. Read the stress and loom logs, classify each observed failure ("dummy bug" vs "ASEP defect"), and write `05-findings.md` with: seed, rustc version, ASEP commit, debug + release results, loom result, and one finding per observed defect with reproduction steps.
4. Maximum two repair rounds per finding; afterwards park the finding with a written decision.
5. Name the verification commands:
   - `cargo test -p spsc_ring --test stress_test` (debug) exits 0.
   - `cargo test --release -p spsc_ring --test stress_test` exits 0.
   - `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test` exits 0 within 60 s.

## Artifacts to write
`05-findings.md`, `05-loom.log` (plus debug/release stress logs in the run record).

## Verification
`05-findings.md` records seed, versions, debug and release outcomes, and loom outcome; `05-loom.log` exists; every finding classified as dummy bug or ASEP defect with a ruling after at most two repair rounds.
