---
name: workflow-router
description: Use when a legacy project still references workflow-router and needs the current activation guidance.
requires: []
version: 2.2.0
depends: []
---
<!-- asep-managed-skill -->

# workflow-router (compatibility reference)

Retired as a mandatory router by decision 07.1. No task-to-skill mapping is required;
native execution without any domain skill is valid. Load a domain skill only when its
scope matches the task. Activation rules and mandatory constraints live in
`protocol/CORE.md`.

Useful procedures were distributed: source intelligence (`srcidx`/`srcdump`) to
`system-architecture`, requirements and design methods to `engineering-specs`, debugging
to `system-architecture` and `code-contracts`, repository documents to
`repository-governance`, and token contracts to `design-tokens`.

## Targeted Verification

Use relevant, bounded tests. `asep affected` is conditional: activate it only when
downstream impact is uncertain and graph data can help, or when explicitly requested.
A Requirement and Plan Phase (`docs/PRD.md`, `docs/TDD.md`) is warranted only by
significant requirements, design or a binding project contract, not by routine work.
Never create .superpowers/ or docs/superpowers/ for this pattern.

## Canonical Anti-Slop Rules

Rule IDs live in `spec-engine/policy/asep-policy.toml`; skills reference IDs and do not
restate patterns: `AS-DESIGN-VAGUE`, `AS-DESIGN-TBD`, `AS-DOCS-PLACEHOLDER`,
`AS-DOCS-WORDY`, `AS-CODE-BANNER`, `AS-CODE-NARRATION`, `AS-CODE-BUZZWORD`,
`AS-CODE-BOXDRAW`, `AS-CODE-NESTING`, `AS-COPY-FILLER`, `AS-COPY-HYPE`,
`AS-COPY-BUZZWORD`.

Canonical coverage and rigor rules `COV-01`..`COV-06` and `Tier T0`..`Tier T4`, with exit
`3` for unproven results, are defined in the same policy file.
