---
name: repository-governance
description: Use when authoring or auditing repository documentation, organization or governance files.
version: 2.2.0
---
<!-- asep-managed-skill -->

# Repository Governance

Load only for repository-level documentation or organization. Produce documents by need, not template completeness.

## Scope
- README: what the project is, when to use it, and a short reading path.
- ARCHITECTURE: components, ownership, boundaries and invariants.
- CONTRIBUTING/SECURITY/MANUAL: only where the project's context requires them.
- Organization follows artifact type at the root and domain internally; do not move files for visual symmetry.

## Correctness
Keep documentation aligned with actual implementation. Historical reports stay historical; do not convert them into normative rules. No fabricated acceptance or coverage claims.

## References
Templates: `repository-governance/templates/`. Diátaxis and SWEBOK structures are optional references, not mandatory outlines.

## Canonical Anti-Slop Rules
Rule IDs live in `spec-engine/policy/asep-policy.toml`. Skills reference IDs and do not restate patterns: AS-COPY-FILLER, AS-COPY-HYPE, AS-COPY-BUZZWORD, AS-DOCS-WORDY.

## Canonical rule references
Coverage `COV-01`..`COV-06`, rigor `Tier T0`..`Tier T4` and exit `3` for unproven results are defined in the same policy file. Advisory style rules are not correctness contracts unless a project adopts them explicitly.
