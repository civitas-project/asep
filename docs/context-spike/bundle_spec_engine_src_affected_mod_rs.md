# Context Bundle — Task 2

## Objective
Understand impact of the affected() transitive traversal implementation on its impls, requirements, and tests.

## Changed path
spec-engine/src/affected/mod.rs

## Impact source
- Tool: `asep affected --json` (MCP: `asep_affected`), run at repo root.
- Source excerpts: MCP `srcdump` on containing directory, mode=signatures, format=ndjson, tokenizer=cl100k_base.

## Impacted node IDs (from affected())
### Impls (10)
- FR-ADRCOV-01 — spec-engine/src/affected/mod.rs:3 (function=None)
- FR-ADRCOV-02 — spec-engine/src/affected/mod.rs:3 (function=None)
- FR-ADRCOV-03 — spec-engine/src/affected/mod.rs:3 (function=None)
- FR-IMPACT-01 — spec-engine/src/affected/mod.rs:1 (function=None)
- FR-IMPACT-02 — spec-engine/src/affected/mod.rs:1 (function=None)
- FR-IMPACT-03 — spec-engine/src/affected/mod.rs:1 (function=None)
- FR-IMPACT-04 — spec-engine/src/affected/mod.rs:1 (function=None)
- FR-IMPACT-05 — spec-engine/src/affected/mod.rs:2 (function=None)
- FR-IMPACT-06 — spec-engine/src/affected/mod.rs:2 (function=None)
- FR-IMPACT-07 — spec-engine/src/affected/mod.rs:2 (function=None)

### Requirements (10)
- FR-ADRCOV-01 — docs/PRD.md:117 (function=None)
- FR-ADRCOV-02 — docs/PRD.md:118 (function=None)
- FR-ADRCOV-03 — docs/PRD.md:119 (function=None)
- FR-IMPACT-01 — docs/PRD.md:105 (function=None)
- FR-IMPACT-02 — docs/PRD.md:106 (function=None)
- FR-IMPACT-03 — docs/PRD.md:107 (function=None)
- FR-IMPACT-04 — docs/PRD.md:108 (function=None)
- FR-IMPACT-05 — docs/PRD.md:109 (function=None)
- FR-IMPACT-06 — docs/PRD.md:110 (function=None)
- FR-IMPACT-07 — docs/PRD.md:111 (function=None)

### Tests (10)
- FR-ADRCOV-01 — spec-engine/src/traceability_tests.rs:9 (function=None)
- FR-ADRCOV-02 — spec-engine/src/traceability_tests.rs:9 (function=None)
- FR-ADRCOV-03 — spec-engine/src/traceability_tests.rs:9 (function=None)
- FR-IMPACT-01 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-02 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-03 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-04 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-05 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-06 — spec-engine/src/traceability_tests.rs:7 (function=None)
- FR-IMPACT-07 — spec-engine/src/traceability_tests.rs:7 (function=None)

### ADRs (0)
- (none)

### Unmapped (0)
- (none)

## Commands (from affected())
- `cargo test --lib`

## Source excerpts (from srcdump signatures mode)
### `mod.rs`  (lang=rust, lines=0, tokens_est=5163)
```

```

## Unmapped changes
- none reported by affected()

## Budget status
- Candidate source tokens (impacted files only, tool-reported cl100k_base tokens_est, summed): 5163
- Markdown wrapper text not separately tokenized (no tiktoken in spike harness).
- Status: explicit truncation none — bundle includes all impacted node IDs and all in-dir impacted source excerpts.

## Coverage check
- Every impacted node ID from affected() is listed above under Impacted node IDs.
