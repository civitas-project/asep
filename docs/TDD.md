# Technical Design Document (TDD)

<!-- @design:FR-CLI-01 -->
## FR-CLI-01 Scanner
`spec-engine/src/scanner/mod.rs` walks the target root with `ignore::WalkBuilder` (gitignore, global gitignore, `.git exclude`, `.asepignore`, `.git` optional), reads each recognized source file line by line, and parses traceability tags with `scanner::lexer::parse_tags_from_line`. Code files (`rs`, `go`, `ts`, `js`, `py`, `cpp`, `c`, `h`, `hpp`, `java`, `cs`) emit `@impl`/`@test` tags only; `@req` declarations are skipped in code files so requirements live in spec/markdown files. Results are collected into `ScannedTag` records with relative paths and 1-indexed line numbers.

<!-- @design:FR-CLI-02 -->
## FR-CLI-02 Check command
`spec-engine/src/runner.rs` implements `asep check`: it scans project tags, builds the canonical relation graph (`graph::builder::build_relation_graph`), evaluates verification (`verification::evaluate`: structural = closed graph, behavioral = `.spec-engine/behavioral.json::passed`, semantic = evidence per tier policy), audits docstring contracts (`auditor::audit_project_docstrings`), checks installed skill drift, and writes `.spec-engine/verification.json`. Exit codes come from `policy::exit_codes()`: Pass=0, Fail=1, Unproven=3, combined deterministically from gate results.

<!-- @design:FR-CLI-03 -->
## FR-CLI-03 Project init, ADR, diagrams
`spec-engine/src/init/mod.rs::init_asep_project` creates `docs/`, `docs/adr/`, `docs/diagrams/rendered/`, `docs/invariants/` and writes a baseline `docs/PRD.md` only when absent. `spec-engine/src/adr/mod.rs::create_new_adr` scaffolds a numbered Rich ADR under `docs/adr/`. `spec-engine/src/diagram/mod.rs::render_plantuml_diagrams` compiles `.puml` files into SVG assets via PlantUML. The CLI exposes these as `asep init`, `asep adr new`, and `asep render` (runner.rs Commands::Init/Adr/Render).

<!-- @design:FR-CLI-04 -->
## FR-CLI-04 Docstring auditor
`spec-engine/src/auditor/doc_contracts.rs::audit_doc_contracts` accumulates Rust doc comment blocks preceding each function and applies the docstring contract: parroting (docstring starts with the function identifier), phantom sections (`# Panics/Errors/Invariants: None ...`), and infallible error sections, each reported with 1-indexed line numbers. `audit_project_docstrings` walks the tree for `.rs` files, skipping tests/, benches/, examples/, templates/, skeletons/ directories. Anti-slop patterns are enforced as blocking violations outside T0 rigor.

<!-- @design:FR-CONFIG-01 -->
## FR-CONFIG-01 Persistent configuration
`spec-engine/src/config/settings.rs` defines `EngineConfig` with `EditorSettings` (preference: Ask/Terminal/Gui, custom_command), an optional `rigor` tier string, and a `CapabilityProfile` (file_read, file_write, terminal, mcp, subagents, web, context_window). `load_config` reads `.spec-engine/config.toml` via TOML and falls back to defaults when missing or corrupt; `save_config` creates the directory tree and writes pretty TOML. The skill `config` subcommand reads/writes editor preference through these functions.

<!-- @design:FR-MCP-01 -->
## FR-MCP-01 JSON-RPC typed protocol
`spec-engine/src/mcp/protocol.rs` defines typed `JsonRpcRequest` (jsonrpc, id, method, params), `JsonRpcResponse` (jsonrpc, id, result, error), and `JsonRpcError` (code, message, data) structs with serde serialization. `spec-engine/src/mcp/server.rs::process_jsonrpc_request` parses each stdin line into `JsonRpcRequest`, routes methods (`initialize`, `tools/list`, `tools/call`, `ping`), and emits typed `JsonRpcResponse` strings (parse error -32700, method not found -32601, execution error -32603); notifications (null id) receive no response. `run_mcp_server` runs the stdio loop.

<!-- @design:FR-MCP-02 -->
## FR-MCP-02 MCP tools
`spec-engine/src/mcp/tools.rs::get_tools_list` publishes six tools with JSON Schema inputSchemas: `asep_check`, `spec_check` (legacy alias), `adr_new`, `render_diagrams`, `srcidx`, `srcdump`. `execute_tool` dispatches by name, validates required arguments (e.g. `title` for `adr_new`), maps string parameters to strongly typed config (`OutputMode`, `OutputFormat`), and returns MCP `content`/`isError` envelopes or an `Unknown tool` error.

<!-- @design:FR-PACK-01 -->
## FR-PACK-01 Agent presets and embedded skills
`spec-engine/src/pack/manifest.rs` generates `plugin.json` and `mcp_config.json` manifests, merges the `asep` MCP server entry into `opencode.json` without clobbering existing keys, and `scaffold_opencode_agents` installs Lead/Executor/Judge/Explorer presets whose permission rules derive from `CapabilityProfile` (deny-all ceiling plus capability allows; read-only agents get extra deny rules). `spec-engine/src/pack/embedded.rs` holds canonical `SKILL.md` text for each embedded skill, returned by `get_embedded_skills`.

<!-- @design:FR-PACK-02 -->
## FR-PACK-02 Agent detection and plugin install
`spec-engine/src/pack/targets.rs` defines `TargetAgent` (Antigravity, OpenCode, ClaudeCode, Cursor, Windsurf, Continue, RooCode, Copilot, Goose, Zed, Aider) with `is_detected` probing each agent's config/skills directory and `resolve_target_paths` for local or global scopes. `spec-engine/src/pack/installer.rs::run_pack_installer` resolves scope and target agents interactively or headlessly, confirms targets, filters embedded skills, then writes per-agent artifacts: `plugin.json` + `mcp_config.json` + `skills/*/SKILL.md` for Antigravity; skills + `opencode.json` merge + agent presets for OpenCode; `SKILL.md` for ClaudeCode/Goose; `.mdc`/`.md` rules for Cursor/Windsurf/Continue/RooCode/Zed/Aider; and `skills/<id>.md` for Copilot.

<!-- @design:FR-SKILL-01 -->
## FR-SKILL-01 Skill inspection and drift detection
`spec-engine/src/skill/inspector.rs::inspect_skills` reads every `SKILL.md` in the skills dir, parses YAML frontmatter (name, description, version, depends), matches each copy against `get_embedded_skills()` by id or directory, and classifies it as Builtin (content identical), Modified (content differs), or Custom (no embedded original), plus Missing entries for embedded skills absent locally. `detect_skill_drift` returns `SkillDrift` records for modified copies only; missing skills are not reported as drift.

<!-- @design:FR-SKILL-02 -->
## FR-SKILL-02 Skill CRUD and import/export
`spec-engine/src/skill/manager.rs` implements pure file operations on `SKILL.md` documents: `add_skill`/`add_skill_full` create a skill directory with frontmatter and body, normalizing the id (lowercase, spaces to dashes); `delete_skill` removes the skill directory; `export_skill` copies `SKILL.md` to a destination; `import_skill` reads an external markdown file, requires a valid `name:` frontmatter field, derives the id from it, and writes the file into the skills directory. State is preserved on disk as individual skill files.

<!-- @design:FR-INIT-01 -->
<!-- @design:FR-VERIFY-01 -->
## FR-VERIFY-01 Verification engine
`spec-engine/src/verification/mod.rs::evaluate` computes `structural` from `graph.is_closed` (no error-severity coverage violation), `behavioral` from `.spec-engine/behavioral.json::passed` (missing file = false + skipped), and `semantic` from `evidence/semantic/<id>.md` for every `Req` node (valid schema, `verdict == ACCEPTED`, skipped when the tier does not require semantic evidence per `policy::load_policy`). Status is `Fail` when structural or behavioral is false, `Unproven` when semantic evidence is required but missing, otherwise `Pass`.

## FR-INIT-01 Baseline project init
`spec-engine/src/init/mod.rs::init_asep_project` (already declared in source) creates the canonical documentation tree (`docs/adr`, `docs/diagrams/rendered`, `docs/invariants`) and writes `docs/PRD.md` containing `<!-- @req:FR-INIT-01:MUST Baseline project specification -->` only when no PRD exists yet. Invoked by the `asep init` CLI command (runner.rs Commands::Init).

<!-- @design:FR-ANTISLOP-05 -->
## FR-ANTISLOP-05 No `else` — guard clauses only
`spec-engine/policy/asep-policy.toml` gains `[[antislop_rules]] id='AS-CODE-ELSE', domain='code', severity='Error', pattern='\belse\b', message='Else branch forbidden; use early return (guard clause)'`. `spec-engine/src/auditor/anti_slop.rs` adds `AntiSlopViolationKind::ElseBranch { token: String }`, `scan_else(violations, path, cleaned_lines)` over comment-stripped code lines, and a `message_for` arm. A dedicated `scan_else` (rather than a generic regex over raw lines) is required because code-domain canonical rules currently run only on comments; the scanner must run on the stripped code body. The existing gate logic in `runner.rs:177-208` and `ci/mod.rs:179-195` already fails on `Severity::Error`, so no gate change is needed.

<!-- @design:FR-ANTISLOP-06 -->
## FR-ANTISLOP-06 No nested `if`
`AS-CODE-NESTING` becomes `severity='Error'`, `max_nesting=0`. The value is redefined as the **number of enclosing `if` levels allowed**: `scan_nesting` must flag an `if` when `enclosing_ifs > max_nesting`, so 0 forbids every `if` inside another `if` while a top-level `if` stays legal (guard clauses require top-level `if cond { return }`). The reported `depth` stays `enclosing_ifs + 1` for continuity with existing tests. Change the fallback `unwrap_or(3)` to `unwrap_or(0)` and update the doc comment. Keep the brace-stack algorithm and `ExcessiveNesting` kind; update the message to `Nested if forbidden; use early return (guard clause)`.

<!-- @design:FR-ANTISLOP-09 -->
## FR-ANTISLOP-09 Non-suppressible strict rules
`scan_nesting` currently skips line numbers in `allowed_lines` (populated by `asep:allow-antislop`) and `scan_else` must initially be wired the same way. Change both scanners to ignore `allowed_lines` for these two rules only; all other anti-slop rules keep the existing opt-out. This makes requirements 05 and 06 absolute as requested.

<!-- @design:FR-ANTISLOP-07 -->
## FR-ANTISLOP-07 Policy-driven enforcement
`spec-engine/src/auditor/anti_slop.rs::code_rule` reads `AS-CODE-NESTING` and `AS-CODE-ELSE` from `spec-engine/policy/asep-policy.toml` through the cached compiled-rule table. Severity and message come from the policy (Error fallback for `AS-CODE-ELSE`), so changing severity or thresholds in the policy changes `asep check`/`asep ci` behavior without recompiling gate logic in `runner.rs` or `ci/mod.rs`.

<!-- @design:FR-ANTISLOP-08 -->
## FR-ANTISLOP-08 Self-compliance
Every audited source file in the repository is refactored to sequential guard clauses (top-level `if cond { return/continue/break }`), removing all `else` tokens and all nested `if`. No `asep:allow-antislop` marker is used for these two rules. `.asepignore` keeps `spec-engine/tests/`, `srcidx/tests/`, `**/benches/**`, `vendor/`, and `harness/e2e/vendor-loom/` out of audit scope.

## Implementation plan (ordered; one file or one function per unit)

### Task 1 — Policy declaration
- File: `spec-engine/policy/asep-policy.toml`
- Symbol: `AS-CODE-NESTING` (severity Error, max_nesting 0), new `AS-CODE-ELSE` (Error)
- Acceptance: `load_policy()` parses the file; `code_rule("AS-CODE-ELSE")` returns an Error-severity rule; nesting rule has `max_nesting = 0`.
- Verification: `cargo test -p asep --lib --offline policy::tests` then the command emitted by `cargo run -q -p asep --bin asep -- affected spec-engine/policy/asep-policy.toml`.

### Task 2 — Auditor scanners
- File: `spec-engine/src/auditor/anti_slop.rs`
- Symbols: `AntiSlopViolationKind::ElseBranch`, `scan_else`, `message_for`, `scan_nesting` (ignore `allowed_lines`; flag when `enclosing_ifs > max_nesting`; default `unwrap_or(0)`; update doc comment)
- Acceptance: unit tests prove `else` and `else if` are flagged; an `if` inside an `if` is flagged at depth 2 while a single top-level `if` and sequential top-level `if`s are NOT flagged; comment text and string literals containing `else` are not flagged; `asep:allow-antislop` does not suppress these two rules but still suppresses AI-buzzword and narration rules.
- Verification: `cargo test -p asep --lib --offline auditor::anti_slop::tests` plus `asep affected spec-engine/src/auditor/anti_slop.rs`.

### Task 3 — Gate behavior test
- File: `spec-engine/tests/integration_check_test.rs` (reuses the existing `write_t2_closed_project` helper and T1 anti-slop test pattern at line ~365)
- Symbol: new test `check_fails_on_guard_clause_violations`
- Acceptance: a temp project containing a `.rs` file with one `else` and one nested `if` makes `asep check --rigor T1` exit non-zero; the same project with guard clauses exits 0.
- Verification: `cargo test -p asep --test integration_check_test --offline`.

### Task 4..N — Self-compliance refactor (one unit per file)
- Files (audited, 35 files / 212 `else` tokens): top offenders `spec-engine/src/runner.rs` (48), `skill/inspector.rs` (13), `skill/menu.rs` (12), `agent_protocol/mod.rs` (11), `pack/manifest.rs` (10), `auditor/anti_slop.rs` (10), `pack/installer.rs` (9), `ci/mod.rs` (9), then the remaining 27 files. Nested `if` sites are found per file by the new scanner.
- Symbol: each file's control-flow sites, converted to sequential guard clauses (`if cond { return/continue/break }`).
- Acceptance: the file has zero `else` tokens and zero nested `if`; public behavior and existing tests unchanged; no `asep:allow-antislop` added for these rules.
- Verification: `cargo run -q -p asep --bin asep -- affected <changed-file>` and run its derived commands.

### Final Task — Whole-branch compliance gate
- Acceptance: `asep check . --rigor T1` exits 0 with the new strict policy in force; zero violations of `AS-CODE-ELSE` and `AS-CODE-NESTING` in audited files.
- Verification: `cargo run -q -p asep --bin asep -- affected <all-changed-paths>`, `cargo clippy -p asep --offline -- -D warnings`, `cargo run -q -p asep --bin asep -- check . --rigor T1`.

## Known consequences of this plan
- The self-compliance phase is the bulk of the work: 35 files, 212 `else` tokens plus nested `if` sites. It is split into one executor unit per file; each unit is independently verified.
- Making the rules Error-severity means intermediate commits will fail `asep check . --rigor T1` until the cleanup phase finishes. The final gate is the acceptance point.
- `.asepignore` already excludes `spec-engine/tests/`, `srcidx/tests/`, `vendor/`, and `harness/e2e/vendor-loom/`, so third-party and fixture code is out of scope.

## Pilot optimasi build Rust

<!-- @design:BUILD-PILOT-01 -->
Bandingkan profil saat ini (A) dengan `debug = 0` hanya untuk dependensi non-workspace di profil `dev` dan `test` (B). Uji B dengan dua flag Cargo: `--config 'profile.dev.package."*".debug=0' --config 'profile.test.package."*".debug=0'`. Kode workspace tetap `line-tables-only`; profil release, incremental, fitur, linker, dan pin `nightly-2026-11-14` tidak berubah. Tidak perlu runner baru.

Gunakan worktree sementara dari SHA yang sama, di filesystem yang sama dengan repo; `/tmp` lokal adalah tmpfs. Pakai satu direktori artefak khusus eksperimen, pada path yang sama untuk kedua varian. Kosongkan hanya direktori eksperimen saat ganti varian; jangan sentuh `target/` utama. Catat SHA, toolchain, linker, jumlah job, dan urutan uji. Jalankan serial. Hentikan pilot jika ruang kosong terukur mendekati 4 GiB. Pada audit awal, `target/` utama 3,3 GiB dan ruang kosong 8,7 GiB.

Ukur wall time build awal dan build tanpa perubahan sebagai konteks. Setelah pemanasan, ulangi tiap skenario edit minimal tiga kali per varian. Terapkan perubahan identik dan pulihkan sumber di luar pengukuran; bangun ulang keadaan awal sebelum pengulangan berikutnya. Catat sampel waktu, median, disk puncak teramati dari sampel `du -s -B1` selama rangkaian, dan disk tersisa tepat setelah rangkaian yang sama. Sampling dapat melewatkan lonjakan singkat. Jangan laporkan p95 dari tiga sampel. Simpan hasil di luar direktori artefak eksperimen.

<!-- @design:BUILD-PILOT-02 -->
Adopsi kandidat hanya jika median kedua skenario edit turun, disk puncak teramati dan disk tersisa sama-sama turun, serta arah perbaikan waktu tidak bergantung pada satu pengukuran. Ulangi perbandingan bila hasil dekat dengan variasi sampel; jika tetap tidak jelas, pertahankan profil saat ini. Bandingkan daftar dan hasil test target yang sama. Full suite tidak dijalankan untuk pilot; gate penerimaan yang berlaku tidak berubah.

### Urutan kerja pilot

| Urutan | File dan simbol | Kriteria penerimaan | Perintah verifikasi |
|---|---|---|---|
| 1. Ukur edit fungsi | `spec-engine/src/harness/e2e.rs`, `now_rfc3339` | Terapkan edit body sementara (`duration.as_secs()` menjadi `duration.as_secs().saturating_add(1)`) di worktree eksperimen. Kedua varian membangun target yang sama; catat waktu pada setiap pengulangan dan pulihkan file. | `cargo build -p asep --bin asep --offline --locked --timings` |
| 2. Ukur edit test dan disk | `spec-engine/tests/harness_e2e_test.rs`, `test_new_run_writes_manifest` | Ganti sementara assertion keberadaan seed dengan pemeriksaan seed `883735528`. Kedua varian memakai daftar test dan hasil yang sama; catat waktu, puncak teramati, dan ukuran akhir untuk rangkaian langkah 1–2. Pulihkan file. | `cargo test -p asep --test harness_e2e_test --offline --locked -- --list`; `cargo test -p asep --test harness_e2e_test --offline --locked` |
| 3. Adopsi bila lolos | `Cargo.toml`, `profile.dev.package."*".debug` dan `profile.test.package."*".debug` | Hanya tambah kedua override. Jangan ubah konfigurasi global, test, atau gate. Jika kandidat tidak lolos, jangan ubah manifest. | `asep affected Cargo.toml`, lalu jalankan perintah dari hasilnya. Jika kosong, catat bahwa graph tidak memetakan profil Cargo; cek `cargo metadata --offline --locked --no-deps --format-version 1` dan ulangi test target pilot tanpa flag override di direktori eksperimen. |

<!-- @impl:BUILD-PILOT-01 -->
Pilot dijalankan di `446a0656` dengan 4 job, dalam urutan A lalu B dan B lalu A. Tiap run mengukur tiga edit fungsi dan tiga edit test. Wall time edit test mencakup kompilasi lewat `--list` dan eksekusi test. Interval sampel disk 1,5 detik; satu sampel baseline kedua gagal saat file sementara hilang.

<!-- @impl:BUILD-PILOT-02 -->
Kedua urutan memenuhi syarat adopsi. Konfigurasi kandidat menjadi dua override di `Cargo.toml`. Angka acuan belum tercapai. Gate penerimaan tidak berubah.

## Lanjutan optimasi build Rust

<!-- @design:BUILD-NEXT-01 -->
Audit awal: `target/debug/build` memakai 1.205.305.344 byte. Hitungan blok file reguler deduplikasi inode: 919,0 MiB output unit kompilasi, 203,9 MiB executable/metadata build script, 20,0 MiB output build script, dan 6,6 MiB file lain. File `.a`/`.o` grammar bukan penyumbang utama. Empat build tanpa edit di pilot tidak melaporkan kompilasi ulang. Ini potret cache campuran, bukan ukuran profil baru maupun bukti semua perintah bebas invalidasi.

<!-- @design:BUILD-NEXT-02 -->
Mulai dari satu duplikasi versi: TOML 0.8.23 dipakai `srcdump`, sedangkan ASEP sudah menarik TOML 1.2.1 lewat batas versi `1.1.8`. Bandingkan unit kompilasi, bukan hanya baris `cargo tree`. Jangan menyatukan `syn` lintas versi atau mengganti backend TLS pada putaran ini.

<!-- @design:BUILD-NEXT-03 -->
Semua uji memakai pin nightly dan profil debug hasil pilot, offline, satu cache eksperimen di filesystem repo, ruang kosong minimal 4 GiB. Jangan bersihkan cache utama. Untuk A/B struktural, bangun dari direktori eksperimen kosong pada path sama; ukur build awal, tiga rebuild edit identik, disk puncak teramati dan akhir. Untuk jobs/antrean, samakan keadaan cache sebelum tiap percobaan dan balik urutan; catat wall time per proses dan sampai keduanya selesai, pesan lock Cargo, serta disk puncak dan akhir. Jangan klaim hasil jobs berlaku di mesin lain. Full suite dan gate penerimaan tidak berubah.

### Urutan kerja

| Urutan | File dan simbol | Kriteria penerimaan | Verifikasi |
|---|---|---|---|
| 1. Audit cache | `target/debug/build`, direktori unit dan output build script | Klasifikasi blok tanpa hitung hard link ganda, daftar komponen terbesar, bedakan artefak lama dari output aktif jika terbukti. Tidak ada penghapusan. | `du -s -B1 target/debug/build`; inventaris inode; log warm no-op pilot |
| 2a. Gabung satu kelompok | `spec-engine/tests/lexer_test.rs`, modul `scanner` | Salin kasus `test_scan_project_tags_respects_gitignore` dari target `scanner_test` tanpa ubah fixture atau assertions. Pertahankan lima kasus lexer. | `cargo test -p asep --test lexer_test --offline --locked` |
| 2b. Hapus target duplikat | `spec-engine/tests/scanner_test.rs`, target `scanner_test` | Hapus file hanya setelah enam kasus terbukti terdaftar dan lulus. Bandingkan dua target lama vs satu target baru; adopsi jika biaya turun dan feedback edit target tidak melambat di luar variasi sampel. | `cargo test -p asep --test lexer_test --offline --locked -- --list`; `cargo metadata --offline --locked --no-deps --format-version 1`; benchmark A/B |
| 3a. Kunci perilaku konfigurasi | `srcdump/tests/config_test.rs`, `Config::from_file` | Uji fixture konfigurasi valid dan input TOML tidak valid sebelum ganti versi parser. | `cargo test -p srcdump --test config_test --offline --locked` |
| 3b. Satukan versi | `Cargo.toml`, `workspace.dependencies.toml` | Samakan batas versi dengan ASEP; jangan ubah fitur lain. | `cargo metadata --offline --format-version 1` pada worktree eksperimen |
| 3c. Perbarui resolusi | `Cargo.lock`, keluarga TOML 0.8 | Resolusi offline; tidak ada upgrade yang tidak terkait. Adopsi jika unit lama hilang, test konfigurasi tetap lulus, dan pilot tidak menunjukkan regresi feedback atau disk. | `cargo tree -p asep -d --offline --locked`; `cargo test -p asep --test lexer_test --unit-graph -Z unstable-options --offline --locked`; `cargo test -p srcdump --test config_test --offline --locked` |
| 4. Uji jobs dan antrean | `spec-engine/tests/lexer_test.rs`, target `lexer_test`; `Cargo.toml`, konfigurasi `build.jobs` tidak diubah | Bandingkan 2 vs 4 job untuk build awal dan rebuild setelah edit yang sama. Pada cache eksperimen, bandingkan pasangan build/test identik secara serial dan serentak. Pisahkan waktu tunggu lock dari kerja kompilasi sejauh log mendukung. Laporkan, jangan ubah default. | `cargo build -p asep --bin asep --offline --locked --jobs <2-or-4> --timings`; `cargo test -p asep --test lexer_test --offline --locked --jobs <2-or-4>` |

Setiap perubahan: `asep affected <changed-paths>` lebih dulu. Jika hasil kosong atau menyebut target Cargo yang tidak ada, catat dan gunakan verifikasi target pada tabel. Tinjau diff dan bukti per unit sebelum lanjut.

<!-- @impl:BUILD-NEXT-01 -->
Audit selesai. Konsolidasi `scanner_test` ke `lexer_test` tidak diadopsi. Putaran valid menghemat 19,9 MiB artefak tersisa (2,7%), tetapi semua sampel edit kandidat lebih lambat daripada semua sampel baseline. Kedua target tetap terpisah di worktree utama.

<!-- @impl:BUILD-NEXT-02 -->
Penyatuan TOML menghilangkan enam unit kompilasi lama tanpa menambah unit baru. Tidak diadopsi: dalam satu putaran, build awal lebih cepat dan disk lebih kecil, tetapi semua sampel edit kandidat lebih lambat daripada semua sampel baseline. Versi parser dan lockfile utama tetap; tiga tes konfigurasi `srcdump` dipertahankan.

<!-- @impl:BUILD-NEXT-03 -->
Pada mesin ini, 4 job menang pada satu putaran build dari cache kosong. Tiga sampel edit per jumlah job tumpang tindih. Jalankan build dan test bergantian bila berbagi target: uji serentak memunculkan tunggu lock tanpa keuntungan waktu total yang konsisten. Default Cargo tetap.
## Capability-Aware Delegation & Ambient Skill Distribution

### Phase M — MCP capability routing (first)

<!-- @design:FR-CAP-01 -->
Capability registry: `CapabilityProfile` in `spec-engine/src/config/settings.rs` gains `supported_media: Vec<String>` and `tool_domains: Vec<String>` (config-driven, validated by `asep check`). No hardcoded enum; unknown entries are accepted but warned. Default profile includes `text`, `code`.

<!-- @design:FR-CAP-02 -->
MCP delegation schema: extend `asep_agent_send` payload (in `spec-engine/src/mcp/tools.rs` and `spec-engine/src/agent/protocol.rs` if present) with optional fields `required_capabilities`, `media`, `domain`, `tool_filter`, `delegation_threshold`. Serde `#[serde(default)]` keeps backward compat. Validation in `execute_tool`.

<!-- @design:FR-CAP-03 -->
Capability guard: executor (and any worker) checks task's `required_capabilities`/`media` against its profile before running; mismatch returns structured `BLOCKED: requires <capability>` without executing. Implemented in `spec-engine/src/agent/executor_guard.rs` (new) or existing executor handler.

<!-- @design:FR-CAP-04 -->
Single-agent default + threshold: leader prompt/rules and `CapabilityProfile.delegation_threshold` (default e.g. 3 tool calls) define when to delegate. Implemented as config + documented rule; no code change beyond config field, enforced by agent behavior + check.

<!-- @design:FR-CAP-05 -->
Tool filtering + trajectory: worker loads only tools in `tool_filter` (or domain default). TASK_REPORT includes `tool_call_count: u32` and optional `duplicate_calls`. Added to report schema in `spec-engine/src/agent/protocol.rs`.

<!-- @design:FR-CAP-06 -->
Trajectory review: judge flags duplicate/unnecessary tool calls as blocking when evidence shows waste. Documented in judge prompt (opencode.json) + `asep check` optional gate.

<!-- @design:FR-CAP-07 -->
Capability-domain workers: `scaffold_opencode_agents` in `spec-engine/src/pack/manifest.rs` generates `code-worker`, `git-worker`, `media-worker`, `verifier` presets with supported_media/tool_domains from CapabilityProfile.

<!-- @design:FR-CAP-08 -->
Media routing: router/leader selects worker whose supported_media covers task media; text-only agents never receive image/pdf/audio/video tasks. Enforced by capability guard (FR-CAP-03) + documented routing rule.

### Phase A — Ambient skill distribution (after Phase M)

<!-- @design:FR-AMB-01 -->
Ambient init: `init_asep_project` in `spec-engine/src/init/mod.rs` (or new `spec-engine/src/init/ambient.rs`) injects pointer block into AGENTS.md/CLAUDE.md/GEMINI.md and scaffolds flat `skills/` with canonical SKILL.md. Idempotent.

<!-- @design:FR-AMB-02 -->
Pack provisioning-only: remove skill-copy logic from `spec-engine/src/pack/installer.rs` (hard remove); keep role/permission/MCP wiring. Update `run_pack_installer`.

<!-- @design:FR-AMB-03 -->
> Historical note (superseded by the approved blueprint): Bootstrap injection: add `using-asep` skill + harness plugin entry (Pi extension style) generated by pack for supported harnesses. Embedded in `spec-engine/src/pack/embedded.rs`.

<!-- @design:FR-AMB-04 -->
Description-first + token budget: update all `*/skill/SKILL.md` frontmatter; `asep check` validates description starts with `Use when...` and word count budget.

<!-- @design:FR-AMB-05 -->
Skill capability metadata: SKILL.md frontmatter gains `requires: [<capability>]`; router skips unavailable. Parsed in `spec-engine/src/skill/inspector.rs` or new parser.

<!-- @design:FR-AMB-06 -->
Ambient governance gate: `asep check` validates pointer block presence, flat skills dir, description-first, token budget. New checks in `spec-engine/src/runner.rs` / auditor.

### Ordered implementation plan (Phase M first, then Phase A)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| M1 | `spec-engine/src/config/settings.rs`, `CapabilityProfile` (add `supported_media`, `tool_domains`, `delegation_threshold`) | Fields serialize/deserialize; defaults include text/code; invalid types rejected. | `cargo test -p asep spec-engine` settings tests; `asep check . --rigor T1` |
| M2 | `spec-engine/src/mcp/tools.rs`, `asep_agent_send` schema + handler | New fields optional with defaults; validation rejects malformed; backward compat with old payloads. | `cargo test -p asep mcp` tests; `asep affected spec-engine/src/mcp/tools.rs` |
| M3 | `spec-engine/src/agent/executor_guard.rs` (new) or executor handler, capability guard | Mismatch returns `BLOCKED: requires <cap>`; no execution; match proceeds. | Unit tests for guard; `asep affected` |
| M4 | `spec-engine/src/pack/manifest.rs`, `scaffold_opencode_agents` | Generates code/git/media/verifier presets from CapabilityProfile. | `asep pack --dry-run` or targeted tests; `asep affected` |
| M5 | `spec-engine/src/agent/protocol.rs`, TASK_REPORT (`tool_call_count`, `duplicate_calls`) | Fields serialized; judge can read. | Tests; `asep affected` |
| M6 | Judge prompt in `opencode.json` (project + global) + docs | Judge flags duplicate/unnecessary tool calls as blocking. | Manual review; `asep check . --rigor T1` |
| M7 | `spec-engine/src/init/mod.rs` + `spec-engine/src/init/ambient.rs` (new) | Pointer block injected idempotently; flat `skills/` scaffolded. | Integration test; `asep affected` |
| M8 | `spec-engine/src/pack/installer.rs`, `run_pack_installer` | Skill copy removed; provisioning only; no fallback. | Tests; `asep check . --rigor T1` |
| M9 | `spec-engine/src/pack/embedded.rs` + harness plugin entries | `using-asep` bootstrap skill embedded and installable. | `asep pack` dry-run; tests |
| M10 | `*/skill/SKILL.md` frontmatter + `spec-engine/src/skill/inspector.rs` | Description-first + token budget + `requires` parsed. | `asep check . --rigor T1`; `asep affected` |
| M11 | `spec-engine/src/runner.rs` + auditor, ambient governance gate | Pointer block, flat skills, description-first, budget validated. | `asep check . --rigor T1` |
| M12 | Verification & reconfig | Run `asep pack` to apply latest config after Phase M and after Phase A; restart MCP. | `asep --version`; MCP `initialize`; `asep check . --rigor T1` |

After Phase M complete: restart with latest config. After Phase A complete: restart again. Full suite reserved for merge to main.

### Scoped non-root acceptance

<!-- @design:FR-ACC-02 -->
Policy lives at `<root>/.spec-engine/acceptance.json` (user-owned, 0600); evidence dir user-owned 0700. Candidate snapshot is executed with unprivileged `bwrap --unshare-user --unshare-pid`, read-only repo/toolchain mounts, a writable scratch target dir, and no policy/evidence mounts. No `/etc` changes and no `sudo`.

<!-- @design:FR-ACC-03 -->
After gates pass, the runner enumerates all `@req:*:MUST` from the trace graph and writes `evidence/semantic/<id>.md` containing `reviewer`, `verdict: ACCEPTED`, and a rationale citing the commit, snapshot digest, and scoped gate results. `asep check` gains `--evidence-dir`; when supplied it replaces the default `<root>/evidence` lookup.

<!-- @design:FR-ACC-04 -->
Mandatory gates: Formal (`asep ci --gate 0`), Structural (`asep check --rigor T1`), ScopedTests (`asep affected <changed-paths>` then execute only listed test targets), Format (`cargo fmt --all --check`). Clippy and BenchPreflight are opt-in flags. Evidence records each gate command, exit code, and the affected-path list.

#### Ordered units for scoped acceptance

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| A1 | `spec-engine/src/runner.rs`, `Commands::Check` (`--evidence-dir`) | Flag parsed and passed to verification; default unchanged. | `cargo test -p asep --test integration_check_test` |
| A2 | `spec-engine/src/acceptance/scoped.rs` (new), `spec-engine/src/lib.rs` | Non-root policy loader, scoped gate runner using `asep affected`, bwrap isolation, semantic evidence writer. | `cargo test -p asep --lib acceptance::scoped` |
| A3 | `spec-engine/src/runner.rs`, `Commands::AcceptScoped` | CLI `asep accept-scoped <commit>` dispatches to A2; writes evidence; no full suite. | `cargo test -p asep --test acceptance_scoped_test` |
| A4 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | Requirements closed with `@rel:satisfies`/`@rel:verifies`, changelog notes scoped acceptance. | `asep check . --rigor T1` |

### OpenCode integration

<!-- @design:FR-MCP-03 -->
`asep_accept_scoped` tool in `spec-engine/src/mcp/tools.rs`: input `{commit: string, dir?: string}`; calls `acceptance_scoped::run_scoped_acceptance(dir or default_dir, commit)`, returns the evidence dir as MCP text. Missing/invalid commit or runner failure yields `isError: true`. The tool uses the same trusted runner; it does not grant `ACCEPTED` itself.

<!-- @design:FR-INT-01 -->
> Historical note (superseded by the approved blueprint): Add a verification rule to the leader system prompt in `spec-engine/src/pack/manifest.rs`, project `opencode.json`, and global `~/.config/opencode/opencode.json`: after implementation, run `asep check --rigor T2`, then call `asep_accept_scoped` (or CLI `asep accept-scoped <commit>`); report the runner's evidence dir. Keep the existing rule that only the trusted runner grants `ACCEPTED`. Mirror the rule in `workflow-router/skill/SKILL.md`.

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| I1 | `spec-engine/src/mcp/tools.rs`, `spec-engine/tests/mcp_test.rs` | `asep_accept_scoped` listed with typed schema; invalid commit returns error; success path uses ASEP_BWRAP shim in tests. | `cargo test -p asep --test mcp_test` |
| I2 | `spec-engine/src/pack/manifest.rs`, `opencode.json`, global `opencode.json`, `workflow-router/skill/SKILL.md` | Leader prompt instructs T2 check + accept-scoped; other prompts and settings unchanged. | JSON parse; `cargo test -p asep --test targets_test` |
| I3 | `docs/TRACEABILITY.md`, `CHANGELOG.md`, tags | FR-MCP-03/FR-INT-01 closed; changelog notes integration. | `asep check . --rigor T2` |

<!-- @design:FR-LEADER-01 -->
Use leader as the primary agent name in generated presets and global configuration.

## IN-01 — Agent Observability Protocol (Layer 05)

Review verdict REVISE. Round 1 folds the shared event/trace envelope (catalog step A: stable ID, schema/version, evidence format, authorization boundary, event envelope) into IN-01 because IN-01 consumes it; skipping A would lock a schema that later has to be replaced. Existing primitives are extended, never rebuilt: `Envelope.reply_to`/`ref` for correlation, `GateResult.duration_ms` for latency, `write_json_atomic` for metadata, evidence dir permissions for access control.

Constraint: new audited source must obey the existing strict policy — sequential guard clauses, no `else`, no nested `if`, no inline opt-out. Requirements are observable; no new engine is introduced because adapters already suffice.

<!-- @design:IN-01-CONTRACT -->
`spec-engine/src/observability.rs` defines `TraceEvent { schema_version, trace_id, parent_id: Option<String>, task_id, seq: u64, ts_wall: String, duration_ms: Option<u128>, ... }`, `TraceSpan` and `SpanGuard`. IDs are caller-supplied; `seq` is assigned by the collector. `duration_ms` is measured with `Instant::elapsed()` and never derived from `ts_wall`. Fact: `now_rfc3339()` in `agent_protocol/mod.rs:586` has second precision and uses wall clock, so it is unsuitable for ordering sub-second events — it is human-readable metadata only.

<!-- @design:IN-01-COMPAT -->
Additive only. No edits to `Envelope` (`agent_protocol/mod.rs:120`), `GateResult` (`ci/mod.rs:24`), `CheckJsonReport` (`runner.rs:28`) or the 11 existing MCP tools. Config gains a `telemetry` table with `#[serde(default)]` in `config/settings.rs`, resolved through `config::load_config`.

<!-- @design:IN-01-COLLECT -->
`TraceCollector` appends NDJSON (one JSON object per line) to `<root>/.spec-engine/trace/<trace_id>.ndjson` (dir configurable). This matches the existing ASEP convention: `srcdump::OutputFormat::Ndjson` is the default (`srcdump/src/config.rs:10,131`, `srcdump/src/main.rs:56`) and MCP `srcidx`/`srcdump` advertise `format: "ndjson"` (`mcp/tools.rs:109,157,172`). Buffered writes flush on span end and on `Drop`; with sampling `off` no file is created. Reuse the atomic-write convention for metadata only, not for the append stream.

<!-- @design:IN-01-COST -->
`CostMetadata { value: Option<f64>, currency: Option<String>, source: CostSource, missing: Vec<String> }`; `CostSource` serializes lowercase (`measured`, `estimated`, `provider_reported`, `unavailable`). `None` is never coerced to `0`.

<!-- @design:IN-01-REPORT -->
`Commands::Trace { action }` in `cli.rs` with `status|report|purge`; `asep_trace_report` added to `get_tools_list` and `execute_tool` in `mcp/tools.rs` (tool count 11 -> 12; keep `test_tool_descriptions_are_compact` at <= 60 chars). Report joins NDJSON events with `CiReport`/`CheckJsonReport` by task/gate id; missing values stay visible.

<!-- @design:IN-01-PRIVACY -->
Config fields `allowlist: Vec<String>` and `redact_patterns: Vec<String>`; persist only allowlisted fields and redact before write. `asep trace purge <trace_id>` rewrites/removes the segment and derived report. Files use the 0600 permission precedent from FR-ACC-02. Fixture asserts a secret string never appears in trace output.

<!-- @design:IN-01-RETENTION -->
Config fields `ttl_secs` and `max_bytes`; rotation starts a new segment and expired segments are deleted. Document that purge/rotation means absolute append-only is not claimed (consistent with KN-03-D04).

<!-- @design:IN-01-OVERHEAD -->
`TelemetryConfig.enabled: bool` in config, default `false`. No sampled mode: ordinary event recording is either on or off. Benchmark on the same pinned commit, telemetry on/off, balanced order, at least three samples, median only; no p95 from three samples and no claim beyond this machine (per BUILD-NEXT-03).

Measured on this machine with a debug build, 10,000 synthetic events, three balanced samples (off/full, full/off, off/full): off = [5, 5, 6] ms (median 5 ms, 0 bytes written); full = [451, 465, 483] ms (median 465 ms, 1,787,780 bytes ≈ 1.7 MiB). Full telemetry is therefore ~93x slower than off for this synthetic workload; the default remains `Off` and no non-off default is adopted. This is a local debug-build measurement, not a universal claim.

<!-- @design:IN-01-METRIC -->
`TraceReport` carries each metric with its denominator, `skipped[]`, failures and a `baseline_commit`; no single aggregate score. Reuse `VerificationReport.skipped` and `CheckJsonReport` totals rather than re-deriving them.

### Ordered implementation plan (one file or one function per unit)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| O1 | `spec-engine/src/observability.rs`, `TraceEvent`/`TraceSpan`/`SpanGuard` | Envelope serializes with schema_version, explicit IDs, seq, ts_wall and Instant-based duration; caller IDs are required; no field inferred. | `cargo test -p asep --lib observability` then `asep affected spec-engine/src/observability.rs` and run its listed commands. |
| O2 | `spec-engine/src/config/settings.rs`, `TelemetryConfig` | `telemetry` table loads with defaults; sampling defaults to off; allowlist, redact_patterns, ttl_secs, max_bytes parse and reject wrong types. | `cargo test -p asep --test config_test` then `asep affected spec-engine/src/config/settings.rs`. |
| O3 | `spec-engine/src/observability.rs`, `TraceCollector` | NDJSON append with bounded buffer and flush on span end/Drop; no file when off; purge removes a trace. | `cargo test -p asep --lib observability` then `asep affected spec-engine/src/observability.rs`. |
| O4 | `spec-engine/src/cli.rs` + `spec-engine/src/runner.rs`, `Commands::Trace` | `asep trace status|report|purge` dispatch and exit codes; no change to other commands. | `cargo test -p asep --test integration_check_test` then `asep affected spec-engine/src/cli.rs spec-engine/src/runner.rs`. |
| O5 | `spec-engine/src/mcp/tools.rs`, `asep_trace_report` | Tool listed with typed schema; invalid trace_id returns `isError`; output reconciles with the CLI report; `test_tool_descriptions_are_compact` still passes at 12 tools. | `cargo test -p asep --test mcp_test -- --test-threads=1 --skip test_mcp_tools_call_srcdump --skip test_mcp_tools_call_srcidx` then `asep affected spec-engine/src/mcp/tools.rs`. |
| O6 | `spec-engine/src/observability.rs` + `spec-engine/tests/observability_test.rs`, reconciliation and privacy | Synthetic trace with measured/estimated/unavailable, nested call, retry, resume and secret fixture; no guessed values, no secret in log, report matches CiReport/CheckJsonReport. | `cargo test -p asep --test observability_test` then `asep affected spec-engine/tests/observability_test.rs`. |
| O7 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | Add `@rel:satisfies`/`@rel:verifies` for all nine IN-01 requirements; changelog notes observability MVP. | `asep check . --rigor T1`, then `asep check . --rigor T2` after scoped acceptance. |
| O8 | Overhead benchmark result (recorded in TDD, not a code unit) | On/off overhead measured on the same commit with balanced order and >=3 samples; default remains off. | Record command output and median; no default change. |

### File-change evidence (added after review discussion)

<!-- @design:IN-01-FILE-EVIDENCE -->
`FileChangeTracker` in `spec-engine/src/observability.rs` holds the root, the in-scope paths supplied by the caller (from `asep affected` / acceptance `changed_paths`), and a `path -> sha256` map. `baseline()` hashes once; `checkpoint(collector, task_id, span_label)` hashes again and emits one `file_change` `TraceEvent` per changed path only. Hashing reuses `agent_protocol::hash_file` (SHA-256 already a dependency) and `now_rfc3339` for the wall stamp; both become public. Structured before/after data lives in a new optional `detail: serde_json::Value` field on `TraceEvent` (`#[serde(default)]`, additive to existing NDJSON). Hashing happens twice per span, not once per edit, so its cost scales with spans rather than with events.

<!-- @design:IN-01-INTENT -->
`FileChangeTracker::intent(collector, path, task_id)` emits a `write_intent` event with the pre-write hash before a controlled tool writes. `TraceCollector::record_evidence()` buffers these events even when ordinary event recording is off, so file-change evidence is never thinned. `TraceReport` gains `event_coverage: String` (`on` or `off`) supplied by the caller from `TelemetryConfig.enabled`; file-change/intent events are always counted as complete. Reports thereby state whether ordinary event coverage was on or off.

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| F1 | `spec-engine/src/observability.rs`, `spec-engine/src/agent_protocol/mod.rs` | `TraceEvent.detail` added additively; `record_evidence` bypasses sampling; `FileChangeTracker` baseline/checkpoint emits only changed paths with before/after hashes and span label; `intent` emits pre-write hash. | `cargo test -p asep --lib observability --offline --locked` |
| F2 | `spec-engine/tests/observability_test.rs` | Synthetic file edits: unchanged path emits nothing, changed path emits exactly one event with correct hashes, intent precedes change, all file evidence survives Sampled mode, report coverage label correct. | `cargo test -p asep --test observability_test --offline --locked` |
| F3 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | Two new requirements closed with relations; changelog notes file-change evidence. | `asep check . --rigor T2` after scoped acceptance |

### Known consequences of this plan
- Adding the nine `@req` tags before `@impl`/`@test`/`@rel` exist makes `asep check --rigor T1` fail COV-04/COV-05. This is expected and identical to the earlier scoped-acceptance rollout; the final gate after O7 is the acceptance point.
- `asep affected` cannot map a new file until its `@impl`/`@test` tags land, so O1–O3 rely on the targeted `cargo test` commands; if `asep affected` returns empty, record that explicitly and use the listed targets.
- Full suite remains reserved for merges to `main`.

## Context Compiler — production hardening

Constraint: no new dependency; reuse `crate::affected::affected`, `srcdump::{Config, Formatter, walk_files}`, `srcidx` parser registry, `crate::runtime::budget::{ResourceBudget, BudgetTracker}`. Deterministic output.

<!-- @design:FR-CTX-01 -->
### FR-CTX-01 `compile_context`
New module `spec-engine/src/context/mod.rs` with `ContextBundle { objective, changed_paths, impacted: AffectedReport, excerpts: Vec<ContextExcerpt>, truncation: String, commands: Vec<String> }` and `compile_context(root: &Path, paths: &[PathBuf], budget: ResourceBudget) -> Result<ContextBundle, String>`. Steps: `affected()` → collect impacted file paths → `srcdump` signatures/summary via existing library → assemble bundle. Deterministic ordering (sort by path/line/id).

<!-- @design:FR-CTX-02 -->
### FR-CTX-02 Budget wiring
Before adding each excerpt, call `BudgetTracker::record_tokens(estimate)`; if `exceeded()` is Some, stop adding excerpts, set `truncation` to `blocked_message()`, and keep all impacted node IDs in `impacted`. No silent omission.

<!-- @design:FR-CTX-03 -->
### FR-CTX-03 CLI and MCP exposure
`spec-engine/src/cli.rs`: `Commands::Context { paths, root, max_tokens }`; `spec-engine/src/runner.rs`: dispatch to `compile_context`, print JSON with `--json` (global flag already exists) or text summary. `spec-engine/src/mcp/tools.rs`: `asep_context` tool with schema `{paths: array<string>, root?: string, max_tokens?: integer}` using same function. Additive: tool count 13 → 14, other commands/tools unchanged.

### Ordered implementation plan

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| C1 | `spec-engine/src/context/mod.rs`, `compile_context` | Bundle deterministic; impacted IDs complete; truncation explicit. | `cargo test -p asep --lib context` |
| C2 | `spec-engine/tests/context_test.rs` | Tests for budget exhaustion, determinism, no silent drop. | `cargo test -p asep --test context_test` |
| C3 | `spec-engine/src/cli.rs`, `runner.rs`, `Commands::Context` | CLI parses and prints bundle; `--max-tokens` wired. | `cargo test -p asep --test integration_check_test` |
| C4 | `spec-engine/src/mcp/tools.rs`, `asep_context` | Tool listed with schema; returns bundle; errors on empty paths. | `cargo test -p asep --test mcp_test` |
| C5 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | Three requirements closed; changelog notes. | `asep check . --rigor T2` after scoped acceptance |

## MCP.2 — Impact Integration (RFC/proposal)

Constraint: reuse `affected()` + `AffectedReport`; no new impact engine, no new dependency; MCP.3/MCP.4/P3.3 deferred.

<!-- @design:FR-MCP-IMPACT-01 -->
### FR-MCP-IMPACT-01 `asep_affected` tool
`spec-engine/src/mcp/tools.rs`: add tool entry with JSON Schema `{paths: array<string>, root?: string}` and dispatch arm calling `crate::affected::affected(root, paths)`, serializing `AffectedReport` into MCP text content. Invalid/empty paths return `isError: true`.

<!-- @design:FR-MCP-IMPACT-02 -->
### FR-MCP-IMPACT-02 Semantics and compatibility
No changes to `affected()` or `AffectedReport`. Tool count 12 → 13; keep `test_tool_descriptions_are_compact` (description <= 60 chars). Existing tools unchanged.

### Ordered implementation plan (MCP.2, pending approval)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| M1 | `spec-engine/src/mcp/tools.rs`, `get_tools_list` + `execute_tool` | Tool listed with typed schema; returns structured report; invalid paths error. | `cargo test -p asep --test mcp_test` |
| M2 | `spec-engine/tests/mcp_test.rs` | Test asserts tool presence, call success with a temp project, and error path. | `cargo test -p asep --test mcp_test` |
| M3 | `docs/TRACEABILITY.md` | Two requirements closed with relations; tool count updated where asserted. | `asep check --rigor T2` after scoped acceptance |

## P3.1 — Transitive Impact Analysis (RFC v0.2)

Constraint: reuse `build_relation_graph` without new engine/database/dependency. Additive to CLI/JSON. Preserve legacy same-ID fallback and coverage rules.

<!-- @design:FR-IMPACT-01 -->
### FR-IMPACT-01 Cycle-safe transitive traversal
`spec-engine/src/affected/mod.rs`: refactor `affected()` to perform BFS over the relation graph, following typed edges in both directions (outgoing via `edge.from == current`, incoming via `edge.to.contains(current)`), with a `HashSet<String>` visited set keyed by node ID. **Transition rules (direction-aware, typed):** `Impl`, `Req`, `Test` are traversable; `Adr` and `Design` are terminal (sinks) — no traversal originates from them, preventing false impact expansion through shared ADRs. Edge kind and direction are recorded per hop.

### FR-IMPACT-03 Impact provenance
`AffectedReport` gains `provenance: BTreeMap<String, Vec<ProvenanceHop>>` keyed by typed node ID (`"Kind:id"`). Each hop records `node: String` (typed ID) and `edge: Option<String>` (edge kind + direction, `None` for start). First path wins on multi-path reach; deterministic ordering via sorted queue. No enumeration of all paths.

<!-- @design:FR-IMPACT-02 -->
### FR-IMPACT-02 Affected ADRs
`AffectedReport` gains `adrs: Vec<AffectedNode>` for `NodeKind::Adr` nodes reached during traversal, categorized separately from impls/requirements/tests.

<!-- @design:FR-IMPACT-03 -->
### FR-IMPACT-03 Impact provenance
`AffectedReport` gains `provenance: BTreeMap<String, Vec<String>>` mapping each affected node ID to the ordered list of node IDs forming the traversal path from a changed node. First path wins on multi-path reach.

<!-- @design:FR-IMPACT-04 -->
### FR-IMPACT-04 Unmapped changes
`AffectedReport` gains `unmapped: Vec<PathBuf>` listing changed paths that map to no graph node. Computed from the normalized changed paths.

<!-- @design:FR-IMPACT-05 -->
### FR-IMPACT-05 Backward compatibility
Existing fields `impls`, `requirements`, `tests`, `commands` remain with same semantics and ordering. New fields are additive with `#[serde(default)]` where applicable. CLI text output keeps existing sections; new info (ADRs, unmapped) added as additional sections.

<!-- @design:FR-IMPACT-06 -->
### FR-IMPACT-06 Legacy same-ID fallback
Preserve the existing fallback: when an impl has no `Satisfies` edge but its ID exists as a Req, treat it as satisfying that req; when a req has no `Verifies` edge but its ID exists as a Test, treat it as verified by that test.

<!-- @design:FR-IMPACT-07 -->
### FR-IMPACT-07 Determinism
Visited set + sorted output (by file, line, id) and dedup. Cycle-safe by construction.

### Ordered implementation plan (P3.1)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| I1 | `spec-engine/src/affected/mod.rs`, `AffectedReport` | Add `adrs`, `unmapped`, `provenance` fields; existing fields unchanged. | `cargo test -p asep --lib affected` then `asep affected` |
| I2 | `spec-engine/src/affected/mod.rs`, `affected()` | Cycle-safe BFS traversal both directions; collects impl/req/test/adr; provenance recorded. | `cargo test -p asep --test affected_test` |
| I3 | `spec-engine/src/affected/mod.rs`, `affected()` | Unmapped changed paths reported. | `cargo test -p asep --test affected_test` |
| I4 | `spec-engine/src/affected/mod.rs`, legacy fallback | Same-ID fallback preserved. | `cargo test -p asep --test affected_test` |
| I5 | `spec-engine/src/runner.rs`, `Commands::Affected` | CLI text output shows ADRs and unmapped (additive); JSON includes new fields. | `cargo test -p asep --test affected_cli_test` |
| I6 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | Seven P3.1 requirements closed with relations. | `asep check . --rigor T2` after scoped acceptance |

## P3.2 — ADR Annotation Coverage (RFC v0.2)

Documentation-first, graph data-quality slice. Reuse existing graph builder, impact traversal, validators. No new engine, dependency, or abstraction.

<!-- @design:FR-ADRCOV-01 -->
### FR-ADRCOV-01 ADR start-node propagation
`spec-engine/src/affected/mod.rs`: when an ADR node is a *start node* (changed path maps to it), allow incoming traversal (find sources that `Informs` it) so dependent requirements/implementations are reported. Outgoing traversal from an ADR remains disabled.

<!-- @design:FR-ADRCOV-02 -->
### FR-ADRCOV-02 Terminal-sink preservation
`spec-engine/src/affected/mod.rs`: ADRs reached during traversal (non-start) remain terminal: no outgoing and no incoming traversal. Prevents false impact expansion through shared ADRs.

<!-- @design:FR-ADRCOV-03 -->
### FR-ADRCOV-03 Backward compatibility
No change to non-ADR node traversal, impact report fields, coverage rules, or legacy fallback.

### Ordered implementation plan (P3.2)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| A1 | `spec-engine/src/affected/mod.rs`, `affected()` | ADR start node propagates incoming Informs; non-start ADR stays sink. | `cargo test -p asep --test affected_test` |
| A2 | `spec-engine/tests/affected_test.rs` | Test: changed ADR reports dependent reqs/impls; shared ADR does not propagate between sources. | `cargo test -p asep --test affected_test` |
| A3 | `docs/TRACEABILITY.md` | Three P3.2 requirements closed with relations. | `asep check . --rigor T2` after scoped acceptance |

## Layer 5 — Agent Runtime (RFC v0.2)

Constraint: new audited source must obey the strict policy — sequential guard clauses, no `else`, no nested `if`, no inline opt-out. Requirements are observable; no new engine is introduced because adapters already suffice. Reuse existing `Role`, `CapabilityProfile`, `Envelope`, `TraceEvent`, `TraceCollector`.

<!-- @design:FR-AUDIT-01 -->
### FR-AUDIT-01 Redundancy audit (P0)
`docs/AUDIT.md` (new) enumerates: (a) existing agent-runtime mechanisms and their file/symbol, (b) their consumers, (c) overlap with native runtime capabilities, (d) baseline measurement plan. No code change; the artifact is the deliverable. A unit test in `spec-engine/tests/runtime_audit_test.rs` asserts the file exists and contains the four required section headings.

<!-- @design:FR-LEVEL-01 -->
### FR-LEVEL-01 EngineerLevel enum
`spec-engine/src/agent_protocol/mod.rs` (or new `spec-engine/src/runtime/level.rs`) defines `EngineerLevel { L1, L2, L3 }` with `as_str()` returning `"L1"`/`"L2"`/`"L3"`, `scope()` returning the scope text (function/component; module/subsystem; architecture/system), serde rename, and `Default = L1`. Unit tests cover round-trip and scope.

<!-- @design:FR-LEVEL-02 -->
### FR-LEVEL-02 CapabilityProfile.engineer_level
`spec-engine/src/config/settings.rs`: add `engineer_level: EngineerLevel` to `CapabilityProfile` with `#[serde(default)]` and default `L1`. Reuse the existing config load/save. Tests in `config_test.rs` cover default and round-trip.

<!-- @design:FR-LEVEL-03 -->
### FR-LEVEL-03 Envelope.level
`spec-engine/src/agent_protocol/mod.rs`: add `level: Option<EngineerLevel>` to `Envelope` with `#[serde(default)]` (additive; absent → None treated as L1). Bump is NOT required because field is optional with default; existing envelopes still parse. Tests cover round-trip and absent-default.

<!-- @design:FR-BUDGET-01 -->
### FR-BUDGET-01 ResourceBudget
New module `spec-engine/src/runtime/budget.rs` (or `spec-engine/src/agent_protocol/budget.rs`) defining `ResourceBudget { max_tokens: Option<u64>, max_tool_calls: Option<u64>, max_cost_usd: Option<f64>, max_latency_ms: Option<u64> }` with serde defaults (None) and `Default`. Unit tests cover round-trip and None handling.

<!-- @design:FR-BUDGET-02 -->
### FR-BUDGET-02 BudgetTracker
Same module: `BudgetTracker { budget: ResourceBudget, tokens: u64, tool_calls: u64, cost_usd: f64, latency_ms: u64 }` with `record_*` methods and `exceeded() -> Option<&'static str>` returning the first exceeded dimension name (`"tokens"`, `"tool_calls"`, `"cost"`, `"latency"`). Unit tests cover threshold and unlimited (None).

<!-- @design:FR-BUDGET-03 -->
### FR-BUDGET-03 Budget exceeded result
`exceeded()` result is formatted by callers as `BLOCKED: budget exceeded (<dimension>)`; provide a helper `blocked_message() -> Option<String>`. Unit test asserts exact string.

<!-- @design:FR-REACT-01 -->
### FR-REACT-01 Action and Observation
New module `spec-engine/src/runtime/react.rs` defining `Action { trace_id: String, seq: u64, tool: String, args: serde_json::Value }` and `Observation { trace_id: String, seq: u64, ok: bool, output: serde_json::Value, error: Option<String> }` with serde. Unit tests cover round-trip.

<!-- @design:FR-REACT-02 -->
### FR-REACT-02 ActionContract::validate
Same module: trait/struct `ActionContract` with `validate(&self, action: &Action, profile: &CapabilityProfile) -> Result<(), String>` rejecting missing tool name, empty trace_id, and capability mismatch (`BLOCKED: requires <capability>` reuse). Unit tests.

<!-- @design:FR-REACT-03 -->
### FR-REACT-03 Observation -> TraceEvent
Same module: `impl From<Observation> for TraceEvent` (or `to_trace_event(&self, task_id)`) producing a `TraceEvent` with event name `"observation"`, reusing `now_rfc3339` and `schema_version`. Unit test asserts conversion and that collector can record it.

### Ordered implementation plan (Layer 5)

| Order | File and symbol | Acceptance criteria | Verification |
|---|---|---|---|
| L0 | `docs/AUDIT.md` (new) | Four required sections present. | `cargo test -p asep --test runtime_audit_test` |
| L1 | `spec-engine/src/agent_protocol/mod.rs` or `spec-engine/src/runtime/level.rs`, `EngineerLevel` | Enum round-trips as L1/L2/L3; scope() returns text; Default L1. | `cargo test -p asep --lib agent_protocol` then `asep affected` |
| L2 | `spec-engine/src/config/settings.rs`, `CapabilityProfile.engineer_level` | Default L1; round-trip TOML; absent field defaults. | `cargo test -p asep --test config_test` |
| L3 | `spec-engine/src/agent_protocol/mod.rs`, `Envelope.level` | Optional field; round-trip; old envelopes parse. | `cargo test -p asep --lib agent_protocol` / `integration_check_test` |
| L4 | `spec-engine/src/runtime/budget.rs` (new), `ResourceBudget` | Optional fields; None = unlimited; round-trip. | `cargo test -p asep --lib runtime` then `asep affected` |
| L5 | `spec-engine/src/runtime/budget.rs`, `BudgetTracker` | Accumulate + exceeded() first dimension; unlimited. | `cargo test -p asep --lib runtime` |
| L6 | `spec-engine/src/runtime/budget.rs`, `blocked_message` | Exact `BLOCKED: budget exceeded (<dim>)`. | `cargo test -p asep --lib runtime` |
| L7 | `spec-engine/src/runtime/react.rs` (new), `Action`/`Observation` | Round-trip; trace_id/seq present. | `cargo test -p asep --lib runtime` |
| L8 | `spec-engine/src/runtime/react.rs`, `ActionContract::validate` | Rejects missing fields and capability mismatch. | `cargo test -p asep --lib runtime` |
| L9 | `spec-engine/src/runtime/react.rs`, `Observation -> TraceEvent` | Conversion reuses collector; event name observation. | `cargo test -p asep --lib runtime` + `observability` |
| L10 | `docs/TRACEABILITY.md`, `CHANGELOG.md` | All nine Layer-5 requirements closed with relations; changelog notes. | `asep check . --rigor T2` after scoped acceptance |

### Known consequences (Layer 5)
- Adding nine `@req` tags before `@impl`/`@test` land makes `asep check --rigor T1` fail COV-04/COV-05 until L10; final gate after scoped acceptance.
- `asep affected` cannot map new files until their tags land; L1–L9 rely on targeted `cargo test`.
- Full suite reserved for merges to `main`.
- New modules must be declared in `lib.rs` and registered in the module tree.

## Static engineering audit remediation — design (snapshot 4355dbc8)

Sumber: `ASEP_dev_audit_4355dbc.md`. Keputusan arsitektur dibuat di sini, bukan oleh executor: tidak ada engine/store/dependency baru; evidence memakai direktori `.spec-engine` yang ada; identitas snapshot memakai Git tree hash (content-addressed) + status working tree; graf typed memakai policy yang ada; trace report memakai satu builder bersama di `observability.rs`. Setiap unit kerja menyentuh satu file sumber dan satu file test, serta menambahkan tepat satu `@impl` dan satu `@test` per requirement (COV-06). Semua kode baru wajib mematuhi `FR-ANTISLOP-08`: tanpa `else`, tanpa `if` bersarang, dan docstring tidak boleh parroting nama identifier.

<!-- @design:FR-AUD-01 -->
<!-- @design:FR-AUD-02 -->
<!-- @design:FR-AUD-03 -->
<!-- @design:FR-AUD-04 -->
<!-- @design:FR-AUD-05 -->
<!-- @design:FR-AUD-06 -->
<!-- @design:FR-AUD-07 -->
<!-- @design:FR-AUD-08 -->
<!-- @design:FR-AUD-09 -->
<!-- @design:FR-AUD-10 -->
<!-- @design:FR-AUD-11 -->
<!-- @design:FR-AUD-12 -->
<!-- @design:FR-AUD-13 -->
<!-- @design:FR-AUD-14 -->
<!-- @design:FR-AUD-15 -->
<!-- @design:FR-AUD-16 -->
<!-- @design:FR-AUD-17 -->
<!-- @design:FR-AUD-18 -->
<!-- @design:FR-AUD-19 -->
<!-- @design:FR-AUD-20 -->
<!-- @design:FR-AUD-21 -->
<!-- @design:FR-AUD-22 -->
<!-- @design:FR-AUD-23 -->
<!-- @design:FR-AUD-24 -->
<!-- @design:FR-AUD-25 -->
<!-- @design:FR-AUD-26 -->
<!-- @design:FR-AUD-27 -->
<!-- @design:FR-AUD-28 -->
<!-- @design:FR-AUD-29 -->
<!-- @design:FR-AUD-30 -->
<!-- @design:FR-AUD-31 -->
<!-- @design:FR-AUD-32 -->
<!-- @design:FR-AUD-33 -->
<!-- @design:FR-AUD-34 -->
<!-- @design:FR-AUD-35 -->
<!-- @design:FR-OPT-00 -->

### Keputusan arsitektur kunci

**B02/B03/B04 — proof integrity.** `acceptance_scoped.rs` tidak boleh lagi mempercayai string commit. Sebelum gate, hitung `snapshot_identity`: `git rev-parse <commit>^{tree}` sebagai tree digest, `git rev-parse HEAD` harus sama dengan commit, `git status --porcelain` harus kosong (termasuk untracked), serta digest policy dan versi toolchain. Karena sandbox me-bind working tree secara read-only, clean tree + HEAD cocok menjamin isi sandbox sama dengan tree commit tanpa meng-materialisasi arsip baru. Evidence per-requirement ditulis dengan status `ACCEPTED` hanya untuk requirement in-scope yang benar-benar mencapai test tereksekusi; sisanya `UNPROVEN`. Header evidence memuat `schema_version`, `revision`, `tree_digest`, `changed_paths_digest`, `policy_digest`, `toolchain`, `verdict`, `reviewer`, `rationale`. `verification/mod.rs` memetakan kondisi ke state `PASS`, `MISSING`, `STALE`, `REJECTED`, `UNPROVEN`.

**B05 — test selection.** `build_test_plan` mengambil `commands` dari `asep affected --json`, menyisipkan `-p asep` bila belum ada, dan gagal bila `executed == 0`. Daftar hardcode `NONEXISTENT_TEST_TARGETS` dihapus; target yang tidak ada menjadi error eksplisit.

**B06/B07 — typed graph.** Satu `NodeKey` typed + tabel admissible `(source_kind, edge_kind, target_kind)` dari policy, dipakai ulang oleh traversal impact. Edge dengan note whitespace-only ditolak. ID yang sama antar kind tetap berbeda. Node dangling dilaporkan invalid/unproven, bukan node sintetis.

**B08 — scanner.** `scan_project_tags_detailed` memisahkan file non-teks, permission/IO error, dan root tidak ada; `ScanCompleteness` mencatat jumlah file dan error.

**B16/B18 — trace.** Satu `TraceReport::from_events` di `observability.rs` dipakai CLI dan MCP. Lookup memakai exact opaque key (bukan `starts_with`). Denominator per domain pengukuran.

**B26–B33 — srcdump/srcwalk.** Comment stripper memverifikasi prefix `/*`. Token dibedakan `original_tokens` vs `emitted_tokens`. Format dispatch mengikuti `config.format`. Symlink default tidak diikuti. Truncation menghitung ellipsis di dalam `max_len`. srcwalk menyetak semua flag git-ignore secara eksplisit.

**B35 — cross-platform (keputusan user).** Tidak ada import `std::os::unix` tanpa `cfg`; jalur Windows memakai implementasi `#[cfg(windows)]` setara.

### Ordered implementation plan

| Wave | ID | File & symbol | Acceptance criteria | Verification command |
|---|---|---|---|---|
| P0-1 | FR-AUD-01 | `spec-engine/src/context/mod.rs`, `spec-engine/tests/context_test.rs`; `.gitignore`, `vendor/cc/src/target/` | Modul dan test ter-git-add; clean checkout build tanpa E0583; vendor tidak diabaikan. | clean-worktree `cargo check -p asep --lib --offline --locked` |
| P0-2 | FR-AUD-02 | `acceptance_scoped.rs::snapshot_identity` + test | Dirty tree/HEAD beda → error; evidence memuat 6 field identitas. | `cargo test -p asep --test acceptance_scoped_test --offline --locked` |
| P1-1 | FR-AUD-03 | `acceptance_scoped.rs::in_scope_requirement_ids` | Requirement di luar scope → UNPROVEN; zero tests → error. | `cargo test -p asep --test acceptance_scoped_test --offline --locked` |
| P1-2 | FR-AUD-04 | `verification/mod.rs::evidence_state` | STALE/REJECTED/MISSING dibedakan. | `cargo test -p asep --lib verification --offline --locked` |
| P1-3 | FR-AUD-05 | `acceptance_scoped.rs::build_test_plan` | JSON commands, `-p`, executed>0. | `cargo test -p asep --test acceptance_scoped_test --offline --locked` |
| P1-4 | FR-AUD-06 | `graph/builder.rs::admissible_edge` | Kind tidak admissible tidak menutup COV; note whitespace ditolak. | `cargo test -p asep --test graph_test --offline --locked` |
| P1-5 | FR-AUD-07 | `affected/mod.rs` traversal | Dangling → invalid; provenance typed. | `cargo test -p asep --test affected_test --offline --locked` |
| P1-6 | FR-AUD-08 | `scanner/mod.rs::scan_project_tags_detailed` | Root invalid → error; binary/error tercatat. | `cargo test -p asep --test scanner_test --offline --locked` |
| P1-7 | FR-AUD-09 | `formal/mod.rs::FormalState` | Solver wajib absent → bukan PASS. | `cargo test -p asep --lib formal --offline --locked` |
| P1-8 | FR-AUD-10/11 | `harness/process.rs::spawn_shell_quiet`, `run_cmd_memory` | Exit nonzero → error; 1 MiB stdout sukses. | `cargo test -p asep --lib harness --offline --locked` |
| P1-9 | FR-AUD-12 | `harness/process.rs::process_tree_rss_bytes` | Tree RSS terukur; unsupported → UNAVAILABLE. | `cargo test -p asep --lib harness --offline --locked` |
| P1-10 | FR-AUD-13 | `harness/asymptotic.rs::CancellationToken` | Wall-clock terikat; label cooperative eksplisit. | `cargo test -p asep --lib harness --offline --locked` |
| P1-11 | FR-AUD-15 | `runtime/react.rs::Observation.task_id` | Task identity tidak tergabung. | `cargo test -p asep --lib runtime --offline --locked` |
| P1-12 | FR-AUD-16/18 | `observability.rs::TraceReport` | Exact key, denominator benar, CLI/MCP parity. | `cargo test -p asep --lib observability --offline --locked` |
| P1-13 | FR-AUD-17 | `config/settings.rs::default_allowlist` | Field wajib ada, secret opt-in. | `cargo test -p asep --test config_test --offline --locked` |
| P1-14 | FR-AUD-19/20 | `observability.rs::rotate`/`append_batch` | Rotasi unik; retry tidak duplikasi. | `cargo test -p asep --lib observability --offline --locked` |
| P1-15 | FR-AUD-21 | `config/settings.rs::validate_capabilities` | Config rusak fail-closed. | `cargo test -p asep --test config_test --offline --locked` |
| P1-16 | FR-AUD-23 | `agent_protocol/mod.rs::publish`/`ack` | Parallel send/ack tanpa partial/lost update. | `cargo test -p asep --lib agent_protocol --offline --locked` |
| P1-17 | FR-AUD-26 | `srcdump/formatter.rs::skip_block_comment` | Dua statement preserved. | `cargo test -p srcdump --offline --locked` |
| P1-18 | FR-AUD-28 | `srcdump/formatter.rs::TokenCounts` | original vs emitted tokens. | `cargo test -p srcdump --offline --locked` |
| P1-19 | FR-AUD-30 | `srcdump/walker.rs::is_safe_entry` | Symlink external tidak dibaca. | `cargo test -p srcdump --offline --locked` |
| P2-1 | FR-AUD-14 | `harness/process.rs::steps_from_output` | NaN/mixed metric → error; repeat + baseline. | `cargo test -p asep --lib harness --offline --locked` |
| P2-2 | FR-AUD-22 | `runtime/budget.rs::record_cost` | Negative/NaN/inf ditolak. | `cargo test -p asep --lib runtime --offline --locked` |
| P2-3 | FR-AUD-24 | `agent_protocol/mod.rs::validate_envelope_dir` | Duplicate/mismatch/orphan → error. | `cargo test -p asep --lib agent_protocol --offline --locked` |
| P2-4 | FR-AUD-25 | `scanner/lexer.rs::parse_tag` | Delimiter wajib; unknown priority error. | `cargo test -p asep --lib scanner --offline --locked` |
| P2-5 | FR-AUD-27 | `srcdump/formatter.rs::format_dispatch` | JSON satu dokumen; text bukan NDJSON. | `cargo test -p srcdump --offline --locked` |
| P2-6 | FR-AUD-29 | `srcdump/config.rs::resolve_options` | TOML precedence; jobs=1. | `cargo test -p srcdump --offline --locked` |
| P2-7 | FR-AUD-31/32 | `srcwalk/lib.rs` | Git-ignore flags; symlink/dedup contract. | `cargo test -p srcwalk --offline --locked` |
| P2-8 | FR-AUD-34 | `mcp/protocol.rs::parse_request` | Version/ID divalidasi; error terpisah. | `cargo test -p asep --test mcp_test --offline --locked` |
| P2-9 | FR-AUD-35 | `observability.rs` + `harness/process.rs` (cfg) | Cross-platform compile. | `cargo check --workspace --all-targets --offline --locked` |
| P3-1 | FR-AUD-33 | `srcdump/formatter.rs::truncate_line` | max_len termasuk ellipsis, Unicode aman. | `cargo test -p srcdump --offline --locked` |
| OPT | FR-OPT-00 | `docs/OPTIMIZATION_BENCHMARKS.md` + test | Adopsi hanya dengan before/after benchmark. | `cargo test -p asep --test optimization_evidence_test --offline --locked` |

### Verification scope per wave

Untuk setiap wave, jalankan `asep affected <changed-paths>` dan pakai keluaran sebagai verification command; gate `asep check . --rigor T1` dan `T2` dijalankan setelah traceability final. Clean-archive build diwajibkan sebelum `accept-scoped`. Setiap executor dilarang menjalankan `git checkout` tanpa pathspec karena dapat mengembalikan spec artifacts yang belum ter-commit.

### Known consequences — audit remediation (risiko yang belum tertutup)

1. **O01–O10 tidak diadopsi.** Status `documented, not adopted` di `docs/OPTIMIZATION_BENCHMARKS.md`. FR-OPT-00 melarang adopsi tanpa laporan benchmark before/after pada commit dan fixture yang sama (median/p95, alokasi, RSS, output_correctness identik). Ini keputusan disiplin, bukan kegagalan gate. Untuk menutup: jalankan benchmark matrix di bagian 4 audit, lalu ubah baris O menjadi `adopted` dengan tautan bukti, dan ulangi gate.
2. **Windows belum dikompilasi di environment ini.** `observability.rs` dan `harness/process.rs` sudah memakai `cfg(unix)`/`cfg(windows)`, tetapi hanya Linux yang dijalankan. Cabang Windows memakai `create_new(true)` dengan catatan batasan ACL; padanan penuh mode 0600 belum terverifikasi. Untuk menutup: tambahkan target Windows ke matrix CI dan jalankan `cargo check --workspace --all-targets --locked` pada runner Windows.
3. **FR-AUD-16 — tidak ada pembaca segmen di `observability.rs`.** Validasi `trace_id` pada event yang dibaca dari rotated segment tidak memiliki titik aplikasi di modul ini; exact-key lookup sudah diterapkan pada jalur penulisan, flush, purge, dan retention. Untuk menutup: bila pembaca segmen ditambahkan kelak, validasi `event.trace_id == trace` wajib disertakan.
4. **Item di luar cakupan audit masih deferred** sesuai keputusan sebelumnya: AD-D05 (workload quality-distinguishing), AD-D06 (failure reproducibility), AD-D07 (lifecycle hook), AD-D11 (backend visual), AD-D12 (migrasi), dan P3.3 (graph export). Ini tidak termasuk 35 temuan `ASEP_dev_audit_4355dbc.md` dan tidak dibuka oleh perbaikan ini.

### Completion ledger — post-refactor handoff (R01–R19)

Status legend: `FIXED` (behavior changed + regression evidence), `VERIFIED_EXISTING`
(revalidated present), `BLOCKED` (external dependency/authorization), `DEFERRED`
(explicitly out of scope this milestone).

| ID | Status | Current location / evidence |
|---|---|---|
| R01 | FIXED | Actual package/target/qualified test identities captured per command; name-only and first-target attribution removed; collision/unrelated-test regression added; `acceptance_scoped_test` 11/11 |
| R02 | FIXED | `config/settings.rs` canonical registry, `load_config_checked`, unknown names blocking; config lib 8/8 |
| R03 | FIXED | `ci/mod.rs::run_gate_1` explicit UNPROVEN on audit error; `watch/mod.rs` Result-returning summary |
| R04 | FIXED | `self_update.rs` digest verification + unique temp + rename-only replacement; 4/4 unit tests, no network |
| R05 | FIXED | Gate 4 removed from default and inert on explicit request; `ci::tests` 5/5; TDD benchmark text superseded |
| R06 | FIXED | `pack/manifest.rs` direct-execution presets + managed marker; `targets_test` 17/17; `opencode.json` synchronized |
| R07 | FIXED | Five skills rewritten as focused conditional references; canonical mirrors updated; `embedded_test` 7/7 |
| R08 | FIXED | Minimal init; ownership solely by explicit managed marker + content validation; directory/plug-in heuristics removed; `skill_cli_test` 4/4 |
| R09 | FIXED | `build_trace_graph`, `TraceabilityReport` and legacy projection types removed; CLI, MCP, verification, impact and coverage use one canonical `GraphReport`; verdict logic unified in `ci::check_core`. |
| R10 | FIXED | `context/mod.rs::compile_context_files` (no mandatory impact), canonicalized containment; CLI/MCP share it; `context_test` 6/6 |
| R11 | FIXED | `policy::tier_for_rigor_level` + `LEVEL_CALIBRATION.md`; removed `context_window`/`delegation_threshold` assumptions |
| R12 | PARTIAL | MCP version negotiation, minimal default exposure, no legacy mode, invalid params (-32602) distinguished from tool failures (-32000); `mcp_test` 30/30. Native-client pilot unavailable. |
| R13 | FIXED | Crates moved under `crates/spec-engine` and `crates/source-intelligence/`; templates/docs/ADRs regrouped; all paths updated; workspace check passes. |
| R14 | REMOVED | `pub mod snapshot`, implementation and dedicated tests deleted; Git snapshot identity in acceptance preserved. |
| R15 | FIXED | `clean::execute` requires explicit `--allow-full`; watch module and CLI removed; presets opt-in; telemetry off by default. |
| R16 | REMOVED | Fixture-dependent e2e script tests deleted from `harness_test`; meaningful harness correctness tests retained (4/4) with corrected workspace paths. No fixture fabricated. |
| R17 | FIXED (inventory below) | Every inherited B item classified; no unclassified finding remains |
| R18 | FIXED (static) | Dead queue/visited/reached initialization removed; no speculative optimization adopted; no performance claim. |
| R19 | FIXED | This ledger plus README/SKILLS/LEVEL_CALIBRATION synchronization; historical notes preserved as historical |

### Additional findings (N01–N10)

| ID | Status | Evidence |
|---|---|---|
| N01 | FIXED | Evidence records actual `test_packages`, `test_targets`, qualified names; collision and unrelated-test regressions added; acceptance 11/11 |
| N02 | FIXED | Evidence carries typed `evidence_state`; enum-based evaluation; verification 10/10 |
| N03 | FIXED | Managed markers + content validation; hidden-directory/plugins heuristics removed; unmanaged artifacts preserved; skill_cli 4/4 |
| N04 | FIXED | `GateResult.unproven`; explicitly requested unavailable gate is unproven; ci 5/5 |
| N05 | FIXED | `compile_context_files` does not call `affected`; context 6/6 |
| N06 | FIXED | `ci::check_core` is the shared verdict logic for CLI and MCP |
| N07 | FIXED | Root skill directories removed; `skills/` is the only canonical source |
| N08 | FIXED | Preset scaffolding opt-in via `enabled` flag and installer gate; targets 18/18 |
| N09 | FIXED | Minimal default MCP exposure; no legacy full-surface mode; mcp 29/29 |
| N10 | FIXED | `context_window`/`delegation_threshold` fields and MCP argument removed |

### Inherited audit finding classification (R17)

`FIXED` = issue not reproducible in current checkout or corrected here. `VERIFIED_EXISTING` = remediation present and covered by tests. `CONDITIONAL` = code-level risk with no bounded reproduction in this environment. `BLOCKED`/`DEFERRED` = external resource or approved deferral.

| Range | Classification | Notes |
|---|---|---|
| B01, B02, B04, B05, B06, B07, B09, B17, B22, B26, B30 | VERIFIED_EXISTING | Context module, snapshot binding, evidence states, zero-test guard, typed graph/impact, formal states, mandatory telemetry allowlist, budget validation, comment prefix guard, symlink containment all present with tests |
| B03, B16, B18, B21 | FIXED | Per-requirement acceptance (R01), exact trace keys, trace report denominators, config fail-closed (R02) |
| B25, B27, B30, B33, B34 | VERIFIED_EXISTING | Lexer unknown-priority diagnostic, srcdump format dispatch, symlink metadata containment, bounded truncation with ellipsis, JSON-RPC error categories all covered by tests |
| B08, B10–B15, B19–B20, B23–B24, B28–B29, B31–B32 | VERIFIED_EXISTING | Revalidated against current source: scanner I/O errors explicit; harness exit-status checks, stdout draining, finite numeric validation and task-id validation present; trace rotation uses monotonic ordinals with create_new and safe file modes; message store uses unique temp files, duplicate/filename checks; srcdump separates original/emitted tokens and reports skips; srcwalk canonicalizes and dedups symlinks. Covered by bounded tests. |
| B35 | CONDITIONAL | `cfg(unix)`/`cfg(windows)` branches present; Windows target not available here, so cross-platform behavior remains unverified. Reopen with a Windows runner. |
| B10–B14 | BLOCKED | Harness process/complexity findings require missing e2e scripts or heavy workloads; benchmarks and stress tests prohibited |
| O01–O10 | DEFERRED | Optimization hypotheses; O01 rejected on prior evidence; no performance claims without approved measurement |

### Known consequences — simplification & consolidation (S0–S6)

Catatan pengerjaan yang belum selesai dan risiko yang tersisa dari inisiatif ini.
Semua butir di bawah adalah **risiko kondisional**, bukan regresi terverifikasi.

1. **Benchmark ditunda.** Tidak ada benchmark, stress test, load test, atau benchmark gate yang dijalankan pada S0–S6. O01–O10 tetap hipotesis; O01 sudah diukur sebelum milestone ini dan ditolak (median regresi), kode associated dibiarkan utuh tapi tidak diadopsi. Tidak ada klaim peningkatan performa. Penutupan: setujui isolasi resource dulu, lalu ukur dengan before/after pada commit dan fixture yang sama.
2. **`harness_test`: 6 kegagalan karena fixture eksternal hilang.** Skrip `new-run.sh`, `vendor-loom.sh`, dan verifier tidak ada di repo (exit 127). Ini kondisi lokal yang sudah ada sebelumnya, bukan efek perubahan ini. Penutupan: sediakan fixture resmi, jangan mengarang pengganti.
3. **MVCC telah dihapus.** Modul snapshot, export publik dan tes dedicated dihapus; identitas snapshot Git untuk acceptance tetap dipertahankan.
4. **Versi protokol MCP masih hardcoded** (`2024-11-05` di `mcp/server.rs`). Negosiasi versi memerlukan pilot native-client; mengubahnya sekarang berisiko memecah klien yang ada.
5. **Installer tidak menyegarkan salinan generated yang sudah ada.** `install_skill` melewati skill yang sudah hadir, sehingga salinan usang tidak diperbarui saat upgrade. Sudah dimitigasi di checkout ini dengan regenerasi; perbaikan kode harus menghindari menimpa kustomisasi pengguna.
6. **Reorganisasi direktori penuh ditunda.** Target tree di blueprint bersifat ilustratif dan pemindahan path memecah referensi. Boundary crate workspace sengaja dipertahankan. Penutupan: keputusan kompatibilitas + migrasi path.
7. **Sebagian temuan warisan belum direproduksi satu per satu.** S0–S5 merevalidasi subset prioritas (B01, B02, B03, B04, B05, B09, B16, B17, B21, B26, B30); sisanya tetap revalidation target, bukan temuan baru yang terverifikasi.
8. **`dev` dilindungi di kedua remote.** Push langsung ditolak; pekerjaan ada di branch `refactor/asep-simplification` dan memerlukan PR. Branch lokal `dev` tertinggal dari `origin/dev` secara history (sudah di-merge secara lokal, konten identik).
9. **`accept-scoped` memerlukan trusted runner.** Agent tidak boleh mengeluarkan `ACCEPTED`; tree bersih setelah commit hanya prasyarat, bukan jaminan acceptance.

Bukti verifikasi untuk milestone ini: 409 tes `asep` lulus (semua integration suite kecuali `harness_test`), 144/144 tes lib, dan `cargo check --workspace --all-targets --offline --locked` lulus.

## Audit backlog execution — desain dan rencana

> **Superseded (benchmark deferral):** paragraf dan perintah benchmark di bawah ini adalah catatan historis dari backlog audit, bukan instruksi aktif. Penundaan benchmark oleh user mengoverride rencana ini. `cargo bench` (termasuk `--no-run`), stress/load, Loom, dan CI penuh **dilarang** pada milestone ini. Gate 4 tidak termasuk default CI dan tidak mengeksekusi Cargo. FR-OPT-00 tetap berlaku: tidak ada adopsi atau klaim performa tanpa pengukuran yang disetujui kemudian.

Keputusan historis: jalankan **semua** O01–O10 dan tutup butir 3 (pembaca trace). Tidak ada engine/dependency/`unsafe` baru. Benchmark memakai `criterion 0.5` yang sudah ada di `spec-engine/benches/engine_benchmarks.rs`; dijalankan dengan `cargo bench -p spec-engine --bench engine_benchmarks -- <filter>`. Setiap kandidat dicatat di `docs/OPTIMIZATION_BENCHMARKS.md` dengan before/after pada commit dan fixture sama; adopsi hanya bila median/p95 membaik dan output identik. Karena `cargo bench` tidak mengukur alokasi/RSS secara native, angka alokasi/RSS diambil dari `harness/e2e` bila tersedia atau dinyatakan `not measured` secara eksplisit — tidak boleh diklaim terukur.

<!-- @design:FR-PERF-01 -->
<!-- @design:FR-PERF-02 -->
<!-- @design:FR-PERF-03 -->
<!-- @design:FR-PERF-04 -->
<!-- @design:FR-PERF-05 -->
<!-- @design:FR-PERF-06 -->
<!-- @design:FR-PERF-07 -->
<!-- @design:FR-PERF-08 -->
<!-- @design:FR-PERF-09 -->
<!-- @design:FR-PERF-10 -->
<!-- @design:FR-TRACE-READ-01 -->

### Keputusan arsitektur

- **Graph (O01–O03):** satu `TypedIndex { by_key: HashMap<NodeKey, NodeId>, outgoing/incoming: HashMap<NodeKey, Vec<EdgeRef>>, path_to_ids }` dibangun sekali di `affected()`; provenance disimpan sebagai predecessor per node dan direkonstruksi saat emit; line index per file dan manifest per crate di-cache per command.
- **Observability (O04–O05):** `pending_count: usize` dipertahankan inkremental; batch di-`take()` dengan recovery owner; `BufWriter` untuk write; hashing streaming dengan buffer tetap dan identitas konten berbasis hash penuh.
- **Parser (O06):** `ParserRegistry` dengan pool worker terikat; query/tokenizer lazy; parse sekali per file per request.
- **Walker (O07):** satu kontrak filter dipakai srcwalk dan srcdump; direktori exclude dipangkas sebelum descent; sniff biner + baca teks digabung.
- **MCP (O08):** pembatas ukuran/jumlah respons dengan cursor dan counts; NDJSON streaming ke writer; tanpa silent truncation.
- **Acceptance (O09):** `ScratchGuard` RAII dengan `create_dir` unik dan `Drop` cleanup; cache compiler tepercaya dipisah dari state build.
- **Build (O10):** ukur dulu `cargo tree -e features`, timing, dan ukuran target; feature-gate hanya dengan bukti.
- **Trace reader (#3):** `read_trace_events(root, trace_id)` di `observability.rs` memakai `trace_key`, membaca file aktif + segmen terurut, memfilter event dengan `event.trace_id != trace_id`, dipakai ulang oleh `runner.rs` dan `mcp/tools.rs`.

### Ordered implementation plan

| Order | Requirement | File & symbol | Acceptance criteria | Verification command |
|---|---|---|---|---|
| 1 | FR-TRACE-READ-01 | `observability.rs::read_trace_events`, `runner.rs`, `mcp/tools.rs::build_trace_summary` | Event trace beda dengan prefix sama tidak muncul; CLI/MCP parity. | `cargo test -p asep --lib observability --offline --locked` + `cargo test -p asep --test trace_cli_test --offline --locked` |
| 2 | FR-PERF-01 | `affected/mod.rs::TypedIndex` | Traversal O(V+E), impacted IDs identik pada 4 fixture. | `cargo test -p asep --test affected_test --offline --locked` (benchmark deferred; no performance claim) |
| 3 | FR-PERF-02 | `affected/mod.rs` provenance reconstruction | Chain identik, alokasi turun. | `cargo test -p asep --test affected_test --offline --locked` |
| 4 | FR-PERF-03 | `graph/builder.rs`, `affected/mod.rs::line_index` | Output/diagnostik identik, lookup terinduksi. | `cargo test -p asep --test graph_test --offline --locked` |
| 5 | FR-PERF-04 | `observability.rs::pending_count`, `flush` | Redaction/durability test lulus; O(1) count. | `cargo test -p asep --lib observability --offline --locked` |
| 6 | FR-PERF-05 | `observability.rs::hash_file`, `agent_protocol/mod.rs` | Hash byte-identik, buffer tetap. | `cargo test -p asep --lib observability --offline --locked` |
| 7 | FR-PERF-06 | `srcidx/src/parser/mod.rs::ParserRegistry` | Symbols identik, pool terikat. | `cargo test -p srcidx --offline --locked` |
| 8 | FR-PERF-07 | `srcwalk/src/lib.rs`, `srcdump/src/walker.rs` | File emitted identik, prune sebelum descent. | `cargo test -p srcwalk --offline --locked` + `cargo test -p srcdump --offline --locked` |
| 9 | FR-PERF-08 | `mcp/tools.rs`, `mcp/server.rs` | Response terikat, cursor eksplisit, NDJSON streaming. | `cargo test -p asep --test mcp_test --offline --locked -- --test-threads=1` |
| 10 | FR-PERF-09 | `acceptance_scoped.rs::ScratchGuard` | Scratch dihapus saat error; cache key lengkap. | `cargo test -p asep --test acceptance_scoped_test --offline --locked` |
| 11 | FR-PERF-10 | `spec-engine/Cargo.toml`, `Cargo.toml`, `docs/OPTIMIZATION_BENCHMARKS.md` | Laporan `cargo tree`/timing/target-size di-commit; tidak ada `unsafe`/allocator swap. | `cargo tree --workspace -e features --offline --locked` + `cargo check --workspace --all-targets --offline --locked` |

### Aturan verifikasi tambahan

- Setiap langkah 2–11 menghasilkan satu baris benchmark di `docs/OPTIMIZATION_BENCHMARKS.md` sebelum adopsi; bila tidak ada perbaikan, baris tetap `documented, not adopted` dan kode tidak diubah.
- Gate T1/T2 dijalankan setelah semua relasi `satisfies`/`verifies` untuk 11 requirement baru ditambahkan; `asep check` akan merah sampai tag `@impl`/`@test` dan relasi tersebut tersedia.
- Executor dilarang `git checkout` tanpa pathspec, dilarang `cargo fmt --all` yang menyentuh file di luar scope, dan dilarang mengubah `docs/PRD.md`/`docs/TDD.md`.

