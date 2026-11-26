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
| O01 | | status = "documented, not adopted" | | | |
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

## Commands

```sh
cargo bench
cargo test --offline --locked
cargo tree --offline --locked
```
