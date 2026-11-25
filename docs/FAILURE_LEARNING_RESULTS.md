# Failure Learning Evidence Audit — Results L1-L3

> Executed per docs/FAILURE_LEARNING.md §4-§6. Existing evidence only: no new instrumentation, no new dependency, no learning mechanism. Branch `dev`, no commit.

## L1 — Evidence inventory

Three real failure evidence sources already in the repo, read and cited below.

### Source 1 — docs/MCP_PILOT.md (MCP.3 client pilot)

- Results table, lines 14-21: `initialize` accepted; server replied `protocolVersion: 2024-11-05` while client sent `2025-06-18`, no rejection, no negotiation. `tools/list` returned 13 tools. `asep_affected` succeeded with `paths=["src/lib.rs"]`; empty `paths` → JSON-RPC error `-32603`.
- Caveats, lines 24-29: direct stdio JSON-RPC client, not OpenCode's built-in MCP client; native client discovery UI / permission prompts / structured content rendering not exercised. Protocol version mismatch silently tolerated; real client compatibility beyond `2024-11-05` unverified. Token/tool-call overhead not measured.
- Line 31-33: MCP.4 gap assessment — existing tools cover discovery/impact/verification/acceptance/reporting; P3.3 deferred.

### Source 2 — docs/BASELINE_RESULTS.md (baseline measurements + SPSC real-task evidence)

- Task 01-04 tables, lines 8-98: all four configurations report `asep check --rigor T1` PASS, 0 defects, 0 rework; token consumption `unavailable`, cost `undefined`, wall-clock not precisely measured (IN-01-COST: never guessed as zero).
- Caveats, lines 169-174: counts not directly comparable (A/B count leader tool calls; C/D count delegated steps — different units); token/cost unavailable; wall-clock not measured; all configs passed on a trivial documentation task, so no quality/defect differentiation observed.
- Real-task evidence — SPSC harness dry-run-1, lines 176-195: source `harness/e2e/RESULTS.md`, `harness/e2e/RUNBOOK.md`; run dir `/tmp/opencode/asep-e2e/dry-run-1/`, ASEP commit `f9ab4d6`; 4 defects found in ASEP tooling/harness, 0 in ring buffer; acceptance pass (`asep check T4` exit 0, `asep ci` exit 0, tests debug+release pass, loom 1 test pass).
- Caveats lines 191-195: C/D-equivalent only, no A/B on same task; artifacts P1-P5 partially reconstructed after server restart — not byte-identical originals; judge had no shell, verifier output authoritative (judge verdicts are claims).
- SPSC A/B runs, lines 197-207: run `ab-spsc-a` lost on server restart; `ab-spsc-a2` reconstructed from same baseline; seed `927466`; stress 100k ops passed; `asep harness verify` → pass=true hash_lock_ok=true; loom proof not rerun in reconstructed run.
- SPSC A/B/C/D runs, lines 209-222: all four configs pass=true hash_lock_ok=true; B/C/D generated from reconstructed baseline; judge verdicts are recorded artifacts, not live independent subagent reviews; loom not rerun.
- Conclusion, lines 224-226: no conclusion on topology superiority OR equivalence; AD-D05 DEFERRED pending real quality-distinguishing workload with captured token/cost/latency.

### Source 3 — harness/e2e/RESULTS.md (dry-run-1, "Real defects found in ASEP (harness/tooling)")

- Outcome, lines 9-17: `asep check . --rigor T4` → exit 0, 4 closed MUST, 0 COV violations, 0 docstring violations, `Verification: PASS`; `asep ci .` → exit 0; `cargo test -p spsc_ring` debug+release passed (4 unit, 6 ring-buffer, 1 stress); loom 1 test explored; final `asep harness verify` → pass:true hash_lock_ok:true, both re-runs passed.
- Real defects found in ASEP (harness/tooling), lines 21-28:
  1. `asep harness verify` ran `cargo test` and `asep check` from run root instead of `dummy/`; fixed in `314e3ea`.
  2. Hash lock extraction concatenated digest with filename, so valid single-file `sha256sum` line never matched; fixed in `314e3ea`.
  3. P3 stress fixture moved `AtomicBool`/`AtomicU64` into multiple `move` closures and assumed 512 pushes into capacity-64 ring; compile/logic fixture defects, fixed under recorded controller ruling in `d97d907`.
  4. `loom_test.rs` assumed `Mutex::lock()` returns a guard, but loom 0.7.3 returns `LockResult`; stress fixture read `manifest.json` from CWD while Cargo runs tests from `dummy/`; fixed in `bc7cd96`.
  Line 28: no dummy (ring-buffer) bug found — stress FIFO/no-loss/no-dup held at 100,000 ops debug+release.
- Honest limitations, lines 30-36: run directory partially wiped by server restart; artifacts P1-P5 and P6 optimization report reconstructed, marked `RECONSTRUCTED`, not byte-identical originals; judge agent has no shell, mandatory re-run delegated to `asep harness verify` — verifier output authoritative, judge verdicts are claims; two post-TDD test changes and one hash refresh authorized by recorded controller rulings (`controller-ruling.json`) — without them run could not reach GREEN, test lock explicitly overridden.
- Reproduction, lines 38-45: `asep harness new-run <run-id>`, `asep harness vendor-loom ... --offline`, `asep harness verify ...` per `harness/e2e/RUNBOOK.md`.
- Judge verdicts, lines 47-49: P1 PASS, P2 PASS, P3 PASS, P4 PASS, P5 PASS, P7 PASS after one scoped re-review supplying missing debug/release test logs.

## L2 — Quality audit table

Quality gate per docs/FAILURE_LEARNING.md §2: each failure needs (a) concrete reference, (b) measurable outcome, (c) reproducible verification command or recorded inability to reproduce, (d) no self-certification (judge advisory only; trusted runner grants acceptance).

| Failure ID | Source path:section | Concrete reference | Measurable outcome | Reproducible verification / inability | Status |
|---|---|---|---|---|---|
| MCP-01 | docs/MCP_PILOT.md:Results L16-17; Caveats L27 | Client `protocolVersion: 2025-06-18`, server replied `2024-11-05`; no rejection, no negotiation; mismatch silently tolerated | Version numbers differ; negotiation absent | Rerun `asep mcp < requests.ndjson` and compare protocol versions; inability to force negotiation with this client | usable |
| MCP-02 | docs/MCP_PILOT.md:L4; Caveats L26 | Native OpenCode MCP client not available as callable tool; discovery UI, permission prompts, structured content rendering not exercised | None (not exercised) | Unable: native client unavailable in this session | incomplete |
| MCP-03 | docs/MCP_PILOT.md:Caveats L28 | Token/tool-call overhead from real agent session not measured | None | Unable: not exposed by this client | incomplete |
| BASE-01 | docs/BASELINE_RESULTS.md:Caveats L170 | A/B count leader tool calls (~2-6); C/D count delegated steps (exec+judge±T2) | Different units; not directly comparable | Re-read table; reproducible as documented limitation | usable |
| BASE-02 | docs/BASELINE_RESULTS.md:L19,L43,L68,L94; Caveats L171 | Token consumption `unavailable` in all tasks | None | Unable: provider not exposing token count (IN-01-COST) | incomplete |
| BASE-03 | docs/BASELINE_RESULTS.md:L20,L44,L69,L95; Caveats L172 | Wall-clock time not precisely measured | None | Unable: no telemetry captured | incomplete |
| BASE-04 | docs/BASELINE_RESULTS.md:Caveats L173 | All configurations passed on trivial documentation task; 0 defects, 0 rework everywhere | No quality/defect differentiation observed | Re-run controlled task; conclusion: no differentiation possible on this workload | usable |
| BASE-05 | docs/BASELINE_RESULTS.md:L193,L207,L219; Caveats L193 | SPSC run `ab-spsc-a` lost on server restart; `ab-spsc-a2` and B/C/D from reconstructed baseline; not byte-identical | Reconstruction marker present | Unable: originals lost | incomplete |
| BASE-06 | docs/BASELINE_RESULTS.md:L194; Caveats L194,L221 | Judge had no shell; verifier output authoritative; C/D verdicts recorded artifacts, not live independent reviews | Judge verdicts are claims | Unable: no shell permission for judge in this session | usable (as documented limitation) |
| HARNESS-01 | harness/e2e/RESULTS.md:L23 | `asep harness verify` ran `cargo test`/`asep check` from run root instead of `dummy/` | Exit-code/path defect; fixed in `314e3ea` | Re-run `asep harness verify /tmp/opencode/asep-e2e/dry-run-1` on fixed commit | usable |
| HARNESS-02 | harness/e2e/RESULTS.md:L24 | Hash lock extraction concatenated digest with filename; valid `sha256sum` line never matched | Match failure; fixed in `314e3ea` | Re-run hash-lock verification path on fixed commit | usable |
| HARNESS-03 | harness/e2e/RESULTS.md:L25 | P3 stress fixture: `AtomicBool`/`AtomicU64` in multiple `move` closures; 512 pushes into capacity-64 ring | Compile/logic fixture defects; fixed in `d97d907` under controller ruling | Re-run stress fixture (100k ops) on fixed commit | usable |
| HARNESS-04 | harness/e2e/RESULTS.md:L26 | `loom_test.rs` assumed `Mutex::lock()` returns guard; loom 0.7.3 returns `LockResult`; `manifest.json` read from CWD while Cargo runs from `dummy/` | Compile/runtime defect; fixed in `bc7cd96` | `LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test` → exit 0 | usable |
| HARNESS-05 | harness/e2e/RESULTS.md:L32 | Artifacts P1-P5 and P6 optimization report reconstructed after server restart; marked `RECONSTRUCTED` | Not byte-identical originals | Unable: originals wiped | incomplete |
| HARNESS-06 | harness/e2e/RESULTS.md:L33 | Judge has no shell; re-run delegated to `asep harness verify`; verdicts are claims | Judge no shell | Unable: no shell permission | usable (as documented limitation) |
| HARNESS-07 | harness/e2e/RESULTS.md:L36 | Two post-TDD test changes + one hash refresh authorized by recorded controller rulings; test lock overridden | Run could not reach GREEN without overrides | Review `controller-ruling.json` reasons | usable |

Classification counts: 10 usable, 6 incomplete, 0 rejected (16 records: MCP-01..03, BASE-01..06, HARNESS-01..07).

Decisive evidence source per record: observability/trace reports supplied gate pass/fail and hash-lock data; judge verdicts supplied advisory PASS claims only; trusted runner (`asep check`, `asep ci`, `cargo test`, loom) supplied authoritative outcomes.

## L3 — Requirement mapping

Usable failures mapped to existing traceability (docs/TRACEABILITY.md relations; `asep check . --rigor T1` reports 79 closed MUST, 0 broken). Requirement IDs cited plainly; no `@rel:` tag syntax inserted per audit scope.

| Failure ID | Mapped requirement | Resolvable? | Basis |
|---|---|---|---|
| MCP-01 | FR-MCP-01 (MCP protocol handling) | Partial | FR-MCP-01 covers JsonRpcRequest deserialization and malformed input (-32700); existing verifies cover deserialization, not protocol-version negotiation. Version-negotiation gap is documented caveat, not a broken requirement. |
| BASE-01 | none | Unresolved | Unit incomparability is a measurement-design caveat, not a requirement violation; no requirement defines comparable counting units. |
| BASE-04 | none | Unresolved | No requirement mandates a non-trivial workload or quality differentiation in baseline measurement. |
| BASE-06 | none | Unresolved | Judge shell permission is an execution-environment constraint, not a requirement; advisory verdict contract already acknowledged. |
| HARNESS-01 | none | Unresolved | Tooling bug in `asep harness verify`, fixed in `314e3ea`; not traced to any FR requirement (harness is tooling, not product requirement). |
| HARNESS-02 | none | Unresolved | Same as HARNESS-01; fixed in `314e3ea`. |
| HARNESS-03 | none | Unresolved | Fixture defect under controller ruling `d97d907`; not an FR requirement. |
| HARNESS-04 | none | Unresolved | Fixture/loom version defect fixed in `bc7cd96`; not an FR requirement. |
| HARNESS-06 | none | Unresolved | Same as BASE-06; advisory verdict contract. |
| HARNESS-07 | none | Unresolved | Controller ruling override is a governance artifact, not a requirement mapping. |

Unresolved mappings (explicit, per FAILURE_LEARNING.md §4.3): MCP-02, BASE-01, BASE-04, BASE-06, HARNESS-01, HARNESS-02, HARNESS-03, HARNESS-04, HARNESS-06, HARNESS-07. Partial mapping: MCP-01 → FR-MCP-01 (version negotiation not covered by existing verifies).

Mapped count: 0 failures mapped to a broken requirement; all usable failures are either environment/tooling limitations or already-fixed defects with no open requirement gap.

## Candidate learnings (proposed only, not auto-applied)

At most 3, each with evidence citation. None applied in this spike.

1. **Doc rule**: every results document must state a reproducibility command and flag whether artifacts are reconstructed (cite: docs/BASELINE_RESULTS.md L170-173, L193; harness/e2e/RESULTS.md L32-36). Prevents unreproducible or byte-uncertain evidence being reused.
2. **Test**: add MCP protocol-version negotiation check (client vs server `protocolVersion`, rejection on mismatch) to MCP tooling tests (cite: docs/MCP_PILOT.md L16-17, Caveats L27; FR-MCP-01 existing verifies cover deserialization only). Gap: negotiation is unverified.
3. **Doc rule**: judge verdicts are advisory claims unless paired with a verifier exit code; acceptance requires trusted-runner gate (cite: harness/e2e/RESULTS.md L33; docs/BASELINE_RESULTS.md L194; docs/FAILURE_LEARNING.md §2 quality gate (d)).

## Conclusion

Evidence collection is already sufficient: observability (`TraceReport.failures`, metrics with denominators, hash checkpoints), judge verdicts (advisory blocking/non_blocking/missing_evidence), and trusted-runner gates (`asep check`, `asep ci`, `cargo test`, loom, `asep harness verify`) provide concrete references and measurable outcomes for every failure above. No new store, engine, or dependency is needed.

Learning remains a documentation-proposal, not an enforced mechanism: no controlled before/after evidence exists showing a learning changed an outcome (per docs/FAILURE_LEARNING.md §5 and docs/BASELINE_RESULTS.md L224-226). Learnings above are candidates for leader review; apply only after a controlled run demonstrates effect. AD-D05 stays DEFERRED pending a real quality-distinguishing workload with captured token/cost/latency.

## Verification

`asep check . --rigor T1` run on branch `dev`, working tree clean except new `docs/FAILURE_LEARNING_RESULTS.md`:

```
============================================================
              ASEP SDLC Traceability Report                 
============================================================
Discovered 79 requirements across workspace.
------------------------------------------------------------
Summary (rigor: T1): 79 closed MUST | 0 broken MUST | 0 pending SHOULD | 0 DRAFT | 0 docstring contract violations | 0 ambient skills violations

✔ Gate Result: PASSED (Exit Code 0)
Verification: PASS | structural=true behavioral=true semantic=true skipped=["semantic"]
EXIT: 0
```

Gate PASS, Verification PASS structural=true. Doc addition changes no Rust source; no new dependency or instrumentation introduced.
