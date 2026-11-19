# Phase P4 — Implement ring buffer

## Role
Executor agent. Implement the dummy SPSC lock-free ring buffer to make P3 tests pass (GREEN).

## Context
Inputs: `02-tdd.md` (design) and locked tests from P3 (`03-hash.txt`). Dummy crate `spsc_ring` at `/tmp/opencode/asep-e2e/<run-id>/`.

## Task
1. Implement `spsc_ring` with exactly this public API and nothing more:
   - `SpscRing::new(capacity: usize) -> Self`
   - `push(&self, u64) -> Result<(), PushError>`
   - `pop(&self) -> Option<u64>`
   - Payload `u64`; capacity a power of two.
2. Safe Rust only: `unsafe_code = "deny"` in `Cargo.toml`; no `unsafe`, no `Mutex`, no `UnsafeCell` anywhere in the crate. Atomics only (`AtomicUsize`/`AtomicU64`), `Release` on publish, `Acquire` on consume, head/tail on separate cache lines (`#[repr(align(64))]`).
3. Run `cargo test -p spsc_ring` until all tests pass (GREEN). Also run `cargo test --release -p spsc_ring`; debug and release results both go into the run record.
4. Name the verification command: `cargo test -p spsc_ring` exits 0 (debug + release), `cargo clippy -p spsc_ring -- -D warnings` exits 0, and `grep -R "unsafe" src/` finds nothing.

## Artifact to write
`04-impl.diff` — complete implementation diff of the dummy crate.

## Verification
`git diff` (or diff against empty crate) produces `04-impl.diff`; `cargo test -p spsc_ring` GREEN in debug and release; `cargo clippy -p spsc_ring -- -D warnings` clean; no `unsafe` in the crate.
