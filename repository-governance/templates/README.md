# {{PROJECT_NAME}}

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]({{CI_PIPELINE_URL}})
[![License](https://img.shields.io/badge/license-{{LICENSE_TYPE}}-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-v{{PROJECT_VERSION}}-orange.svg)](CHANGELOG.md)
[![MSRV](https://img.shields.io/badge/MSRV-{{MSRV_VERSION}}-informational.svg)](Cargo.toml)
[![Quality Gate](https://img.shields.io/badge/spec--check-passed-emerald.svg)]({{CI_PIPELINE_URL}})
[![Option C](https://img.shields.io/badge/Option%20C-100%25%20Verified-blue.svg)](trace_manifest.json)

> **{{SINGLE_SENTENCE_TECHNICAL_DEFINITION}}**

---

<!-- @gov:README-SUMMARY -->
## Executive Overview

{{EXECUTIVE_TECHNICAL_PARAGRAPH}}

---

<!-- @gov:README-HIGHLIGHTS -->
## Architectural Highlights

- **Deterministic Memory Guarantees:** {{MEMORY_MODEL_GUARANTEES}}
- **Concurrency & State Synchronization:** {{CONCURRENCY_MODEL}}
- **Domain Invariance & Design by Contract:** {{CONTRACT_INVARIANTS}}
- **Zero-Drift Architecture:** Physical directories map 1:1 to logical bounded contexts without unmapped directories.

---

<!-- @gov:README-PREREQ -->
## Target Platform & Prerequisites

- **Language Runtime / Compiler:** `{{MSRV_OR_RUNTIME_REQUIREMENT}}`
- **Supported Operating Systems:** Linux (x86_64, aarch64), macOS (Apple Silicon, Intel), Windows (x86_64)
- **External System Libraries:** `{{SYSTEM_DEPENDENCIES}}`

---

<!-- @gov:README-BENCH -->
## Quantitative Verification & Benchmarks

<!-- @gov:BENCH-BASELINE -->
All performance claims are bounded by reproducible micro-benchmarks evaluated against a calibrated hardware baseline:
- **Baseline Hardware:** {{CPU_CORES}} cores @ {{CPU_GHZ}} GHz, {{RAM_GB}} GB RAM, {{STORAGE_TYPE}} NVMe
- **Operating Environment:** {{OS_KERNEL_DISTRO}} (MSRV {{MSRV_VERSION}})

| Operation | Traceability Tag | Workload Benchmark | Latency (p50) | Latency (p99) | Memory / Throughput |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **{{OPERATION_1}}**<br>{{OPERATION_1_DESC}} | `<!-- @gov:BENCH-{{TAG_1}} -->`<br>`@gov:BENCH-{{TAG_1}}` | {{WORKLOAD_1}} | {{P50_1}} | {{P99_1}} | {{ALLOC_OR_THROUGHPUT_1}} |
| **{{OPERATION_2}}**<br>{{OPERATION_2_DESC}} | `<!-- @gov:BENCH-{{TAG_2}} -->`<br>`@gov:BENCH-{{TAG_2}}` | {{WORKLOAD_2}} | {{P50_2}} | {{P99_2}} | {{ALLOC_OR_THROUGHPUT_2}} |

Reproduce benchmarks locally:
```bash
{{BENCHMARK_COMMAND}}
```

---

<!-- @lim:README-LIMITS -->
## Known Limitations & System Limits

- `@lim:LIM-RES-01` Maximum concurrent worker threads bounded to `{{MAX_THREADS}}`.
- `@lim:LIM-BUF-01` Input frame buffer ceiling: `{{MAX_FRAME_BYTES}}`.
- Non-goal: `{{EXPLICIT_NEGATIVE_SCOPE_ITEM}}`.

---

<!-- @gov:README-LAYOUT -->
## Workspace Layout (Software Configuration Items)

```text
.
├── Cargo.toml               # [@gov:CONFIG-MANIFEST] Root build and dependency declarations
├── src/                     # [@gov:MOD-CORE] Core domain implementation
│   ├── main.rs              # Application entrypoint
│   └── lib.rs               # Library root and exported domain API
├── tests/                   # [@gov:MOD-TESTS] Integration and end-to-end verification suites
└── docs/                    # [@gov:MOD-SPECS] Living specifications, PRD, and ADR catalogs
```

### Software Configuration Items (SCI) Traceability Matrix

| Software Configuration Item | Traceability Tag | Canonical Scope | Primary Artifacts |
| :--- | :--- | :--- | :--- |
| **Domain Core**<br>`src/` | `<!-- @gov:MOD-CORE -->`<br>`@gov:MOD-CORE` | Core domain abstractions, entities, and state machines | `main.rs`, `lib.rs`, `domain/*` |
| **Verification Suite**<br>`tests/` | `<!-- @gov:MOD-TESTS -->`<br>`@gov:MOD-TESTS` | Unit, integration, and end-to-end regression suites | `tests/*.rs` |
| **Living Specifications**<br>`docs/` | `<!-- @gov:MOD-SPECS -->`<br>`@gov:MOD-SPECS` | PRDs, TDD architecture models, and Rich ADR archives | `PRD.md`, `TDD.md`, `adr/*` |

---

<!-- @gov:README-QUICKSTART -->
## Quick Start: 5-Minute Proof

```bash
# 1. Clone repository
git clone {{REPOSITORY_URL}}
cd {{REPOSITORY_DIR}}

# 2. Build or compile
{{BUILD_COMMAND}}

# 3. Execute primary command and verify output
{{PRIMARY_EXECUTION_COMMAND}}
```

Expected verifiable output:
```text
{{EXPECTED_VERIFIABLE_OUTPUT}}
```

---

<!-- @gov:README-CONTRIB -->
## Quality Gates & Local Verification

All contributions must pass automated gates with **Exit Code 0** before submitting a pull request:

```bash
# Run complete verification suite locally
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Review our branch naming, commit standards, and configuration baselines in [CONTRIBUTING.md](CONTRIBUTING.md).

---

<!-- @gov:README-LEGAL -->
## Governance, Provenance & Security

| Governance Document | Traceability Tag | Responsibility & Policy Enforcement |
| :--- | :--- | :--- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | `<!-- @gov:ARCH-OVERVIEW -->`<br>`@gov:ARCH-OVERVIEW` | Architectural topology map, crate boundaries, and 5 Non-Negotiable Invariants |
| [CONTRIBUTING.md](CONTRIBUTING.md) | `<!-- @gov:CONTRIB-VERIFY -->`<br>`@gov:CONTRIB-VERIFY` | Contributor SDLC, Dual-Branch git topology (`main`/`dev`), and 5-Gate CI |
| [SECURITY.md](SECURITY.md) | `<!-- @gov:SEC-SLA -->`<br>`@gov:SEC-SLA` | Coordinated Vulnerability Disclosure (CVD), CVSS triage, and SLA response matrix |
| [CHANGELOG.md](CHANGELOG.md) | `<!-- @gov:GOV-RELEASE -->`<br>`@gov:GOV-RELEASE` | Keep a Changelog v1.1.0 semantic versioning and release provenance |
| [LICENSE](LICENSE) | `<!-- @gov:GOV-LICENSE -->`<br>`@gov:GOV-LICENSE` | Permissive open-source {{LICENSE_TYPE}} license and legal IP boundaries |
| [trace_manifest.json](trace_manifest.json) | `<!-- @gov:GOV-MANIFEST -->`<br>`@gov:GOV-MANIFEST` | Dynamic trace manifest and Option C automated repository health gate |
