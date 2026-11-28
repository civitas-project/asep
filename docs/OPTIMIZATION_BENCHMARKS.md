# Optimization Benchmarks — FR-OPT-00

## Benchmark protocol

<!-- @impl:FR-OPT-00 -->

All optimization candidates O01–O10 are **documented, not adopted**. No candidate may be adopted without before/after benchmark evidence produced under this protocol:

- Same commit: exactly one commit, no source changes between the before and after runs.
- Same fixtures: identical input fixtures for both runs.
- Release build only: measure with `cargo bench` on a release build.
- Warmup: discard at least one warmup iteration before measuring.
- Samples: at least 10 samples per run; report median and p95.
- Allocations: capture allocation delta between before and after runs.
- RSS: capture resident set size before and after.
- Correctness: output must be unchanged (`cargo test` passes and fixture outputs identical).

## Candidates O01–O10

| ID | Candidate | Status | Before | After | Notes |
|----|-----------|--------|--------|-------|-------|
| O01 | 100 sparse 15.26 ms / dense 19.02 ms / cyclic 13.55 ms / same-id 21.62 ms; 1000 sparse 1.2712 s (median, `cargo bench -p asep --bench engine_benchmarks -- affected`, commit 295f7a86) | status = "documented, not adopted" | | 100 sparse 17.873 ms / dense 24.756 ms (median, post-index, `cargo bench -p asep --bench engine_benchmarks -- affected_100`, same commit); cyclic/same-id and 1000/10000 variants not measured (criterion exceeded budget, run killed) | Output identical (20 affected_test + 141 lib tests pass); medians regressed on measured variants (sparse +17.1%, dense +30.0%), not adopted |
| O02 | | status = "documented, not adopted" | | | |
| O03 | | status = "documented, not adopted" | | | |
| O04 | | status = "documented, not adopted" | | | |
| O05 | | status = "documented, not adopted" | | | |
| O06 | | status = "documented, not adopted" | | | |
| O07 | | status = "documented, not adopted" | | | |
| O08 | | status = "documented, not adopted" | | | |
| O09 | | status = "documented, not adopted" | | | |
| O10 | | status = "documented, not adopted" | | | |

## Adoption rule

A row that claims adoption must fill Before and After with measured median/p95, allocations, and RSS on the same commit and fixtures. Current rows claim no adoption, so Before and After remain empty.

## S6 status (simulation and consolidation milestone)

Benchmark execution is explicitly deferred for this milestone. No benchmark suite,
stress test, load test or benchmark gate was run during S0–S5. Consequently:

- O01–O10 remain **optimization hypotheses**, not measured outcomes. No performance
  improvement is claimed.
- O01 was measured before this milestone and is documented above as rejected on the
  available evidence (regressed medians); it is not adopted here. The pre-existing
  working-tree changes that implement the rejected index are left intact and untouched.
- Correctness was verified with bounded targeted tests only (see the S1–S5 reports):
  409 `asep` tests passed across all integration suites except `harness_test`, plus
  the 144-test lib suite and `cargo check --workspace --all-targets --offline --locked`.
- Resource-isolation and safe measurement must be agreed before any benchmark is
  reactivated. Do not silently run `cargo bench` through full CI.

## Commands (deferred; do not run in this milestone)

```sh
cargo bench
cargo test --offline --locked
cargo tree --offline --locked
```
