# Security Policy & Vulnerability Stewardship

This document defines the policy for Coordinated Vulnerability Disclosure (CVD), response timelines, and supported versions.

---

<!-- @gov:SEC-VERSIONS -->
## 1. Supported Versions

Security updates are provided exclusively for the following active release line:

| Release Line | Support Status | End of Life (EOL) |
| :--- | :--- | :--- |
| `v2.x` | Active Security Support | Current |
| `v1.x` | Maintenance Support | End of 2026 |
| `< v1.0.0` | Deprecated (End of Life) | Unsupported |

---

<!-- @gov:SEC-REPORTING -->
## 2. Reporting a Vulnerability

> [!CAUTION]
> **DO NOT report suspected security vulnerabilities through public Codeberg Issues or public pull requests.** Public disclosure exposes all users to potential attacks before a patch is available.

Please report vulnerabilities through private channels:
- **Security Email:** Send report to `dzulkiflianwar2@gmail.com` with subject line `[SECURITY] ASEP Vulnerability Report`.
- **Codeberg Security:** Open a private issue on Codeberg if repository private reports are enabled.

Please include:
1. Vulnerability description and potential threat impact.
2. Minimal Reproducible Example (PoC) or step-by-step reproduction instructions.
3. Affected versions, operating system, and runtime environment.

---

<!-- @gov:SEC-SLA -->
## 3. Response SLAs (CVSS v3.1)

Maintainers adhere to the following timeline for verified security issues:

| Incident Phase | Target SLA | Mandatory Deliverable |
| :--- | :--- | :--- |
| **Initial Triage** | <= 24 Hours | Written receipt confirmation and preliminary CVSS severity. |
| **Reproduction** | <= 72 Hours | Confirmed reproduction in an isolated test environment. |
| **Private Patch** | <= 14 Days | Verified patch created in a private branch and tested via CI. |
| **Public Release & Disclosure** | <= 30 Days | Release deployment and coordinated public security advisory. |

---

## 4. Attribution

Security researchers who responsibly disclose vulnerabilities in accordance with this policy will be publicly credited in the release notes and security advisory.
