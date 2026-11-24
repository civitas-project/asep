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

## Summary

| Config | Tasks run | Pass rate | Median tool calls | Median latency | Cost per accepted |
|---|---|---|---|---|---|
| A | 1 | 1/1 | ~5 | not measured | undefined |
| B | 1 | 1/1 | ~3 | not measured | undefined |
| C | 0 | — | — | — | — |
| D | 0 | — | — | — | — |

No conclusion drawn from a single sample.
