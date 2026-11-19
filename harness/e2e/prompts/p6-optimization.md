# Phase P6 — Optimize (relative measurement only)

## Role
Executor agent. Optimize the ring buffer and report only relative performance claims.

## Context
Input: `05-findings.md` (P5). Optimizations under test: modulo → bitmask, cache-line padding, batch push/pop, minimal correct `Ordering`.

## Task
1. Apply exactly one optimization at a time to `spsc_ring`.
2. Measure before/after with the same harness: stress fixture (100_000 ops, fixed seed, 30 s watchdog), debug and release, same machine, same cargo profile settings.
3. Report relative claims only ("release is X% faster than debug", "bitmask version lowers elapsed time by Y% vs modulo version"). Absolute performance claims (absolute ns/op, absolute throughput) are forbidden.
4. All correctness gates must stay green: `cargo test -p spsc_ring` debug + release, and `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test`.
5. Name the verification command: `cargo test -p spsc_ring --release` exits 0 and the loom run exits 0; the optimization report contains only before/after ratios.

## Artifact to write
`06-optimization.md` — relative before/after measurements with the applied optimization(s), no absolute performance claims.

## Verification
`06-optimization.md` contains only relative comparisons; all correctness tests (debug, release, loom) still pass.
