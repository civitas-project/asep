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

## Summary

| Config | Tasks run | Pass rate | Median tool calls | Median latency | Cost per accepted |
|---|---|---|---|---|---|
| A | 1 | 1/1 | ~5 | not measured | undefined |
| B | 1 | 1/1 | ~3 | not measured | undefined |
| C | 1 | 1/1 | ~4 leader + 1 exec + 1 judge | not measured | undefined |
| D | 1 | 1/1 | ~6 leader + 1 exec + 1 judge + T2 | not measured | undefined |

No conclusion drawn from a single sample.
