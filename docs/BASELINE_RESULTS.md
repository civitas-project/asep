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

## Summary

| Config | Tasks run | Pass rate | Median tool calls | Median latency | Cost per accepted |
|---|---|---|---|---|---|
| A | 1 | 1/1 | ~5 | not measured | undefined |
| B | 1 | 1/1 | ~3 | not measured | undefined |
| C | 1 | 1/1 | ~4 leader + 1 exec + 1 judge | not measured | undefined |
| D | 1 | 1/1 | ~6 leader + 1 exec + 1 judge + T2 | not measured | undefined |

No conclusion drawn from a single sample.
