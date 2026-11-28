# Docstring Contract Template: Go

In Go, docstrings are standard `//` comments directly preceding the exported identifier. Go docstrings begin with the identifier name by language convention, but under the docstring contract rules, they must state the operational behavior without repeating parameter type signatures.

---

## 1. Pattern: Infallible Pure Function
```go
// @impl:FR-STR-01

// SanitizeIdentifier converts arbitrary strings into URL-safe slug representations.
func SanitizeIdentifier(raw string) string {
    // implementation
}
```

---

## 2. Pattern: Fallible Function (Returns error)
```go
// @impl:FR-FILE-01

// ReadConfig parses YAML configuration from the designated filesystem path.
// Errors: Returns ErrFileNotFound if path is missing; ErrMalformedYAML on syntax error.
func ReadConfig(path string) (*Config, error) {
    // implementation
}
```

---

## 3. Pattern: Function with Non-Obvious Invariant & Panic
```go
// @impl:FR-BUF-01

// WritePacket flushes buffered bytes to the underlying raw socket descriptor.
// Invariants: Buffer length must not exceed MaxPacketMTU (1500 bytes).
// Panics: If connection context has already been closed.
func WritePacket(conn net.Conn, packet []byte) (int, error) {
    // implementation
}
```

---

## 4. Pattern: Critical Path Function (`@criticality:high`)
```go
// @impl:FR-MEM-01
// @criticality:high

// RentBuffer claims a recyclable memory slice from the internal sync.Pool.
// Memory Ownership: Zero-Allocation; caller must invoke ReleaseBuffer when finished.
// Concurrency: Thread-Safe across concurrent goroutines.
func RentBuffer(size int) []byte {
    // implementation
}
```
