---
name: engineering-specs
description: Use when authoring or auditing requirements, specifications, ADRs or diagrams.
version: 2.2.0
---
<!-- asep-managed-skill -->

# Engineering Specs

Load only when the task concerns requirements, designs, ADRs or diagrams. Create PRD/TDD/ADR artifacts when the objective, a significant decision, or a binding project contract warrants them; routine changes do not require them.

## Scope
- Requirements: active voice, named actor, observable behavior and a quantified constraint where the project demands one. Do not invent requirements to satisfy a gate.
- Designs: state boundaries, dependencies, decisions and trade-offs. Requirement-to-test closure applies only when the project policy requires it.
- ADRs and diagrams: use the skeletons and templates in this domain when requested; vector output is preferred for diagrams.

## Correctness
Distinguish binding project contracts from conditional style preferences. Mark unverified claims UNPROVEN. Never fabricate evidence or acceptance.

## References
Skeletons: `engineering-specs/skeletons/`. Templates: `engineering-specs/templates/`. Methodology (ISO 29148 grammar, Architecture-as-Code, benchmark publication) is optional reference material, not an unconditional workflow.

## Canonical Anti-Slop Rules
Rule IDs live in `spec-engine/policy/asep-policy.toml`. Skills reference IDs and do not restate patterns: AS-DESIGN-VAGUE, AS-DESIGN-TBD, AS-DOCS-PLACEHOLDER, AS-DOCS-WORDY.

## Canonical rule references
Coverage `COV-01`..`COV-06`, rigor `Tier T0`..`Tier T4` and exit `3` for unproven results are defined in the same policy file. Advisory style rules are not correctness contracts unless a project adopts them explicitly.
