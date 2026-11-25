# Baseline Measurement Results

> Raw data from controlled comparison (RFC §7.3, `docs/BASELINE_MEASUREMENT.md`).  
> No claims of improvement until all configurations A–D are measured on identical tasks.

## Task 01 — CHANGELOG update for recent features

| Field | Value |
|---|---|
| Task | Add Layer 5, P3.1, P3.2, baseline-plan entries to `CHANGELOG.md` |
| Level / Rigor | L1 / Lightweight |
| Configuration | A — native baseline (single agent, no delegation, no added memory) |
| Date (UTC) | 2026-11-24T22:03:24Z |
| Acceptance | `asep check --rigor T1` PASS |
| Acceptance pass | Yes |
| Defects | 0 |
| Rework count | 0 |
| Tool calls | ~5 (read file, edit, date, check) |
| Token consumption | unavailable (provider not exposing token count) |
| Wall-clock time | not precisely measured in this run |
| Cost per accepted task | undefined (no measured cost source) |
| Cost source | unavailable |

### Notes
- This is the first data point only. Configurations B/C/D not yet run on this task.
- Tool calls counted approximately from this session; not from `TASK_REPORT` because no delegated executor was used (Config A).
- Token/cost remain `unavailable` per IN-01-COST — never guessed as zero.
- Wall-clock time and tool calls should be captured automatically via IN-01 telemetry on a later run for higher fidelity.

## Task 02 — Record baseline execution status in DOUBT.md

| Field | Value |
|---|---|
| Task | Update AD-D05 status in `docs/DOUBT.md` and define Config B operationally |
| Level / Rigor | L1 / Lightweight |
| Configuration | B — explicit context retrieval from repository artifacts before edit |
| Context retrieved | `docs/PRD.md`, `docs/BASELINE_MEASUREMENT.md`, `docs/BASELINE_RESULTS.md`, `docs/DOUBT.md` |
| Acceptance | `asep check --rigor T1` PASS |
| Acceptance pass | Yes |
| Defects | 0 |
| Rework count | 0 |
| Tool calls | ~3 (grep context, two edits) |
| Token consumption | unavailable |
| Wall-clock time | not precisely measured |
| Cost per accepted task | undefined |
| Cost source | unavailable |

### Notes
- Config B differs from A by explicit context retrieval from repository artifacts before editing.
- No persistent memory engine used; repository files are the only memory.

## Task 03 — README transitive impact subsection

| Field | Value |
|---|---|
| Task | Add `### Transitive Impact Analysis` subsection to `README.md` |
| Level / Rigor | L1 / Lightweight |
| Configuration | C — B + bounded executor unit + independent judge review |
| Context retrieved | Repository artifacts + executor/judge delegation |
| Acceptance | `asep check --rigor T1` PASS (77 MUST) |
| Judge verdict | PASS |
| Acceptance pass | Yes |
| Defects | 0 |
| Rework count | 0 |
| Leader tool calls | ~4 (define config, delegate, verify, judge) |
| Executor steps | 1 unit (README.md only) |
| Judge steps | 1 review |
| Token consumption | unavailable |
| Wall-clock time | not precisely measured |
| Cost per accepted task | undefined |
| Cost source | unavailable |

### Notes
- Config C adds coordination overhead (executor + judge) vs Config A/B.
- No rework; judge PASS on first review.
- Single sample; no conclusion on overhead benefit.

## Task 04 — Exact scope value test

| Field | Value |
|---|---|
| Task | Strengthen `EngineerLevel::scope()` test to assert exact strings |
| Level / Rigor | L1 / High Assurance (Config D process applied) |
| Configuration | D — C + risk assessment, structured acceptance, T2 scoped acceptance, independent judge |
| Acceptance | T1 PASS, T2 PASS, accept-scoped exit 0 (77 evidence) |
| Judge verdict | PASS |
| Acceptance pass | Yes |
| Defects | 0 |
| Rework count | 0 |
| Leader tool calls | ~6 (define config, delegate, verify, judge, commit, accept) |
| Executor steps | 1 unit (level.rs only) |
| Judge steps | 1 review |
| Extra gates | T2 + accept-scoped (vs C: T1 only) |
| Token consumption | unavailable |
| Wall-clock time | not precisely measured |
| Cost per accepted task | undefined |
| Cost source | unavailable |

### Notes
- Config D adds T2 scoped acceptance overhead (~one bwrap run) vs Config C.
- No rework; judge PASS on first review.
- Single sample; no conclusion on assurance benefit.

## Controlled Round — identical task, balanced order ABCD/DCBA

Task: add one line to this section describing the run. Acceptance: `asep check --rigor T1` PASS. Telemetry: off (provider token counts unavailable).

| Run | Config | Order | Tool calls | Pass | Rework | Defects |
|---|---|---|---|---|---|---|
| R1 | A | 1 | 2 | yes | 0 | 0 |
| R2 | B | 2 | 3 | yes | 0 | 0 |
| R3 | C | 3 | exec 1 + judge 1 | yes | 0 | 0 |
| R4 | D | 4 | exec 1 + judge 1 + T2 | yes | 0 | 0 |

### Notes
- R1/R2 executed directly; R3/R4 pending delegation. Same task content across all four.
- R3 executed via executor unit + judge review; verdict PASS; no rework.
- This is one sequence only; DCBA and two more repetitions pending.
- No conclusion drawn.
- R4 executed via executor + judge + T1/T2 scoped acceptance; verdict PASS; no rework.

## Controlled Round — DCBA sequence

Same task, reverse order. Acceptance: `asep check --rigor T1` PASS.

| Run | Config | Order | Tool calls | Pass | Rework | Defects |
|---|---|---|---|---|---|---|
| R5 | D | 1 | exec 1 + judge 1 + T2 | yes | 0 | 0 |
| R6 | C | 2 | exec 1 + judge 1 | yes | 0 | 0 |
| R7 | B | 3 | 3 | yes | 0 | 0 |
| R8 | A | 4 | 2 | yes | 0 | 0 |

### Notes
- R5 executed via executor + judge + T1/T2; verdict PASS; no rework.
- R6 executed via executor + judge; verdict PASS; no rework.
- R7 executed directly with context retrieval; verdict PASS; no rework.
- R8 executed directly; verdict PASS; no rework.

## Controlled Round — repetition 2 (ABCD)

| Run | Config | Order | Tool calls | Pass | Rework | Defects |
|---|---|---|---|---|---|---|
| R9 | A | 1 | 2 | yes | 0 | 0 |
| R10 | B | 2 | 3 | yes | 0 | 0 |
| R11 | C | 3 | exec 1 + judge 1 | yes | 0 | 0 |
| R12 | D | 4 | exec 1 + judge 1 + T2 | yes | 0 | 0 |

## Controlled Round — repetition 3 (DCBA)

| Run | Config | Order | Tool calls | Pass | Rework | Defects |
|---|---|---|---|---|---|---|
| R13 | D | 1 | exec 1 + judge 1 + T2 | yes | 0 | 0 |
| R14 | C | 2 | exec 1 + judge 1 | yes | 0 | 0 |
| R15 | B | 3 | 3 | yes | 0 | 0 |
| R16 | A | 4 | 2 | yes | 0 | 0 |

### Notes
- R9-R16: repetitions 2 (ABCD) and 3 (DCBA) complete; C/D runs delegated with judge PASS; A/B runs direct; all pass, no rework.

## Median summary (4 balanced sequences: ABCD, DCBA, ABCD, DCBA)

| Config | Runs | Pass | Coordination per run (median) | Defects | Rework |
|---|---|---|---|---|---|
| A | 3 (R1,R8,R9,R16 → 4 samples) | 4/4 | 2 leader tool calls | 0 | 0 |
| B | 4 (R2,R7,R10,R15) | 4/4 | 3 leader tool calls | 0 | 0 |
| C | 4 (R3,R6,R11,R14) | 4/4 | 1 executor + 1 judge | 0 | 0 |
| D | 4 (R4,R5,R12,R13) | 4/4 | 1 executor + 1 judge + T2 | 0 | 0 |

### Caveats
- Counts are not directly comparable: A/B count leader tool calls; C/D count delegated steps. Different units.
- Token consumption and cost remain `unavailable` (no provider data); no cost-per-accepted-task figure.
- Wall-clock time not precisely measured.
- All configurations passed on this trivial documentation task; no quality or defect differentiation observed.
- No claim that one topology is better. AD-D05 remains evidence-driven; a real task with measurable quality differences is needed.

## Real-task evidence — SPSC harness (dry-run-1)

Source: `harness/e2e/RESULTS.md`, `harness/e2e/RUNBOOK.md`. Run dir `/tmp/opencode/asep-e2e/dry-run-1/`, ASEP commit `f9ab4d6`.

| Field | Value |
|---|---|
| Task | Build `spsc_ring` lock-free SPSC ring buffer from empty crate to green gates (P0–P7) |
| Level / Rigor | L2 / High Assurance (loom, stress, T4) |
| Configuration | C/D-equivalent: executor + judge per phase, harness re-run verification |
| Acceptance pass | Yes — `asep check T4` exit 0, `asep ci` exit 0, tests debug+release pass, loom 1 test pass |
| Defects | 4 found in ASEP tooling/harness (not in ring buffer); 0 ring-buffer defects |
| Rework | Multiple scoped repair rounds under recorded controller rulings |
| Token / cost | unavailable |
| Wall-clock | not recorded in RESULTS.md |

### Caveats
- This is C/D-equivalent only; no A/B run on the same task, so no cross-config comparison.
- Run artifacts P1–P5 were partially reconstructed after a server restart — not byte-identical originals.
- Judge had no shell; verifier output is authoritative (judge verdicts are claims).
- Useful as **real-task** evidence of orchestration cost and defect detection, not as a controlled comparison.

## SPSC A/B runs (AD-D05 closure) — progress

| Run | Config | Phase | Tool calls | Pass | Notes |
|---|---|---|---|---|---|
| AB-A-P1 | A | P1 Requirements (PRD) | ~3 | yes | 5 `@req:*:MUST`, no judge (single agent) |
| AB-A-P2 | A | P2 Design (TDD) | ~2 | yes | 5 `@design:` entries, API specified |
| AB-A-P3 | A | P3 TDD RED | ~5 | yes | stub failed 1 test; hash locked |
| AB-A-P4 | A | P4 Implement | ~4 | yes | safe atomic ring; test passes |
| AB-A-FINAL | A | P5–P7 | ~8 | yes | stress PASS, verify PASS, no judge (single agent) |

Run dir: `/tmp/opencode/asep-e2e/ab-spsc-a2`, seed `927466` (original `ab-spsc-a` was lost when the server restarted; artifacts reconstructed). P1–P7 complete: stress 100k ops passed, `asep harness verify` → `pass=true hash_lock_ok=true`, T4 check and ci pass. Loom proof not rerun in the reconstructed run.

## SPSC A/B/C/D runs — completion

| Run | Config | Seed | Verify | Pass | Coordination |
|---|---|---|---|---|---|
| ab-spsc-a2 | A | 927466 | pass=true, hash_lock_ok=true | yes | single agent, no judge |
| ab-spsc-b | B | 1005694320 | pass=true, hash_lock_ok=true | yes | single agent + context retrieval |
| ab-spsc-c | C | 3821759026 | pass=true, hash_lock_ok=true | yes | executor unit + judge verdict artifacts |
| ab-spsc-d | D | 3623576354 | pass=true, hash_lock_ok=true | yes | executor + judge + T2/T4 |

### Caveats
- A run was reconstructed after temp-dir loss; B/C/D were generated from the same reconstructed baseline. Not byte-identical to the original `dry-run-1`.
- Loom proof not rerun in these runs (only stress + T4 + ci).
- Judge verdicts in C/D are recorded artifacts, not live independent subagent reviews for this comparison.
- Outcome-level data only; no token/cost/wall-clock. No claim of topology superiority.

## Conclusion (for AD-D05)

Measurements are not comparable: A/B count leader tool calls while C/D count delegated steps (different units); token, cost, and wall-clock were never captured; the controlled doc task was trivial; SPSC run A was reconstructed after temp-dir loss and B/C/D were generated from the same reconstructed baseline; C/D judge verdicts are recorded artifacts, not live independent reviews. Therefore **no conclusion** about topology superiority OR equivalence can be drawn. Single agent remains the default per RFC §1 / AD-D02 as a design default, not as an empirically proven optimum. AD-D05 stays **DEFERRED**; reopen only with a real quality-distinguishing workload and captured token/cost/latency.

## Summary

| Config | Controlled samples | Pass rate | Real-task evidence | Median latency | Cost per accepted |
|---|---|---|---|---|---|
| A | 4 | 4/4 | none | not measured | undefined |
| B | 4 | 4/4 | none | not measured | undefined |
| C | 4 | 4/4 | SPSC harness (C/D-equivalent) | not measured | undefined |
| D | 4 | 4/4 | SPSC harness (C/D-equivalent) | not measured | undefined |

No conclusion drawn: units of coordination differ between A/B and C/D; token/cost unavailable; controlled task was trivial; real-task SPSC evidence has no A/B counterpart. AD-D05 remains evidence-driven.
