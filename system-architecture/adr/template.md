# ADR-[NUMBER]: [Title of Decision]

- **Status:** Proposed | Accepted | Superseded by ADR-XXXX | Deprecated
- **Date:** YYYY-MM-DD
- **Deciders:** [Names or Epistemic Roles, e.g., Lead Systems Engineer, Team]
- **Trace Tag:** `@adr:[NUMBER]`
- **Associated Requirement:** `@req:FR-[MOD]-[ID]` (optional if cross-cutting)

---

## 1. Context & Problem Statement
[Describe the technical context, friction, operational failure, or architectural constraint driving the need for this decision. Provide quantitative evidence where possible.]

---

## 2. Decision Drivers
- [Driver 1: e.g., Elimination of tight coupling and cyclic elicitation deadlocks]
- [Driver 2: e.g., Sub-millisecond parsing speed and zero runtime dependency requirement]
- [Driver 3: e.g., Developer cognitive fatigue reduction and token economy]

---

## 3. Considered Alternatives

### Alternative A: [Name of Alternative]
- **Description:** [Brief summary of the alternative approach.]
- **Technical Drawbacks:** [Why this alternative was deemed insufficient or problematic.]
- **Verdict:** Rejected.

### Alternative B: [Name of Alternative]
- **Description:** [Brief summary of the alternative approach.]
- **Technical Drawbacks:** [Why this alternative was deemed insufficient or problematic.]
- **Verdict:** Rejected.

---

## 4. Chosen Decision & Implementation Strategy
[Unambiguous, active-voice declaration of the chosen architecture, library, protocol, or design pattern.]

- **Chosen Approach:** [Name of approach]
- **Implementation Mechanism:** [How it is configured or coded]
- **Enforcement:** [How CI/CD, compilers, or test suites guarantee adherence]

---

## 5. Consequences & Trade-offs

### Positive (Gains & Unlocked Capabilities)
- [Gain 1: e.g., Complete isolation between core backend logic and visual design tokens]
- [Gain 2: e.g., Fast exploratory prototyping without automated checker interference]

### Negative (Incurred Costs & Known Limitations)
- [Cost 1: e.g., Requires explicit transition discipline from Spike mode to Production TDD]
- [Cost 2: e.g., Developers must maintain ADR documents manually or via CLI tooling]
