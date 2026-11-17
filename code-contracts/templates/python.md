# Docstring Contract Template: Python

In Python, docstrings follow PEP 257 format using triple double-quotes (`"""..."""`) placed directly under the function definition. Strict Conditionality applies: omit `Raises:` if function is infallible.

---

## 1. Pattern: Infallible Pure Function
```python
# @impl:FR-SLUG-01
def generate_slug(title: str) -> str:
    """Transform human-readable article title into a canonical URL-safe slug."""
    # implementation
```

---

## 2. Pattern: Fallible Function (with `Raises:`)
```python
# @impl:FR-PARSER-01
def parse_payload(raw_json: bytes) -> dict:
    """Deserialize raw bytes into validated operational payload dictionary.

    Raises:
        PayloadTooLargeError: If raw buffer exceeds 1MB limit.
        SchemaValidationError: If required JSON keys are missing or malformed.
    """
    # implementation
```

---

## 3. Pattern: Function with Non-Obvious Invariant
```python
# @impl:FR-CACHE-01
def evict_lru_entry(cache: dict, max_size: int) -> str:
    """Evict the least recently accessed key to maintain cache bounds.

    Invariants:
        cache size must be strictly > 0 before calling this function.
    """
    # implementation
```

---

## 4. Pattern: Critical Path Function (`@criticality:high`)
```python
# @impl:FR-CIPHER-01
# @criticality:high
def decrypt_aes_gcm(ciphertext: bytes, key: bytes, nonce: bytes) -> bytes:
    """Decrypt authenticated ciphertext using AES-256-GCM.

    Invariants:
        key must be exactly 32 bytes; nonce must be 12 bytes.

    Raises:
        AuthenticationTagError: If message integrity check fails.

    Notes:
        Concurrency: Thread-Safe; stateless execution.
    """
    # implementation
```
