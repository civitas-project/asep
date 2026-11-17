# ADR-0001: Decoupled Modular SDLC & 2-Phase Lifecycle Architecture

- **Status:** Accepted
- **Date:** 2026-10-15
- **Deciders:** da578 & AI Assistant (Antigravity)
- **Trace Tag:** `@adr:0001`
- **Associated Requirement:** `\@req:FR-ARCH-01:MUST`

---

## 1. Context & Problem Statement
The original ASEP protocol operated as a monolithic "God-Protocol" tightly coupled with companion protocols: TATANG (meta-specs), CODET (low-level code documentation), DUDUNG (design tokens), and UJANG (repository governance). 

Executing any task required running multiple 6-question elicitation interviews across all protocols, while automated checkers demanded 1:1 traceability tags even on internal helpers and experimental prototypes. This resulted in severe cognitive fatigue, massive token waste, and friction during iterative discovery. Furthermore, the toolchain prescribed an external custom Zig compiler (`asep_check`) that was absent from most developer environments, causing continuous integration gate failures.

---

## 2. Decision Drivers
- **Elimination of Cognitive Deadlocks:** Removing multi-tiered Phase 0 questionnaires and enabling autonomous, independent module execution.
- **Support for Rapid Prototyping:** Permitting unhindered technical experimentation without automated gate harassment.
- **Portability & Zero Phantom Tooling:** Binding validation to standard project-native test runners rather than non-existent external binaries.
- **High-Standard Agent Skills:** Aligning skill authoring with proven patterns from Superpowers (TDD, Hard Gates, Anti-Rationalization) and Antislop (concise operational rules).

---

## 3. Considered Alternatives

### Alternative A: Monolithic Multi-Protocol Unified Pipeline
- **Description:** Maintain all protocols (ASEP, CODET, DUDUNG, UJANG) in a single unified validator that checks code, tokens, docstrings, and governance in one run.
- **Technical Drawbacks:** Enforces unnecessary dependencies (e.g., backend CLI projects forced to author KDL design tokens for DUDUNG). A failure in one domain halts unrelated development.
- **Verdict:** Rejected.

### Alternative B: Script-Based Monorepo Checker (Bun/TypeScript)
- **Description:** Implement `asep_check` as a zero-dependency TypeScript script inside the workspace.
- **Technical Drawbacks:** Imposes a Node/Bun runtime requirement on non-JavaScript repositories (e.g. pure Rust, Go, or Python codebases).
- **Verdict:** Rejected in favor of a universal prebuilt Rust binary CLI (to be implemented in a subsequent phase) while currently relying on native ecosystem test runners.

---

## 4. Chosen Decision & Implementation Strategy
1. **2-Phase SDLC Lifecycle:**
   - **Phase 1 (Spike):** Rapid throwaway prototyping with zero traceability tags and automated gates disabled.
   - **Phase 2 (Production TDD):** Requirements and designs locked in PRD/TDD, tests written first (*RED*), minimal implementation (*GREEN*), CODET contracts applied on refactoring (*REFACTOR*), and pre-merge verification (*GATE*).
2. **Decoupled On-Demand Sub-skills:**
   - ASEP operates 100% independently.
   - CODET is invoked as an implementation sub-skill only for memory-critical or concurrency-sensitive functions.
   - DUDUNG is invoked only when UI/tokens are modified.
   - UJANG is invoked only during repo initialization or governance release audits.
3. **Rich ADR Standard:** All significant architectural decisions follow the structured 5-block format.
4. **Toolchain Independence:** Pre-merge gates rely on native runners (`cargo test`, `bun test`, `pytest`) and markdown inspection.

---

## 5. Consequences & Trade-offs

### Positive (Gains & Unlocked Capabilities)
- Zero elicitation fatigue: Single brainstorming session replaces 24+ questionnaire inquiries.
- Rapid exploration: Developers and AI agents can experiment without tag errors.
- Clean separation of concerns: Backend projects are completely insulated from visual design rules.
- High token efficiency: `skills/asep-engineer/SKILL.md` reduced to 140 lines of high-impact operational directives.

### Negative (Incurred Costs & Known Limitations)
- Developers must maintain discipline when transitioning from Phase 1 (Spike) to Phase 2 (Production) to ensure experimental code is not shipped unverified.
- Traceability tagging requires deliberate interface annotation on public domain boundaries.
