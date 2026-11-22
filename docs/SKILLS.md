# SKILLS — Canonical Skill Index

This index lists the six canonical ASEP skills. Each skill's canonical source
lives in its own domain folder; see the canonical location rule below.

## Canonical Skills (source of truth)

| Skill | Canonical path | Version | Purpose (from frontmatter `description`) |
|-------|----------------|---------|------------------------------------------|
| code-contracts | `code-contracts/skill/SKILL.md` | 2.1.0 | Use when authoring or auditing function docstrings, enforcing strict conditionality, eliminating comment parroting, or documenting memory ownership and lock hierarchies |
| design-tokens | `design-tokens/skill/SKILL.md` | 2.1.0 | Use when authoring or auditing KDL design tokens, OKLCH color math, sRGB fallbacks, WCAG contrast, interactive UI states, or ergonomic layout bounds |
| engineering-specs | `engineering-specs/skill/SKILL.md` | 2.1.0 | Use when authoring or auditing PRD/TDD specifications, ISO 29148 requirements grammar, Architecture-as-Code contracts, vector SVG diagrams, or cloud benchmark reports |
| repository-governance | `repository-governance/skill/SKILL.md` | 2.1.0 | Use when authoring or auditing root governance files (README.md, ARCHITECTURE.md, CONTRIBUTING.md, SECURITY.md, CHANGELOG.md) or human-centric manuals (MANUAL.md via Diátaxis) |
| system-architecture | `system-architecture/skill/SKILL.md` | 2.1.0 | Use when orchestrating Phase 1 Spikes, Phase 2 Vertical-Slice TDD, Systematic Debugging, Rich ADRs, Alloy/TLA+ formal proofs, or running the 5-Gate CI verification pipeline |
| workflow-router | `workflow-router/skill/SKILL.md` | 2.2.0 | Use at the start of any task to route intent, sequence phases, and orchestrate skills across the ASEP (Axiomatic Systems Engineering Protocol) ecosystem |

## Generated copies (do not edit)

Other `SKILL.md` files under `.opencode/skills/`, `.agents/plugins/asep/skills/`,
and `spec-engine/.agents/plugins/spec-engine/skills/` are **GENERATED copies**
produced by `asep pack`. Those directories are gitignored (`.opencode/`,
`.agents/`, `.spec-engine/` in `.gitignore`) and the copies must not be edited
directly. Edit the canonical `<domain>/skill/SKILL.md` files only, then
regenerate with `asep pack`.

## Canonical location rule

**New skills live in a NEW DOMAIN FOLDER as `<domain>/skill/SKILL.md`; do not
pile skills into a single `skills/` directory.** Each domain folder keeps its
own docs/templates/ADRs next to the skill.

## Embedding and drift guard

`spec-engine/src/pack/embedded.rs` embeds the canonical skill text via
`include_str!` from each `<domain>/skill/SKILL.md`. `asep check` drift
detection is the guard that verifies generated copies match the canonical
text.
