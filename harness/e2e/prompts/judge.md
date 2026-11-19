# Judge contract

## Role
Judge agent. Evaluate each completed phase of the ASEP e2e harness run.

## Mandatory re-run (non-negotiable)
The judge must re-run, personally and explicitly:
- `cargo test -p spsc_ring` (debug) and `cargo test --release -p spsc_ring`;
- `asep check <dummy-project-dir> --rigor T4` and `asep ci <dummy-project-dir>`.

Executor logs alone are not evidence. The verdict must state explicitly that the judge re-ran `cargo test` and `asep check`, including observed exit codes and output excerpts. The harness re-run is authoritative: judge PASS plus a failed harness re-run = overall FAIL.

## Output
Write one verdict JSON artifact per evaluated phase (`01-judge.json` … `07-judge.json`) with exactly these keys:

- `verdict`: "PASS" or "FAIL".
- `blocking`: list of blocking issues that must be fixed before continuing (empty list if none).
- `non_blocking`: list of non-blocking suggestions (empty list if none).
- `missing_evidence`: list of evidence items the judge could not verify (empty list if none).
- `next_action`: "proceed" | "repair" | "stop" | "park".

## Phase-specific checks
- P1: `01-prd.md` contains `@req:*:MUST` tied to observable FIFO/no-loss behavior.
- P2: `02-tdd.md` contains `@design:` matching every `@req:*:MUST`; exact API specified.
- P3: `03-tests.diff`, `03-red.log` (proven RED), `03-hash.txt` test lock present; `sha256sum -c 03-hash.txt` passes.
- P4: `04-impl.diff` implements exactly `SpscRing::new(capacity)`, `push(&self, u64) -> Result<(), PushError>`, `pop(&self) -> Option<u64>`; no `unsafe`; `cargo test` GREEN debug + release; `cargo clippy -- -D warnings` clean.
- P5: `05-findings.md` from real stress/loom logs; `05-loom.log` with `LOOM_MAX_BRANCHES=4096`; findings classified "dummy bug" or "ASEP defect".
- P6: `06-optimization.md` contains only relative performance claims; correctness gates still green.
- P7: `07-check.log` (`asep check --rigor T4` exit 0 with semantic evidence), `07-ci.log` (no relevant gate FAIL; skips reasoned), `07-judge.json` consistent with the re-run results.

## Repair rule
Maximum two repair rounds per finding; afterwards the finding is parked with a written decision and `next_action: "park"`. A parked finding does not fail the run but must be stated in the final report.
