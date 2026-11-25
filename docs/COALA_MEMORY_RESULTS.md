# CoALA Memory Exploration Results — M1/M2/M3

> Branch: `dev`, no commit. Evidence only: existing repo docs and traces; no new instrumentation, no new dependency, no new store.  
> Source: `docs/COALA_MEMORY.md` §4–§6 (M1–M3 spike plan). Policy: AD-D06 native-first ownership.

## M1 — Two cross-session recovery scenarios from this repository

### Scenario 1 — SPSC dry-run-1 wiped by server restart

- Evidence: `harness/e2e/RESULTS.md:32`; `docs/BASELINE_RESULTS.md:176-195` (caveats L193); `docs/FAILURE_LEARNING_RESULTS.md:20,57` (HARNESS-05).
- Run dir: `/tmp/opencode/asep-e2e/dry-run-1/` (ASEP commit `f9ab4d6`, seed `4044884631`).
- What happened: run directory partially wiped by a server restart. Artifacts P1–P5 and the P6 optimization report were reconstructed from controller/judge records, explicitly marked `RECONSTRUCTED`, and are **not byte-identical originals**.
- Current state of the run dir: `/tmp/opencode/asep-e2e/` does not exist in this environment (checked) — the wiped temp dir is gone.
- Recovered product state: the four ASEP tooling/harness defects found during the run were fixed in commits `314e3ea` and `bc7cd96`, and the P3 fixture fix under controller ruling `d97d907` — all in git history, inside the repository.

### Scenario 2 — SPSC ab-spsc-a lost on server restart

- Evidence: `docs/BASELINE_RESULTS.md:207,219`; `docs/FAILURE_LEARNING_RESULTS.md:21,51` (BASE-05).
- Original run dir: `/tmp/opencode/asep-e2e/ab-spsc-a`, seed `927466`; replaced by `ab-spsc-a2`.
- What happened: original `ab-spsc-a` was lost when the server restarted; `ab-spsc-a2` and runs B/C/D were generated from the same **reconstructed** baseline, explicitly not byte-identical to the original. The loom proof was not rerun in the reconstructed run.
- Recovered product state: stress 100k ops passed, `asep harness verify` → `pass=true hash_lock_ok=true`, T4 check and `ci` pass on reconstructed run. Task inputs (`harness/e2e/prompts/p1-requirements.md` … `p4-implement.md`) live inside the repository, so the task itself is re-derivable from repo artifacts.

## M2 — Classification: sufficient or insufficient?

| Scenario | Repo artifacts sufficient for necessary engineering state? | Gap |
|---|---|---|
| 1 dry-run-1 | Partially. Product engineering state (the four tooling fixes and their commit history) is inside the repository and was recovered. | **Insufficient for run artifacts.** Original P1–P5 outputs, per-phase judge verdicts, seeds, and hash files lived outside the repo in `/tmp/opencode/asep-e2e/` and were wiped. Reconstruction was non-byte-identical and relied on records not stored in the repository. |
| 2 ab-spsc-a | Partially. The task inputs (P1–P4 prompts) and the recovered green result are reproducible from repo artifacts + rerun. | **Insufficient for original run evidence.** Original run dir and loom proof are lost; only the reconstructed baseline remains, not byte-identical. |

**Gap existed in both scenarios.** The missing knowledge item is identical for both:

- **Missing item**: durable cross-session checkpoint of run artifacts — phase outputs (P1–P5), per-phase judge verdicts, seeds, controller rulings, hash files from `/tmp/opencode/asep-e2e/<run-id>/`.
- **Owner per AD-D06**: none assigned. AD-D06 decided native-first ownership but left the operational contract (persistence, retrieval, source of truth, invalidation, recovery) undetermined (`ASEP_Architectural_Decisions.md:17`, `docs/DOUBT.md:15` DECIDED policy, implementation deferred). The native runtime explicitly disclaims it: AD-D07 defers compaction/session hooks and retains only a non-crash-safe periodic fallback (`docs/DOUBT.md:16`).
- **Why native runtime could not preserve it**: run directories live outside the repository in `/tmp`, are wiped on server restart, and no runtime hook persists checkpoints to repo artifacts (AD-D07 deferred; `docs/COALA_MEMORY.md:31` notes checkpoints are not crash-safe unless a runtime hook exists).

## M3 — Decision

**Default outcome: no new memory engine.**

- No vector store, embedding model, custom memory engine, or new module exists in the repository, and none is justified by this evidence (`docs/COALA_MEMORY.md:14,33`).
- All CoALA classes map to existing owners: working memory → native runtime; episodic → session history (runtime-owned), with only durable engineering checkpoints across sessions as the ASEP gap; semantic → repository facts (PRD/TDD/TRACEABILITY/ADR); procedural → existing skills/prompts/runbooks (`docs/COALA_MEMORY.md:7-13`).
- Necessary engineering state was recovered from repository artifacts in both scenarios (git commits for fixes; `harness/e2e/prompts/` for task inputs), so repository artifacts alone were sufficient for the engineering decisions that matter.

**AD-D06 reopening criterion (documented with evidence):**

- Criterion (from `docs/BASELINE_MEASUREMENT.md:59` and `docs/COALA_MEMORY.md:39`): a **reproducible failure to preserve or recover necessary engineering state using existing repository artifacts**.
- Evidence on hand: two real server-restart wipes of `/tmp/opencode/asep-e2e/` runs where run artifacts were lost and only partially reconstructed (non-byte-identical, loom proof not rerun). This proves the checkpoint gap is real, but the lost items were **run records and evaluative evidence**, not engineering decisions — the fixes and task inputs remained in the repository and were re-derived successfully.
- Verdict: criterion is **not yet triggered as a blocker**. Reopen AD-D06 only when a reproducible failure shows repository artifacts alone cannot recover necessary engineering state. Until then: no new memory engine; document the unowned checkpoint gap under AD-D06 aspects Ownership / Persistence / Recovery.

## Verification of "no duplicate native working memory"

- Checked repository: only `docs/COALA_MEMORY_RESULTS.md` is added; no Rust source changes, no new module, no new store, no new dependency (`Cargo.lock` untouched).
- Working memory remains the native agent runtime only; no second copy of runtime state is written anywhere (`docs/COALA_MEMORY.md:21` boundary rule).
