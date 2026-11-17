# Docstring Contract Template: TypeScript / JavaScript

In TypeScript/JavaScript, docstrings use standard JSDoc/TSDoc syntax (`/** ... */`). Strict Conditionality applies: omit `@throws` or `@remarks` if not applicable.

---

## 1. Pattern: Infallible Pure Function
```typescript
// @impl:FR-FMT-01
/**
 * Formats epoch timestamp into ISO 8601 UTC string.
 */
export function formatTimestamp(epochMs: number): string {
  return new Date(epochMs).toISOString();
}
```

---

## 2. Pattern: Fallible Async Function (with `@throws`)
```typescript
// @impl:FR-HTTP-01
/**
 * Fetches user profile data from remote identity provider.
 * 
 * @throws {NetworkError} If network connection drops or DNS lookup fails.
 * @throws {AuthenticationError} If bearer token is expired or unauthorized (HTTP 401).
 */
export async function fetchProfile(userId: string): Promise<UserProfile> {
  // implementation
}
```

---

## 3. Pattern: Function with Non-Obvious Invariant
```typescript
// @impl:FR-RATE-01
/**
 * Consumes tokens from in-memory token bucket.
 * 
 * @remarks Invariants: `tokens` must be positive integer; bucket refills monotonically.
 * @returns True if tokens were successfully consumed, false if bucket is exhausted.
 */
export function tryConsume(tokens: number): boolean {
  // implementation
}
```

---

## 4. Pattern: Critical Path Function (`@criticality:high`)
```typescript
// @impl:FR-CRYPTO-01
// @criticality:high
/**
 * Derives cryptographic session key using HKDF-SHA256.
 * 
 * @remarks Invariants: Salt length must be >= 32 bytes; info string must be ASCII.
 * @throws {CryptoError} If underlying web crypto subsystem encounters random entropy failure.
 * @remarks Concurrency: Thread-Safe; stateless deterministic derivation.
 */
export async function deriveSessionKey(secret: Uint8Array, salt: Uint8Array): Promise<CryptoKey> {
  // implementation
}
```
