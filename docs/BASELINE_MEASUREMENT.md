# Empirical Baseline Measurement (P0 / AD-D05)

> Status: Plan — not yet executed.  
> Source: RFC v0.2 §7.3, §8 (P0). Purpose: provide evidence for orchestration overhead and capability value before adopting defaults.

## 1. Principle

No mechanism becomes default without measurable benefit (AX-05). P0 inventory is not an empirical baseline. Claims of improvement require controlled comparison.

## 2. Configurations

Compare identical tasks across four configurations (RFC §7.3):

| Config | Description |
|---|---|
| A | Native agent/runtime baseline with commonly available tools |
| B | A + explicit context retrieval from repository artifacts (PRD, TDD, TRACEABILITY, DOUBT) before task execution; persistent memory limited to repository files |
| C | B + bounded executor unit and independent judge review before acceptance |
| D | C + explicit risk assessment, structured acceptance criteria, T2 scoped acceptance, and independent judge review |

Control: task requirements, acceptance criteria, repository starting state, model, runtime, resource constraints. Record uncontrolled variables.

## 3. Metrics

| Metric | Purpose |
|---|---|
| Acceptance pass rate | Outcome correctness |
| Defect rate | Quality |
| Token consumption | Context efficiency |
| Tool calls | Interaction overhead |
| Wall-clock time | Completion speed |
| Rework count | Coordination effectiveness |
| Cost per accepted task | Economic efficiency (includes failed attempts; undefined if zero accepted) |

Repeat trials where practical to account for model stochasticity. Use ablation per capability to isolate benefit.

## 4. Task selection

- Use real project tasks (not synthetic benchmarks).
- Mix L1/L2/L3 scopes and Lightweight/Structured/High Assurance rigor.
- Include at least one auth/payment/data-deletion-class task (High Assurance) and one routine task (Lightweight).
- Minimum 10 tasks, repeated across configurations.

## 5. Measurement method

- Use existing observability (IN-01): telemetry on/off, cost metadata with source (`measured`/`estimated`/`provider_reported`/`unavailable`), file-change evidence.
- Record `tool_call_count` and `duplicate_calls` in TASK_REPORT (FR-CAP-05).
- Use `asep trace report` and `asep check --rigor T2` for evidence.
- Store results in `docs/BASELINE_RESULTS.md` (to be created after execution) with raw data, median, and caveats.

## 6. Reopening criteria for deferred decisions

| Decision | Reopening criteria |
|---|---|
| AD-D05 (orchestration overhead) | Comparable pilot workloads + measurable outcomes available. |
| AD-D07 (lifecycle hooks) | Runtime exposes relevant hooks, or periodic fallback demonstrably fails. |
| AD-D11 (visual backend) | Real UI pilot identifies concrete unmet requirement. |
| AD-D12 (migration compatibility) | Migration target, affected artifacts, compatibility requirements known. |
| AD-D06 (memory implementation) | Reproducible failure to preserve/recover engineering state with existing artifacts. |

## 7. Constraints

- No benchmark numbers claimed until executed.
- No new runtime, dependency, or engine.
- Use real projects as evidence source; no synthetic-only claims.
- Full suite reserved for merges to `main`.

## 8. Controlled round protocol

Pilot samples (Tasks 01–04) used different tasks and sequential order; they are not comparable. A valid comparison requires:

1. **Identical task** for all configurations (same file, same change, same acceptance criteria).
2. **Balanced order**: ABCD then DCBA (and reverse on repeat) to cancel sequence effects.
3. **Minimum 3 repetitions** per configuration.
4. **Telemetry on** (IN-01) to capture token/tool/latency automatically where the provider exposes them.
5. Record raw data per run; report medians only; no p95 from small samples (per BUILD-NEXT-03).

Controlled task: add one line to the `Controlled Round` section of `docs/BASELINE_RESULTS.md` describing the run. Acceptance: `asep check --rigor T1` PASS.

## 9. Next step

Run the first balanced sequence (ABCD) on the controlled task, record raw metrics, then repeat with DCBA.