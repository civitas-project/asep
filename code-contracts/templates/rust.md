# Docstring Contract Template: Rust

In Rust, docstrings use triple-slashes `///` for items and `//!` for module-level documentation. Strict Conditionality applies: omit `# Invariants`, `# Errors`, or `# Panics` if not applicable.

---

## 1. Pattern: Infallible Pure Function
```rust
// @impl:FR-MATH-01
/// Computes integer square root using binary search algorithm.
pub fn isqrt(value: u64) -> u64 {
    // implementation
}
```

---

## 2. Pattern: Fallible Function (with `# Errors`)
```rust
// @impl:FR-AUTH-01
/// Verifies JWT bearer token against public key claims.
///
/// # Errors: Returns `AuthError::Expired` if `exp` claim is in the past; `AuthError::InvalidSignature` on key mismatch.
pub fn verify_token(token: &str, key: &PublicKey) -> Result<Claims, AuthError> {
    // implementation
}
```

---

## 3. Pattern: Fallible Function with Non-Obvious Invariant
```rust
// @impl:FR-POOL-01
/// Acquires leased database connection from active pool.
///
/// # Invariants: Caller must release connection before thread termination to prevent pool starvation.
/// # Errors: `PoolError::Timeout` if no connection is available within configured timeout duration.
pub async fn acquire(&self) -> Result<Connection, PoolError> {
    // implementation
}
```

---

## 4. Pattern: Function with Non-Trivial Panic
```rust
/// Slices byte buffer at aligned word boundaries.
///
/// # Panics: If `offset + length > buffer.len()` or if `offset` is not 4-byte aligned.
pub fn slice_word(buffer: &[u8], offset: usize, length: usize) -> &[u8] {
    // implementation
}
```

---

## 5. Pattern: Critical Path (`@criticality:high`)
```rust
// @impl:FR-CORE-01
// @criticality:high
/// Executes atomic state transition for ledger balance.
///
/// # Invariants: Balance conservation law: `debit + credit == 0`.
/// # Errors: `LedgerError::InsufficientFunds` if source balance < transfer amount.
/// # Memory Ownership: Zero-Allocation; executes entirely on borrowed state.
/// # Concurrency: Thread-Safe via internal optimistic concurrency control.
pub fn transfer_balance(source: &mut Account, target: &mut Account, amount: u64) -> Result<(), LedgerError> {
    // implementation
}
```
