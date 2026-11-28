---
name: system-architecture
description: Use when defining system boundaries, dependencies, interfaces, invariants or architectural decisions.
version: 2.2.0
---
<!-- asep-managed-skill -->

# System Architecture

Load only for boundary, dependency, interface, invariant or architectural-decision work. Methodologies are optional: spike, vertical-slice TDD, systematic debugging, formal methods and CI each apply only when the task or contract warrants them.

## Scope
- Document boundaries, dependencies and interfaces with explicit trade-offs.
- Invariants: state the property and the evidence that supports it; formal proof is required only for a binding high-assurance contract.
- Debugging: isolate root cause and add a bounded regression test before changing production behavior when the change is nontrivial.
- Verification: choose relevant bounded checks. Full CI and benchmark gates are never automatic.

## Safety
Do not weaken correctness or safety contracts. `#![deny(unsafe_code)]`, lock ordering and storage-engine rules apply only to domains that actually use them; they are not universal project rules.

## References
ADRs: `system-architecture/adr/`. Diagrams: `system-architecture/diagrams/`. The 5-gate pipeline and FXP phases are historical optional procedures, not mandatory steps.

## Canonical Anti-Slop Rules
Rule IDs live in `spec-engine/policy/asep-policy.toml`. Skills reference IDs and do not restate patterns: AS-DESIGN-VAGUE, AS-DESIGN-TBD.

## Canonical rule references
Coverage `COV-01`..`COV-06`, rigor `Tier T0`..`Tier T4` and exit `3` for unproven results are defined in the same policy file. Advisory style rules are not correctness contracts unless a project adopts them explicitly.
