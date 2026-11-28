---
name: design-system
description: Use when authoring or auditing design-system tokens, visual consistency, accessibility or interaction contracts.
version: 2.2.0
---
<!-- asep-managed-skill -->

# Design System Tokens

Load only for token, color, layout or accessibility work. Semantic tokens and interaction contracts are the focus; a specific file format is conditional on the project's stack.

## Scope
- Semantic tokens: name by role, carry explicit units, and provide fallbacks where a color space is wide-gamut.
- Visual consistency: reuse tokens rather than raw values; document the mapping.
- Accessibility: verify contrast and target sizes against the project's declared standard.
- Interaction states: describe behavior, not just appearance.

## Correctness
Do not impose universal dimensions, palettes or layout grids. Treat format rules (KDL, OKLCH) as optional references unless the project adopts them.

## References
Templates: `templates/design-system/`.

## Canonical Anti-Slop Rules
Rule IDs live in `spec-engine/policy/asep-policy.toml`. Skills reference IDs and do not restate patterns: AS-COPY-HYPE, AS-DOCS-PLACEHOLDER.

## Canonical rule references
Coverage `COV-01`..`COV-06`, rigor `Tier T0`..`Tier T4` and exit `3` for unproven results are defined in the same policy file. Advisory style rules are not correctness contracts unless a project adopts them explicitly.
