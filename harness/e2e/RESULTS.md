# ASEP E2E Harness — First Dry Run

Run ID: `dry-run-1`
Run dir: `/tmp/opencode/asep-e2e/dry-run-1/`
Seed: `4044884631`
rustc: `1.102.0-nightly (5f4effd57 2026-11-13)`
ASEP commit at bootstrap: `f9ab4d6`

## Outcome

The harness ran an agentic loop (executor + judge per phase) on a dummy project `spsc_ring`, a lock-free SPSC ring buffer for `u64`, from an empty crate to green gates:

- `asep check . --rigor T4` → exit 0, 4 closed MUST, 0 COV violations, 0 docstring violations, `Verification: PASS`.
- `asep ci .` → exit 0. Gates 0–3 passed; Gate 4 skipped because the dummy has no `benches/`.
- `cargo test -p spsc_ring` (debug + release) → all tests passed (4 unit, 6 ring-buffer, 1 stress).
- `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test` → exit 0, 1 loom test explored.
- Final `verify-artifacts.sh` → `pass: true`, `hash_lock_ok: true`, both re-runs passed.

Every phase P1–P7 produced a PASS verdict from the judge, persisted as `NN-judge.json`.

## Real defects found in ASEP (harness/tooling)

1. `verify-artifacts.sh` ran `cargo test` and `asep check` from the run root instead of `dummy/`; fixed in `314e3ea`.
2. Hash lock extraction concatenated the digest with the filename, so a valid single-file `sha256sum` line never matched; fixed in `314e3ea`.
3. The P3 stress fixture moved `AtomicBool`/`AtomicU64` into multiple `move` closures and assumed 512 pushes into a capacity-64 ring; both were compile/logic defects in the fixture, fixed under a recorded controller ruling in `d97d907`.
4. `loom_test.rs` assumed `Mutex::lock()` returns a guard, but loom 0.7.3 returns `LockResult`; the stress fixture also read `manifest.json` from CWD while Cargo runs tests from `dummy/`. Both fixed in `bc7cd96`.

No dummy (ring-buffer) bug was found: stress FIFO/no-loss/no-dup held at 100,000 operations in debug and release, and loom explored the schedule without exposing a defect.

## Honest limitations

- The run directory was partially wiped by a server restart. Artifacts P1–P5 and P6 optimization report were reconstructed from controller/judge records and are explicitly marked `RECONSTRUCTED`. They are not byte-identical originals.
- The judge agent in this harness has no shell permission, so its mandatory re-run obligation was delegated to `verify-artifacts.sh`; the verifier output is authoritative. Judge verdicts are claims, not independent executions.
- The `vendor-loom.sh` proof crate compiles offline but runs 0 loom schedules; the actual loom exploration happened in the dummy project at P5/P7.
- The test-output log contains unused-import/dead-code warnings in the dummy fixtures. They are warnings only; all gates exit 0.
- Two post-TDD test changes and one hash refresh were authorized by recorded controller rulings; without them the run could not have reached GREEN. The test lock was therefore explicitly overridden, with reasons persisted in `controller-ruling.json`.

## Reproduction

```bash
bash harness/e2e/scripts/new-run.sh <run-id>
bash harness/e2e/scripts/vendor-loom.sh /tmp/opencode/asep-e2e/<run-id>
# follow harness/e2e/RUNBOOK.md for P1–P7
bash harness/e2e/scripts/verify-artifacts.sh /tmp/opencode/asep-e2e/<run-id>
```

## Judge verdicts

P1 PASS, P2 PASS, P3 PASS, P4 PASS, P5 PASS, P7 PASS after one scoped re-review that supplied the missing debug/release test logs.
