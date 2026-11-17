---
name: design-tokens
description: Use when authoring or auditing KDL design tokens, OKLCH color math, sRGB fallbacks, WCAG contrast, interactive UI states, or ergonomic layout bounds
version: 2.1.0
depends: []
---

# Role: Design Token & Visual Systems Architect (Design-Tokens)

## Overview
Design-Tokens establishes **mathematical, type-safe design token contracts** authored in the KDL Document Language (`.tokens.kdl`) and enforces visual accessibility, contrast, and layout resilience across diagrams, TUI dashboards, and web interfaces. It guarantees unit invariance, perceptual OKLCH color coordinates with deterministic sRGB `fallbackHex`, WCAG AA/AAA contrast ratios, and real interactive state modifiers without poetic naming slop.

---

## When to Use This Skill
- Authoring or auditing design token files (`.tokens.kdl`) and multi-context theme resolvers (`.resolver.kdl`).
- Calibrating perceptual OKLCH color palettes and sRGB `fallbackHex` values for PlantUML/Mermaid diagrams, TUI monitors (`ratatui`), or web UIs.
- Auditing visual contrast (WCAG AA $\ge 4.5:1$ for body text, WCAG AAA $\ge 7:1$ for technical diagrams, WCAG 1.4.11 $\ge 3:1$ for non-text boundaries) and tap-target ergonomics ($\ge 44\text{px}$).
- Enforcing mandatory unit properties on all dimensions and durations.

**When NOT to use:**
- Authoring PRD/TDD requirements prose (use `asep:engineering-specs`).
- Writing Rust/Python function docstrings (use `asep:code-contracts`).
- Managing repository governance files (use `asep:repository-governance`).

---

## Autonomous Token Layering Architecture

Infer token layers directly from the system's visual surface without running unprompted questionnaires:

1. **Foundational Layer (`tokens/base/`):** Raw numeric lightness scales (`50` to `900`), modular spacing steps, and monospace/sans font families.
2. **Semantic Layer (`tokens/semantic/`):** Role-based tokens (`surface-canvas`, `surface-mantle`, `surface-highlight`, `text-primary`, `text-secondary`, `border-subtle`, `feedback-critical`).
3. **Component & State Layer (`tokens/components/`):** Element-specific tokens with explicit interactive state children (`state:hover`, `state:focus`, `state:active`, `state:disabled`).
4. **Resolver Manifests (`resolver/`):** Multi-context theme and density controllers (`.resolver.kdl`).

---

## Core Token & UI Craftsmanship Invariants

### 1. Anti-Slop Functional Naming
- **Forbidden Poetic Metaphors:** Never name tokens `ocean-breeze`, `sunset-coral`, `cyber-neon`, or `forest-mist`.
- **Mandatory Semantic or Numeric Scales:** Use numeric scales (`brand-50` through `brand-900`) or functional roles (`surface-canvas`, `accent-boundary`, `text-primary`).

### 2. Mandatory Unit Invariance
- Every `dimension` must declare an explicit `unit="px"` or `unit="rem"`, including zero values (`0 unit="px"`).
- Every `duration` must declare `unit="ms"` or `unit="s"`.

### 3. Wide-Gamut Fallback & Contrast Invariant
- Any color defined in `oklch`, `display-p3`, `prophoto-rgb`, or `rec2020` **must** declare a deterministic `fallbackHex="#RRGGBB"` attribute:
  ```kdl
  color "accent-actor" colorSpace="oklch" fallbackHex="#89B4FA" {
      components 0.76 0.12 248.0
  }
  ```
- Text tokens against surface tokens must satisfy **WCAG AA ($\ge 4.5:1$)** at minimum, and **WCAG AAA ($\ge 7.0:1$)** for technical diagrams and code/terminal views.

### 4. Interactive, Layout & Mobile Ergonomics (When Building UI / TUI Surfaces)
- **8px Modular Spatial Grid:** Layout spacing, paddings, and margins must strictly follow multiples of 8px (`4 unit="px"`, `8 unit="px"`, `16 unit="px"`, `24 unit="px"`, `32 unit="px"`, `48 unit="px"`, `64 unit="px"`). Arbitrary one-off spacing numbers (e.g., `13px`, `19px`) are rejected.
- **Mobile Responsive Bounds:** Layout containers must fluidly reflow without horizontal scrollbars down to `320 unit="px"` viewport width.
- **Ergonomic Touch/Click Targets:** Interactive controls on web/mobile surfaces must enforce a minimum hit area of `44 unit="px"` by `44 unit="px"`.
- **Complete Interactive States:** Every interactive component token must define `state:hover`, `state:focus`, `state:active`, and `state:disabled`—no dead buttons or unstyled focus rings.
- **Terminal Guard & Clean Restoration (TUI):** Terminal dashboards (`ratatui` / `crossterm`) must use RAII guards (`TerminalGuard`) to restore raw mode and alternate screens cleanly on exit or panic.

---

## Mandatory Rule Tiers

### 1. Hard Gates: Absolute Disqualifications (Zero Tolerance)
- **Zero Dimensionless Quantities:** Any dimension or duration lacking an explicit `unit` attribute is rejected.
- **Zero Wide-Gamut Fallback Omission:** Any `oklch` or `display-p3` token missing `fallbackHex` is rejected.
- **Zero Poetic Color Slop:** Poetic metaphor token names are rejected.
- **Zero Cyclic Alias Loops:** Circular token references ($A \to B \to A$) are rejected.
- **Zero Low-Contrast Text or Dead Controls:** Text failing WCAG AA contrast ($< 4.5:1$) or UI buttons lacking real interactive states/destinations are rejected.

---

## AI Anti-Rationalization Table

| Agent Rationalization / Excuse | Reality & Counter-Directive |
| :--- | :--- |
| *"Unit `px` is obvious for `0`, so I wrote `dimension "none" 0`."* | Strict schema parsers reject unitless numbers. Always write `0 unit="px"`. |
| *"Modern browsers support OKLCH, so `fallbackHex` is unnecessary."* | SVG renderers (PlantUML, Matplotlib), terminals, and legacy PDF pipelines require deterministic sRGB hex values. Always provide `fallbackHex`. |
| *"The UI looks clean, so I don't need to check tap targets or focus states."* | Visual minimalism that omits focus rings or shrinks buttons below 44px breaks accessibility. Enforce full state modifiers and 44px bounds. |

---

## Red Flags: STOP and Restart
- A token uses a poetic name (`sunset-amber`, `midnight-dream`).
- A dimension or duration omits `unit="..."`.
- An `oklch` color node omits `fallbackHex="#..."`.
- Text contrast against its paired surface falls below `4.5:1`.
