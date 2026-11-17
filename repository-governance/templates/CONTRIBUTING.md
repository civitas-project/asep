# Contributor Guidelines & Execution Pipeline

Thank you for contributing! All contributions follow a mechanical, automated execution pipeline to ensure deterministic quality and contract compliance.

---

## 1. Development Environment Setup
<!-- Specify language runtime, toolchain, and prerequisite tools -->
- **Prerequisites:** `[Language Runtime version]`, `git`
- **Installation:**
  ```bash
  # Install dependencies
  [dependency installation command, e.g., cargo check / npm install / go mod download]
  ```

---

## 2. Branch Naming Conventions
Branch names must bind directly to an explicit requirement or issue identifier:
- **Features:** `feat/FR-[MODULE]-[ID]` (e.g., `feat/FR-AUTH-01`)
- **Bug Fixes:** `fix/BUG-[ID]` (e.g., `fix/BUG-104`)
- **Performance:** `perf/BM-[MODULE]-[ID]` (e.g., `perf/BM-ROUTER-02`)

---

<!-- @gov:CONTRIB-VERIFY -->
## 3. Local Verification Gate
Contributors are barred from opening Pull Requests until the local test gate executes with **Exit Code 0**:

```bash
# Execute local verification suite
[test/ci command, e.g., cargo test / npm test / task ci]
```

CI will automatically reject PRs that fail formatting, linting, or unit tests.

---

## 4. Commit Standards
Commit messages follow Conventional Commits and reference traceability tags where applicable:

```text
feat([scope]): [imperative action description] [@req:FR-[MODULE]-[ID]]

- Explanation of operational behavior change
- Invariant or performance implications
```

Examples:
- `feat(auth): enforce constant-time password comparison [@req:FR-AUTH-01]`
- `fix(ledger): eliminate deadlock during concurrent account settlements [@req:FR-SETTLE-02]`

---

## 5. Pull Request Lifecycle
1. Open a Pull Request targeting the `main` branch.
2. Ensure automated CI checks pass completely.
3. PRs require review and approval from at least one maintainer prior to merge.
