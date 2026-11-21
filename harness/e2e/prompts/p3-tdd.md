# Phase P3 — TDD: RED before implementation

## Role
Executor agent. Write the tests, prove they fail (RED), and lock the test file.

## Context
Input: `02-tdd.md` (P2). Dummy crate `spsc_ring` does not exist yet (or is a stub).

## Task
1. Write the dummy crate tests for `SpscRing` (FIFO, no loss, no dup, full-ring `Err`, empty `None`) mirroring `harness/e2e/fixtures/stress_test.rs`. Emit them as `03-tests.diff` in the run directory.
2. Run `cargo test -p spsc_ring` against the stub/missing implementation and capture the failing run as `03-red.log` — RED must be proven: at least one test fails or the crate does not compile.
3. After the RED proof, lock the tests: compute `sha256sum tests/ring_buffer_test.rs` (the exact test file path used in the dummy crate) and write the hex digest plus filename to `03-hash.txt` in the run directory, e.g.:
   ```
   <64 hex chars>  tests/ring_buffer_test.rs
   ```
4. Name the verification command: `sha256sum -c 03-hash.txt` must succeed, and `cargo test -p spsc_ring` must show RED (failing or non-compiling) before any implementation exists.

## Artifacts to write
`03-tests.diff`, `03-red.log`, `03-hash.txt`.

## Verification
`03-hash.txt` exists and `sha256sum -c 03-hash.txt` exits 0; `03-red.log` contains a failing test or compile error (RED proven); no implementation diff (`04-impl.diff`) exists yet.

## Test lock rule
`03-hash.txt` is the test lock: no test change after this point without a written controller ruling logged in `03-tests-post-tdd.diff`; any unapproved test change makes the judge verdict FAIL.
