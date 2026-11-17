---
name: code-contracts
description: Use when authoring or auditing function docstrings, enforcing strict conditionality, eliminating comment parroting, or documenting memory ownership and lock hierarchies
---

# Role: Low-Level Operational & Contract Documenter (Code-Contracts)

## Overview
Code-Contracts enforces **contract-first, zero-parroting docstrings** governed by **Strict Conditionality (Zero Phantom Sections)** and **SWEBOK 4.4 Design by Contract (DbC)**. It ensures source code documentation states real behavioral postconditions, error variants, panic triggers, memory ownership lifecycles, and concurrency invariants without bloating files with redundant boilerplate.

---

## When to Use This Skill
- Authoring or auditing docstrings (`///`, `//!`, `"""`, `/** */`) for public types, traits, and domain methods.
- Formulating accurate `# Errors`, `# Panics`, `# Invariants`, and `// SAFETY:` blocks.
- Eliminating signature parroting (where a comment merely repeats the function name or parameter types).
- Documenting memory ownership (`Zero-Allocation`, `Caller-Owned`, `Borrowed-View`) and lock-acquisition hierarchies on `@criticality:high` execution paths.
- Writing runnable docstring examples verified by `cargo test --doc` or `pytest --doctest-modules`.

**When NOT to use:**
- Authoring `docs/PRD.md`, `docs/TDD.md`, or `docs/adr/` (use `asep:engineering-specs` or `asep:system-architecture`).
- Authoring `README.md` or `MANUAL.md` (use `asep:repository-governance`).

---

## The Strict Conditionality Standard

### 1. Zero Signature Parroting
A docstring summary must state the **operational effect and systemic purpose**, never paraphrasing the identifier name.
- **Fail:** `/// get_reader: gets a reader for generation.`
- **Pass:** `/// Retrieves an opened SstableReader for generation, loading and caching it on demand.`

### 2. Conditional Section Rules (Zero Ghost Sections)
Include a docstring section **only when the condition holds**:
- **Summary Line:** Mandatory on all exported items (`#![deny(missing_docs)]`).
- **`// @impl:FR-*`:** Placed on the module or primary public domain boundary implementing a PRD requirement. Never scatter across private helpers.
- **`# Invariants`:** Included **only if** there are non-obvious domain or ordering preconditions (such as `UserKey Ascending x SeqNum Descending` or descending lock-acquisition order) not already enforced by the static type system.
- **`# Errors`:** Included **only if** the function returns a `Result` or error union. Never write `# Errors: None` on infallible functions.
- **`# Panics`:** Included **only if** the function can panic or abort. Never write `# Panics: None`.
- **`# Memory Ownership` & `# Concurrency`:** Included on `@criticality:high` hot paths (e.g., `Zero-Allocation` steady-state lookups or lock-free MVCC readers).
- **`// SAFETY:`:** Mandatory immediately above every `unsafe` block inside the hardware quarantine crate (`*-sys`), proving why pointer alignment, lifetime, and aliasing invariants hold.

### 3. Inline Comment Hygiene & Negative Filtering
Source code comments must never narrate obvious syntax:
- **Prohibited Syntax Narration:** Never write comments explaining *what* code syntax does (e.g., `// loop through items`, `// return result`, `// check if admin`, `// instantiate client`).
- **Prohibited Variable Parroting:** Never restate field names or types (e.g., `// id: the user id`).
- **Permitted Invariant Documentation:** Only explain non-obvious *WHY*:
  - Subtle lock-acquisition ordering and deadlock prevention guarantees.
  - Hardware barrier or memory fence semantics.
  - Workarounds for upstream compiler, libc, or kernel driver quirks.
  - Non-intuitive algorithmic trade-offs (e.g., cache line alignment, bitwise tricks).

---

## Canonical Contract Patterns

### Pattern A: Infallible Pure / Accessor Function
```rust
/// Returns the pinned read sequence number for this snapshot.
#[inline]
#[must_use]
pub fn read_seq(&self) -> u64 {
    self.read_seq
}
```

### Pattern B: Fallible Domain Operation (`Result`)
```rust
// @impl:FR-TENANT-01
/// Stores a tenant-scoped key-value mutation into the database.
///
/// # Errors
/// Returns [`EngineError::KeyTooLarge`] if `key` exceeds 252 bytes, or [`EngineError`] if the underlying write fails.
pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), EngineError>
```

### Pattern C: High-Criticality / Ordering Invariant Contract
```rust
/// An internal key stored inside MemTable and SSTable blocks.
///
/// # Ordering Invariant
/// 1. User keys are ordered lexicographically ascending.
/// 2. If user keys are identical, sequence numbers are ordered descending (higher sequence first).
/// 3. If sequence numbers are identical, value kinds are ordered descending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalKey { ... }
```

### Pattern D: Quarantined `unsafe` Contract (`*-sys` Crate Only)
```rust
// SAFETY: `ptr` was allocated by `System.alloc` with the identical `layout` contract.
unsafe { System.dealloc(ptr, layout) }
```

---

## Multi-Language Idiomatic Mapping

- **Rust (`///`, `//!`):** Enforces workspace lints `missing_docs = "deny"`, `broken_intra_doc_links = "deny"`, and `invalid_rust_codeblocks = "deny"`.
- **Python (`"""`):** Google-style docstrings with `Args:`, `Returns:`, `Raises:` (only if raising), and `Invariants:` (only if applicable), as used in `scripts/cloud_runner.py` and `scripts/kaggle_bench.py`.
- **TypeScript (`/** */`):** TSDoc with `@remarks` for invariants and `@throws` only on throwing functions.
- **Go (`//`):** Declarative Godoc block naming the exported identifier, documenting sentinel errors explicitly.
- **C++ (`/** */`):** Doxygen format documenting `@brief`, `@invariant`, and `@throw` contracts.

---

## Mandatory Rule Tiers

### 1. Hard Gates: Absolute Disqualifications (Zero Tolerance)
- **Zero Verbatim Parroting:** Never restate the function name and parameter types as the summary sentence.
- **Zero Ghost Sections:** Writing `# Errors: None`, `# Panics: None`, or attaching `# Errors` to an infallible function is a hard failure.
- **Zero Broken Doctests or Intra-Doc Links:** All `[`Type`]` links and rustdoc code blocks must pass `cargo test --doc` and `cargo doc --no-deps`.
- **Zero Unjustified `unsafe`:** Every `unsafe` block in `*-sys` must have a preceding `// SAFETY:` proof.

---

## AI Anti-Rationalization Table

| Agent Rationalization / Excuse | Reality & Counter-Directive |
| :--- | :--- |
| *"I added `# Panics: None` so the reader knows it is safe."* | Phantom sections waste context tokens and obscure real panics. If a function cannot panic, omit `# Panics` completely. |
| *"I wrote `/// new: creates a new instance` to satisfy `missing_docs`."* | Parroting the identifier adds zero information. State what state or resource the constructor initializes (e.g., `/// Allocates a new TinyLfuCache holding up to capacity blocks.`). |
| *"I put `// @impl:FR-CORE-01` on all 15 helper methods in the file."* | Tag spam breaks 1:1 traceability clarity. Attach `@impl` to the module header or primary public entrypoint. |

---

## Red Flags: STOP and Restart
- A docstring contains `# Errors: None` or `# Panics: None`.
- An infallible function has an `# Errors` section.
- A docstring summary merely repeats the function name in words.
- An `unsafe` block lacks a `// SAFETY:` comment.
