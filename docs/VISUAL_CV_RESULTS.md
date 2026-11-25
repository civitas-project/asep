# Visual / Computer Vision Pilot Results — Orbit (V1–V3)

## 0. Scope and limitations

- Single pilot only. External consumer: `github.com/dzul-x64/orbit` (TypeScript/Vite/Bun, 2 commits, no releases), inspected read-only; no Orbit file was modified.
- `git clone --depth 1 https://github.com/dzul-x64/orbit.git` into `/tmp/opencode/orbit-pilot` was rejected by this environment (`permission.rejected: external_directory`) on two attempts. Fell back to read-only HTTP fetch via `raw.githubusercontent.com` and the GitHub REST API.
- No screenshot capture: no browser is installed (`chromium`, `firefox` absent). Native `import`/`spectacle` binaries exist but cannot render Orbit — no running dev server (`node_modules` absent; installing dependencies is prohibited).
- Evidence is textual/DOM only. No pixel comparison. No CV accuracy claim.

## V1 — Orbit inspection and candidate behavior

- `package.json`: name `react-example`, private; scripts `dev` (`tsx server.ts`), `start`, `build` (`vite build`), `preview`, `lint` (`tsc --noEmit`). Stack: React 19, Vite 8, Tailwind 4, tsx, express.
- `src/`: `App.tsx`, `main.tsx`, `index.css`; directories `components/`, `context/`, `domain/`, `infrastructure/`, `views/`.
- `index.html`: static markup; `<title>ORBIT — Sistem Penjadwalan & Reschedule Perkuliahan</title>`, `<div id="root"></div>`, entry `/src/main.tsx`.
- Git history (2 commits): `01b1733f` Initial commit (2026-11-24T20:45:39Z, tree contains only `README.md`); `92b44842` docs: update PRD, TDD, and FDD architecture (2026-11-24T20:46:28Z, adds `index.html`).
- Selected candidate behavior (small, observable, unmodified): presence/content of `<title>` in served `index.html`, compared across the two commits.

## V2 — Evidence record (native tools only)

Record `V2-01` — Orbit page title, before/after textual DOM comparison

| Field | Value |
|---|---|
| Behavior | `<title>` content of Orbit `index.html` at two commits |
| Source tool | `curl` (HTTP fetch), `grep` (title extraction); no browser |
| Command | `curl -sL --max-time 30 -o /dev/null -w 'HTTP=%{http_code} wall=%{time_total}\n' https://raw.githubusercontent.com/dzul-x64/orbit/<sha>/index.html` and `curl -sL --max-time 30 https://raw.githubusercontent.com/dzul-x64/orbit/<sha>/index.html \| grep -o '<title>[^<]*</title>'` |
| BEFORE commit | `01b1733f8577dfc760032d59215d00459aab1111` (2026-11-24T20:45:39Z) |
| BEFORE result | HTTP=404, wall=0.141869s, title absent (initial commit tree has only README.md; no index.html) |
| AFTER commit | `92b44842dfa1123cb3aea11799b1659fb6c9acf7` (2026-11-24T20:46:28Z) |
| AFTER result | HTTP=200, wall=0.204285s, title=`<title>ORBIT — Sistem Penjadwalan & Reschedule Perkuliahan</title>` |
| Timing total | ~0.346154s wall for the two fetches (BEFORE 0.141869s + AFTER 0.204285s) |
| Tolerance | Byte-exact title equality between commits; title literal must match `ORBIT — Sistem Penjadwalan & Reschedule Perkuliahan` |
| Evidence type | Objective DOM/native textual (served markup), not subjective judge verdict, not pixel screenshot |
| Observed result | CHANGED — index.html added between commits; title present in AFTER, absent in BEFORE. Before/after pair captured. |

Acceptance criteria status (honest):
- Criterion 1 (before/after pair, native tool, command + timestamp): MET as DOM/textual pair (not pixel screenshot).
- Criterion 2 (evidence record linked to UI behavior): MET in this document.
- Criterion 3 (tolerance, objective vs subjective): MET — tolerance recorded; objective textual comparison.
- Criterion 4 (no new ASEP dependency/engine): MET — zero new dependencies, no engine.
- Criterion 5 (cost/latency of native tool): MET — wall times recorded above; no CV accuracy claim.

## V3 — Conclusion

- Native tools suffice for this single textual/DOM pilot: before/after capture of served `index.html` title across two commits, with recorded command, timestamp, tolerance, and wall time. No new ASEP dependency, no new engine.
- Gap documented (not built): pixel screenshot or rendered DOM-diff before/after capture — requires a browser and a running Orbit dev server; neither available without new installations. Exact unmet requirement: rendered/pixel before/after pair of a live Orbit render. No CV backend is built; the gap is recorded only.
- Verdict: native tools sufficient for this textual pilot only; CV backend gap remains for any pixel or rendered visual workload.

## Redundancy audit summary

Existing traceability/evidence infrastructure reused; no new store.

- Traceability/impact: `asep check` plus the traceability graph (`docs/TRACEABILITY.md`) governs which UI files and tests a visual change affects; this record references Orbit's `index.html` as the inspected artifact.
- Evidence storage: `.spec-engine/evidence` exists (`acceptance.json`, `behavioral.json`, `verification.json` + `evidence/` directory); that pattern is reused conceptually, no new evidence store created. The single pilot record lives in this document only.
- Diagram rendering: `asep render` + PlantUML exists but is non-pixel and out of scope.
- MCP tools: 13 source-context tools available; direct HTTP fetch was used instead.
- No new dependency, no new engine, no new store created.

## Verification evidence (`asep check . --rigor T1`)

Exact output from this session:

```
============================================================
              ASEP SDLC Traceability Report                 
============================================================
Discovered 79 requirements across workspace.
------------------------------------------------------------
Summary (rigor: T1): 79 closed MUST | 0 broken MUST | 0 pending SHOULD | 0 DRAFT | 0 docstring contract violations | 0 ambient skills violations

✔ Gate Result: PASSED (Exit Code 0)
Verification: PASS | structural=true behavioral=true semantic=true skipped=["semantic"]
EXIT=0
```
