# Phase P1 — Requirement (PRD)

## Role
Executor agent. Write the Product Requirement Document for the dummy SPSC lock-free ring buffer.

## Context
Dummy crate `spsc_ring`, payload `u64`, safe Rust only (no `unsafe`). Runs under harness at `/tmp/opencode/asep-e2e/<run-id>/`.

## Task
Write `01-prd.md` in the run directory. The PRD must:

1. State the observable behavior as `@req:*:MUST` traceability requirements, one per requirement, e.g. `@req:fifo:MUST`, `@req:no-loss:MUST`, `@req:no-dup:MUST`, `@req:capacity:MUST`, `@req:safe:MUST`.
2. Tie every `@req:*:MUST` to observable FIFO/no-loss behavior:
   - `push` of `u64` values returns `Ok(())` while the ring has a free slot, `Err(PushError)` when full; nothing is dropped silently.
   - `pop` returns values in strict FIFO order, exactly once, `None` only when empty.
   - Capacity is a power of two; `SpscRing::new(capacity)` panics on non-power-of-two (or as specified in P2).
   - Safe Rust only: `unsafe_code = "deny"` must hold.
3. Name the verification command: `cargo test -p spsc_ring` (debug) and `cargo test --release -p spsc_ring` must exercise these requirements through the stress fixture.
4. Record the seed source: `manifest.json` seed (P0), fixed at 100_000 operations per configuration, 30 s timeout per configuration.

## Artifact to write
`01-prd.md` — PRD containing `@req:*:MUST` requirements tied to observable FIFO/no-loss behavior, plus the verification command.

## Verification
`grep -R "@req:.*MUST" 01-prd.md` returns at least one requirement, and each `@req:*:MUST` maps to an observable assertion in the stress fixture (`harness/e2e/fixtures/stress_test.rs`).
