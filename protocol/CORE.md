# ASEP Core Protocol

ASEP owns engineering constraints and evidence contracts. The native runtime owns
inference, ordinary file reads/search/edits, compilation, test execution and delegation
transport. ASEP mechanisms are **available, not mandatory**: a simple request may require
none of them. Use native tools when they are sufficient.

## Activation contract

| Capability | Activate when | Do not activate merely because |
|---|---|---|
| `asep check` | Validation requires ASEP-specific contracts, or a check is explicitly requested | An agent received a request or made an edit |
| `asep affected` | Downstream impact is uncertain and graph data can help, or the user requests it | A file changed |
| `asep context` | Multiple sources need a bounded, composed engineering context | The agent needs to read a file |
| `srcidx` / `srcdump` | Symbol-level or controlled-density extraction adds value over native search | A source file exists |
| Audit | Explicit audit request or an applicable risk/contract | Implementation completed |
| PRD / TDD / ADR | Significant requirements, design or decisions warrant them, or a project contract requires them | A coding task started |
| Formal methods | A specific property warrants proof, or a binding contract requires it | Software was described as complex |
| Independent review | High assurance, specified high-risk change, or explicit review requirement | A role name exists in a workflow |
| Full CI | Required by an acceptance contract or explicitly requested | A local relevant test passed |
| Agent messaging | Native delegation cannot meet an actual exchange need | Multiple agents exist |
| Telemetry | Explicit measurement or diagnostic need | A task executed |

A missing graph or unsupported capability is not proof of no impact. Do not repeat a check
against unchanged inputs; invalidate evidence when its relevant inputs or contract change.

## Mandatory constraints

- A failed or unavailable mandatory check stays failed or unproven. Never fabricate a pass.
- Only the trusted local runner grants `ACCEPTED`. Agent judgement cannot grant it, and a
  local `ACCEPTED` does not authorize a merge. Where independent review is required but
  unavailable, acceptance remains unverified.
- Report missing evidence as incomplete or unverified; never silently waive required
  verification.
- Honor binding project contracts and explicit user instructions. Methodology, technical
  preferences and examples are conditional; correctness, safety and contracts are not.

## Skills and discovery

Domain skills live in `skills/` and are discovered through native metadata. Load a skill
only when its scope matches the task; native execution without any domain skill is valid.
No bootstrap or router is required to start. The retired `using-asep` and `workflow-router`
skills are optional compatibility references, not activation layers.

Canonical rule IDs (`COV-01`..`COV-06`, `Tier T0`..`Tier T4`, exit codes `0`/`1`/`2`/`3`
and `AS-*` anti-slop rules) live in `spec-engine/policy/asep-policy.toml`. Skills reference
IDs and do not restate rule patterns.
