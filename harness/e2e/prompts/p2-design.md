# Phase P2 — Design (TDD plan)

## Role
Executor agent. Write the test-driven design document for the dummy SPSC lock-free ring buffer.

## Context
Input: `01-prd.md` (P1) with `@req:*:MUST` requirements. Dummy crate `spsc_ring`, payload `u64`, safe Rust only.

## Task
Write `02-tdd.md` in the run directory. The design must:

1. Contain `@design:` entries that map one-to-one to each `@req:*:MUST` from `01-prd.md`.
2. Specify the exact public API:
   - `SpscRing::new(capacity: usize) -> Self`
   - `push(&self, u64) -> Result<(), PushError>`
   - `pop(&self) -> Option<u64>`
   - `PushError` type (unit-like error is acceptable).
3. Specify the safe concurrency design: `AtomicUsize`/`AtomicU64` only (no `unsafe`, no `Mutex`, no `UnsafeCell`); `head`/`tail` cache-line separated (`#[repr(align(64))]`); publish with `Release`, consume with `Acquire`.
4. Specify the test plan: stress fixture (`100_000` ops, seed from `manifest.json`, `thread::yield_now()` hook, 30 s watchdog) under debug and release; loom under feature `loom-test` with `LOOM_MAX_BRANCHES=4096` and 60 s timeout per test.
5. Name the verification command: `cargo test -p spsc_ring` compiles and the loom test runs with `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test`.

## Artifact to write
`02-tdd.md` — TDD design with `@design:` entries matching every `@req:*:MUST`, the exact API, and the verification command.

## Verification
Each `@req:*:MUST` in `01-prd.md` has a corresponding `@design:` entry in `02-tdd.md`; `grep -c "@design:" 02-tdd.md` equals the requirement count; API signatures match the contract above.
