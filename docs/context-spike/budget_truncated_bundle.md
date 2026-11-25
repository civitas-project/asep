# Context Bundle — Budget Truncation Spike (criterion 4.4)

## Changed path
spec-engine/src/runtime/level.rs

## Budget
- ResourceBudget { max_tokens: 100, max_tool_calls: None, max_cost_usd: None, max_latency_ms: None }
- Simulated BudgetTracker from spec-engine/src/runtime/budget.rs (exceeded() -> 'tokens' when tokens > max_tokens)

## Impacted node IDs (all retained; no silent omission)
### Impls (1)
- FR-LEVEL-01 — spec-engine/src/runtime/level.rs:1 (function=None)
### Requirements (1)
- FR-LEVEL-01 — docs/PRD.md:135 (function=None)
### Tests (1)
- FR-LEVEL-01 — spec-engine/src/runtime/level.rs:46 (function=round_trip_serde)
### ADRs (0)
- (none)
### Unmapped (0)
- (none)

## Source excerpts (budget applied)
### `level.rs`  (lang=rust, tokens_est=532, budget max_tokens=100)
TRUNCATED by budget.
budget exceeded (tokens): excerpt omitted

No content silently dropped: all impacted node IDs above are retained in full;
only the source excerpt is omitted with an explicit truncation-status line.

## Commands
- `cargo test --lib -- round_trip_serde --exact`

## Budget status
- recorded tokens: 532
- max_tokens: 100
- exceeded: True (tokens)
- truncation line present: True
- all impacted IDs retained: True

