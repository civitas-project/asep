# Doubts — ASEP Adaptive Architecture

> Status tracker for technical doubts (RFC v0.2 §9, plus operational doubts raised during implementation).  
> Each entry: ID, doubt, required evaluation, verdict (`OPEN` / `RESOLVED` / `DEFERRED` / `REJECTED`).

## From RFC v0.2 §9 (AD-D01..AD-D12)

| ID | Doubt | Required evaluation | Verdict |
|---|---|---|---|
| AD-D01 | Redundansi dengan runtime native | Petakan capability yang sudah tersedia pada runtime dan ASEP; identifikasi gap aktual. | DECIDED — reuse native runtime (ReAct, tools, memory); ASEP only adds contracts, budget, evidence. Guardrail for every new slice. |
| AD-D02 | Role overlap | Tentukan kapan satu agent cukup dan kapan independensi Judge memberikan manfaat. | OPEN |
| AD-D03 | Level calibration | Rumuskan task scope dan capability evidence tanpa mengandalkan nama/ukuran model. | DECIDED — rules in `docs/LEVEL_CALIBRATION.md` §1; Leader assigns, Judge verifies. |
| AD-D04 | Rigor misclassification | Tentukan indikator risiko, uncertainty, dan escalation; uji kasus task kecil berisiko tinggi. | DECIDED — triggers in `docs/LEVEL_CALIBRATION.md` §2. |
| AD-D05 | Orchestration overhead | Ukur biaya koordinasi dan kualitas terhadap single-agent baseline. | OPEN |
| AD-D06 | Memory and context overlap | Tetapkan ownership state, retrieval, source of truth, dan mekanisme invalidasi. | OPEN |
| AD-D07 | Lifecycle hook limitations | Audit dukungan compaction/session hooks; rancang fallback yang tidak menjanjikan checkpoint saat crash. | OPEN |
| AD-D08 | Graph duplication | Evaluasi apakah traceability dan source index yang sudah ada dapat menjadi fondasi graph bersama. | DECIDED — reuse `build_relation_graph` and `@rel`; no new graph engine. |
| AD-D09 | Verification independence | Bedakan objective checks, judgement subjektif, dan acceptance manusia; hindari self-certification. | RESOLVED — trusted local runner grants ACCEPTED; agent never grants it (FR-ACC-02..05, FR-INT-01). |
| AD-D10 | Formal methods applicability | Identifikasi properti yang benar-benar membutuhkan model checking, bukan sekadar test biasa. | OPEN |
| AD-D11 | Visual backend cost | Ukur akurasi, toleransi, portability, dan overhead CV terhadap DOM/native screenshot tools. | OPEN |
| AD-D12 | Migration compatibility | Audit dampak penyederhanaan terhadap skill, CLI, MCP, schema, dan existing quality gates. | OPEN |

## Operational doubts raised during IN-01 (resolved)

| ID | Doubt | Resolution |
|---|---|---|
| OP-D01 | Sampling 1/10 (thinning) vs file evidence | Sampling removed entirely; telemetry is on/off (default off); file-change and intent evidence always recorded (IN-01-OVERHEAD, IN-01-FILE-EVIDENCE, IN-01-INTENT). |
| OP-D02 | File-change evidence cost (93x slower) | Evidence is hash twice per span, not per edit; measured on debug build, 10k events: off 5ms vs full 465ms; default remains off. |
| OP-D03 | Silent flush failure in Drop | `close()` returns `Result`; `Drop` warns to stderr; `with()` helper guarantees explicit close (IN-01-COLLECT). |
| OP-D04 | Unbounded buffer (OOM risk) | Auto-flush sync at `max_buffer_events` (default 1000, configurable); `flush` no longer removes buffer until write succeeds (IN-01-COLLECT). |

## Layer 5 (Agent Runtime) — open doubts

- Engineer level (L1/L2/L3) is a scope classification, not a model size or permission grant (RFC §3.2). Need capability evidence to calibrate (AD-D03).
- ReAct loop: reuse native runtime loop; ASEP adds action contracts, observation, budget, evidence (RFC §5.1). Risk of duplicating runtime (AD-D01).
- Resource budgeting: thresholds and escalation need empirical tuning; default values are provisional.
- Audit redundancy (P0) must precede new capability work to avoid duplicating existing mechanisms (FR-CAP, agent_protocol).
