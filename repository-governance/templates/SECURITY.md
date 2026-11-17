# Security Policy & Vulnerability Stewardship

We take the security of our software seriously. This document defines our policy for Coordinated Vulnerability Disclosure (CVD), response timelines, and supported versions.

---

<!-- @gov:SEC-VERSIONS -->
## 1. Supported Versions
Security updates are provided exclusively for the following active release lines:

| Release Line | Support Status | End of Life (EOL) |
| :--- | :--- | :--- |
| `v[Current].x` | Active Security Support | `YYYY-MM-DD` |
| `v[Previous].x` | Critical Patches Only | `YYYY-MM-DD` |
| `< v[Previous].0` | Deprecated (End of Life) | Unsupported |

---

<!-- @gov:SEC-REPORTING -->
## 2. Reporting a Vulnerability

> [!CAUTION]
> **DO NOT report suspected security vulnerabilities through public GitHub Issues or public pull requests.** Public disclosure exposes all users to zero-day attacks before a patch is available.

Please report vulnerabilities through one of the following private channels:
- **Private Advisory:** Open a [GitHub Private Security Advisory](https://github.com/) on this repository.
- **Security Email:** Send an encrypted email to `[security@example.com]`.
- **PGP Fingerprint:** `[Optional: PGP Key Fingerprint]`

Please include:
1. Vulnerability description and potential threat impact.
2. Minimal Reproducible Example (PoC) or step-by-step reproduction instructions.
3. Affected versions and environment configuration.

---

<!-- @gov:SEC-SLA -->
## 3. Response SLAs (CVSS v3.1)
Maintainers adhere to the following deterministic timeline for verified security issues:

| Incident Phase | Target SLA | Mandatory Deliverable |
| :--- | :--- | :--- |
| **Initial Triage** | $\le 24\text{ Hours}$ | Written receipt confirmation and preliminary CVSS severity. |
| **Reproduction** | $\le 72\text{ Hours}$ | Confirmed reproduction in an isolated test environment. |
| **Private Patch** | $\le 14\text{ Days}$ | Verified patch created in a private fork and tested via CI. |
| **Public Release & Disclosure** | $\le 30\text{ Days}$ | Release deployment and coordinated public security advisory. |

---

## 4. Bounty & Attribution
Security researchers who responsibly disclose vulnerabilities in accordance with this policy will be publicly credited in the release notes and security advisory.
