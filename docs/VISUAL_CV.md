# Visual / Computer Vision — workload-driven pilot design (Layer 2)

> Status: pilot design — no implementation. External UI consumer: `https://github.com/dzul-x64/orbit`. Source: user direction; reopens AD-D11 condition.

## 1. Redundancy audit

| Existing capability | Location | Reuse |
|---|---|---|
| Traceability / impact | `asep_check`, `asep_affected`, graph builder | defines which UI files/tests are affected by a visual change |
| Evidence storage | `.spec-engine/evidence`, observability traces | stores visual verification evidence, no new store |
| Diagram rendering | `asep render` + PlantUML | existing visual artifact pipeline, not pixel comparison |
| MCP tools | 13 tools including source dump/index | supplies source context for a visual task |

Conclusion: ASEP has evidence and traceability infrastructure but no visual-backend capability. Native screenshot/DOM tools are preferred over a custom CV engine (AD-D11, AX-01).

## 2. Consumer profile (Orbit)

Observed from the public repository (2026-11-25): TypeScript/Vite/Bun UI generated from an AI Studio template, with `src/`, `server/`, `public/assets/`, `index.html`, `package.json`. Two commits, no releases. This is a valid small external UI pilot, not yet a production workload.

## 3. Contract (pilot)

- **Input**: a UI change in Orbit (commit diff or screenshot pair), acceptance criteria expressed as observable visual behavior, tolerance.
- **Process**: 
  1. use native screenshot tool available in the runtime to capture before/after;
  2. attach captures as evidence files under the existing evidence directory;
  3. record comparison result (pass/fail) with tolerance and tool used;
  4. link to requirement/test via existing trace relations where applicable.
- **Output**: visual evidence record with source (native tool), tolerance, observed result, and limitations. No new engine.
- **Non-goal**: no embedding CV library, no custom model, no new dependency in ASEP.

## 4. Ownership & dependency

| Item | Owner |
|---|---|
| Screenshot capture | native runtime tool (Orbit side) |
| Evidence record | ASEP repository, existing evidence directory |
| Requirement linkage | existing traceability graph |
| Visual judgement | advisory; trusted runner still grants acceptance |

Dependencies: zero new ASEP dependencies initially. Orbit may use its own existing toolchain.

## 5. Acceptance criteria (pilot)

1. Capture at least one before/after pair from Orbit using a native tool, with recorded command/tool and timestamp.
2. Produce one evidence record linked to a UI behavior and, where possible, an existing requirement/test.
3. Record tolerance and whether the comparison is objective (DOM/native) or subjective (judge).
4. Demonstrate no new ASEP dependency or engine was introduced.
5. Report cost/latency of the native tool; no claim of CV accuracy without measurement.

## 6. Empirical validation

- Compare native screenshot/DOM check vs a manual judge verdict on the same change.
- Record agreement, time, and tool-call count. No accuracy claim beyond the single pilot.

## 7. Spike plan

| Step | Deliverable | Verification |
|---|---|---|
| V1 | Clone/checkout Orbit read-only, identify a small UI change | repository evidence |
| V2 | Native screenshot before/after + evidence record | `asep check T1` unchanged |
| V3 | Decision: native tools sufficient, or document gap for CV backend | evidence-backed |

Visual pilot runs last, after Context Compiler, Failure Learning, and CoALA Memory spikes.
