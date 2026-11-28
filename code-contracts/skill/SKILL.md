---
name: code-contracts
description: Use when authoring or auditing function contracts, pre/postconditions, side effects, error semantics or safety.
version: 2.2.0
---
<!-- asep-managed-skill -->

# Code Contracts

Load only for contract authoring or review. Contracts describe observable behavior, not identifier names or documentation boilerplate.

## Scope
- State preconditions, postconditions, side effects, error semantics and panic conditions when they affect callers.
- Omit empty sections. Do not parrot signatures or add phantom sections.
- Ownership, concurrency and unsafe-boundary rules apply only where the code actually uses those mechanisms.

## Correctness
A contract is conditional on the domain: do not impose universal storage, allocator or lock rules. Verify claims with bounded tests; do not treat a comment as evidence.

## References
Templates: `code-contracts/templates/` (Rust, TypeScript, Python, Go, C++). Use a template only when its language matches the task.

## Canonical Anti-Slop Rules
Rule IDs live in `spec-engine/policy/asep-policy.toml`. Skills reference IDs and do not restate patterns: AS-CODE-BANNER, AS-CODE-NARRATION, AS-CODE-BUZZWORD, AS-CODE-BOXDRAW, AS-CODE-NESTING.

## Canonical rule references
Coverage `COV-01`..`COV-06`, rigor `Tier T0`..`Tier T4` and exit `3` for unproven results are defined in the same policy file. Advisory style rules are not correctness contracts unless a project adopts them explicitly.
