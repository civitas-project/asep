---
name: using-asep
description: Use when a legacy project still references the ASEP bootstrap skill and the current activation rules must be located.
---

# using-asep (compatibility reference)

This skill is retained for compatibility with projects that still list it. It is not
required to start a session and is no longer installed as an ambient skill.

Activation rules, mandatory constraints and skill discovery live in `protocol/CORE.md`.
Native tools handle ordinary reads, search, edits, compilation and tests. An ASEP
capability activates only when the activation contract applies.

Canonical rules: `COV-01`..`COV-06`, `Tier T0`..`Tier T4`, exit `3` are defined in
`spec-engine/policy/asep-policy.toml`; skills reference IDs and do not restate patterns.
