# Level Calibration & Rigor Indicators

> Operational rules for assigning Engineer Level (L1/L2/L3) and Engineering Rigor (Lightweight/Structured/High Assurance).  
> Source: RFC v0.2 §3.2, §3.3, §4, §7. Doubts: AD-D02, AD-D03, AD-D04.

## 1. Engineer Level — scope of competence

Level is a **scope classification**, not model size, seniority, or permission. Assigned by the Leader, verified by the Judge.

| Level | Scope | Typical size / blast radius |
|---|---|---|
| L1 | Function, component, implementation | Single function/component; 1 file; no public API change |
| L2 | Module, subsystem, integration | Multiple files within one module/subsystem; public API or integration touched |
| L3 | Architecture, system, strategic decision | Cross-subsystem; new invariant; migration; security boundary |

### Assignment rules
1. **Default to the lowest level that fits.** A one-line fix in a critical system is still L1 (implementation scope), even if rigor is High Assurance.
2. **Level rises with scope, not with risk.** Risk affects rigor, not level.
3. **One agent may hold multiple levels** across tasks; level is per-task, not per-agent.
4. **Judge verifies** the assigned level against the actual diff; mismatch is a blocking finding.

### Quick examples
| Task | Level |
|---|---|
| Fix button padding | L1 |
| Add a unit test | L1 |
| Refactor a module's internal API | L2 |
| Add a new CLI subcommand | L2 |
| Design a transaction engine | L3 |
| Change auth/permission boundary | L3 |
| Migrate storage schema | L3 |

## 2. Engineering Rigor — depth of process

Rule classes: **correctness constraints** (traceability closure, evidence identity, security boundaries) are hard and cannot be relaxed by preference. **Project policies** are hard only when the project adopts them (for example `--deny-drafts`). **Style preferences** such as guard-clause formatting (`AS-CODE-ELSE`, `AS-CODE-NESTING`) are Warning by default, reported without blocking, and promoted only under `--strict` or an explicit project contract.

One canonical rigor classification is `Tier T0`..`T4`, defined in `crates/spec-engine/policy/asep-policy.toml`. `policy::tier_for_rigor_level` maps the three descriptive aliases used in planning documents to that classification: Lightweight -> `T1`, Structured -> `T2`, High Assurance -> `T4`. Aliases never introduce separate mechanisms; they only select a tier. Explicit tiers always take precedence, and the mapping never weakens evidence obligations.

| Tier | Evidence obligation | Typical mechanisms |
|---|---|---|
| `T0` | none | direct execution, relevant local checks |
| `T1` | none | structural check, bounded tests |
| `T2` | required | semantic evidence, regression tests |
| `T3` | required | risk-specific validators, independent review when required |
| `T4` | required | formal/invariant evidence for binding contracts |

Rigor is **independent of engineer level**. A single-line change in auth can be High Assurance (`T4`).

### Escalation triggers
- Security/permission/persistence boundary, hard-to-revert change, cross-subsystem invariant, uncertain root cause, or a new external dependency → escalate at least one tier.
- Formal methods are selected only when a specific property or binding contract requires proof, never by tier name alone.

### De-escalation
- Reduce tier only when all triggers are absent and the change is reversible and well understood; never reduce below an explicit project contract.

## 3. Combined decision table (examples)

| Task | Level | Rigor | Why |
|---|---|---|---|
| Fix button padding | L1 | Lightweight | Single component, reversible |
| Refactor module API (non-critical) | L2 | Structured | Multi-file, integration risk |
| One-line auth check fix | L1 | High Assurance | Security boundary, hard to revert |
| Design transaction engine | L3 | High Assurance | Architecture + invariants |
| Add optional CLI flag | L2 | Lightweight | Reversible, clear scope |
| Migrate DB schema | L3 | High Assurance | Data loss risk |

## 4. Execution Topology

A role is a responsibility, not a permanent agent identity. One agent may fulfill multiple roles, provided mandatory independence constraints remain satisfied.

### Topologies
- **Single Agent (default):** one agent performs Leader/Explorer/Executor/Judge. No separate role instance is created merely to satisfy a nominal workflow.
- **Independent Review (risk-based):** Judge does not implement the change being reviewed; receives requirements, artifacts, and evidence in a separate review context. Different model not mandatory; separation of responsibility is.
- **Selective Delegation (complexity-based):** delegate only when specialization, parallelism, or context separation is expected to provide value. Activating one delegated role does not require activating all others.

### Decision rules
| Condition | Behavior |
|---|---|
| Routine, low-risk, reversible change | Prefer single agent |
| Material uncertainty | Explorer responsibilities; delegate only if beneficial |
| Independently partitionable work | Delegate Executor when expected benefits justify coordination |
| High-risk auth/authorization/payment/data-deletion | Require independent Judge |
| High Assurance workflow | Require independent Judge |
| Explicit independent acceptance requirement | Require independent Judge |
| Mandatory independent Judge unavailable | Acceptance remains unverified; do not claim acceptance |

Rules requiring independent review take precedence over the single-agent default.

### Verification semantics
- Acceptance evidence does not automatically require an independent Judge; deterministic checks (compiler, tests, linters) can suffice where independent review is not mandatory.
- Judge independence requires separation from implementation responsibility and a separate review context.
- The Judge gives an advisory verdict only; only the trusted local runner grants `ACCEPTED` (AD-D09).
- A mandatory independent review must not be bypassed due to time, budget, or runtime limitations.

## 5. Resource Budget

ASEP does not replace native runtime resource management; it may express optional additional constraints.

| Parameter | Default | Conservative preset |
|---|---:|---:|
| Token budget | `None` (no ASEP limit) | 100,000 |
| Tool call budget | `None` | 50 |
| ASEP enforcement | Disabled | Requested, subject to runtime capability |

`None` means no additional ASEP-imposed limit; native runtime and provider limits still apply.

### Behavioral contract
- Without an explicit budget, rely on native runtime limits.
- When configured, enforce through available native mechanisms or supported measurement capabilities.
- On exhaustion, stop initiating new work and persist progress where possible.
- When measurement/enforcement is unavailable, report the limitation as unsupported; do not claim a hard cap was enforced.
- Budget exhaustion must not waive mandatory verification; if verification cannot finish, record acceptance as incomplete/unverified.
- Preset values are provisional and require empirical calibration.

## 6. Governance

- Leader assigns level + rigor + topology before delegation; documents in task objective.
- Judge verifies assignment against diff and triggers; mismatch → blocking finding.
- Default: single agent, Lightweight, relevant verification. Add mechanism only when benefit is provable (AX-05).
- This document is additive; it does not change existing CLI, MCP, or schema (IN-01-COMPAT precedent).