# Traceability relations (@rel:satisfies)

@rel:satisfies:FR-CLI-01->FR-CLI-01
@rel:satisfies:FR-CLI-02->FR-CLI-02
@rel:satisfies:FR-CLI-03->FR-CLI-03
@rel:satisfies:FR-CLI-04->FR-CLI-04
@rel:satisfies:FR-CONFIG-01->FR-CONFIG-01
@rel:satisfies:FR-MCP-01->FR-MCP-01
@rel:satisfies:FR-MCP-02->FR-MCP-02
@rel:satisfies:FR-PACK-01->FR-PACK-01
@rel:satisfies:FR-PACK-02->FR-PACK-02
@rel:satisfies:FR-SKILL-01->FR-SKILL-01
@rel:satisfies:FR-SKILL-02->FR-SKILL-02
@rel:satisfies:FR-VERIFY-01->FR-VERIFY-01
@rel:satisfies:FR-INIT-01->FR-INIT-01
@rel:satisfies:FR-CLI-SCAN-01->FR-CLI-01
@rel:satisfies:FR-CLI-SCAN-01-B->FR-CLI-01
@rel:satisfies:FR-CLI-SCAN-02->FR-CLI-01
@rel:satisfies:FR-CLI-CHECK-01->FR-CLI-02
@rel:satisfies:FR-CLI-CHECK-02->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-01->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-A->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-B->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-C->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-D->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-E->FR-CLI-02
@rel:satisfies:FR-CLI-GRAPH-02-F->FR-CLI-02
@rel:satisfies:FR-CLI-ADR-01->FR-CLI-03
@rel:satisfies:FR-CLI-INIT-01->FR-CLI-03
@rel:satisfies:FR-CLI-RENDER-01->FR-CLI-03
@rel:satisfies:FR-CLI-DOC-01-A->FR-CLI-04
@rel:satisfies:FR-CLI-DOC-01-B->FR-CLI-04
@rel:satisfies:FR-CLI-DOC-02->FR-CLI-04
@rel:satisfies:FR-CONFIG-LOAD-01->FR-CONFIG-01
@rel:satisfies:FR-CONFIG-LOAD-02->FR-CONFIG-01
@rel:satisfies:FR-CONFIG-SAVE-01->FR-CONFIG-01
@rel:satisfies:FR-MCP-PROTO-01-A->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-01-B->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-02-A->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-02-C->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-03-A->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-03-B->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-04->FR-MCP-01
@rel:satisfies:FR-MCP-PROTO-02-B->FR-MCP-02
@rel:satisfies:FR-PACK-MANIFEST-01->FR-PACK-01
@rel:satisfies:FR-PACK-MANIFEST-02->FR-PACK-01
@rel:satisfies:FR-PACK-EMBED-01->FR-PACK-01
@rel:satisfies:FR-PACK-EMBED-02->FR-PACK-01
@rel:satisfies:FR-PACK-INSTALL-01->FR-PACK-02
@rel:satisfies:FR-PACK-INSTALL-02->FR-PACK-02
@rel:satisfies:FR-PACK-TARGETS-01->FR-PACK-02
@rel:satisfies:FR-PACK-TARGETS-02->FR-PACK-02
@rel:satisfies:FR-SKILL-INSPECT-01-A->FR-SKILL-01
@rel:satisfies:FR-SKILL-INSPECT-01-B->FR-SKILL-01
@rel:satisfies:FR-SKILL-DRIFT-01->FR-SKILL-01
@rel:satisfies:FR-SKILL-CRUD-01->FR-SKILL-02
@rel:satisfies:FR-SKILL-PORT-01->FR-SKILL-02
@rel:satisfies:FR-VERIFY-01-B->FR-VERIFY-01
@rel:satisfies:FR-VERIFY-01-C->FR-VERIFY-01
@rel:satisfies:FR-CLI-INIT-01->FR-INIT-01

# Traceability relations (@rel:verifies)

@rel:verifies:FR-CLI-01->FR-CLI-01 "parse_tags_from_line extracts \@req:FR-CLI-01:MUST from a source line"
@rel:verifies:FR-CLI-02->FR-CLI-02 "load_policy().exit_codes() yields pass=0 fail=1 unproven=3"
@rel:verifies:FR-CLI-03->FR-CLI-03 "CLI parses init, adr new, render subcommands"
@rel:verifies:FR-CLI-04->FR-CLI-04 "audit_doc_contracts reports parroting when docstring repeats fn name"
@rel:verifies:FR-CONFIG-01->FR-CONFIG-01 "save/load config round-trip and CapabilityProfile::missing reports disabled capabilities"
@rel:verifies:FR-MCP-01->FR-MCP-01 "JsonRpcRequest deserializes and malformed input yields -32700 parse error"
@rel:verifies:FR-MCP-02->FR-MCP-02 "get_tools_list lists asep_check, spec_check, adr_new, render_diagrams, srcidx, srcdump"
@rel:verifies:FR-PACK-01->FR-PACK-01 "generate_plugin_json emits the asep manifest and embedded skills carry real content"
@rel:verifies:FR-PACK-02->FR-PACK-02 "TargetAgent::all has 11 agents; detection and resolve_target_paths behave as specified"
@rel:verifies:FR-SKILL-01->FR-SKILL-01 "detect_skill_drift on an empty temp dir returns no drift records"
@rel:verifies:FR-SKILL-02->FR-SKILL-02 "add_skill normalizes the id and delete_skill removes the skill dir"
@rel:verifies:FR-VERIFY-01->FR-VERIFY-01 "evaluate yields Fail/Unproven/Pass for broken graph, missing evidence, and controlled inputs"
@rel:verifies:FR-INIT-01->FR-INIT-01 "init_asep_project creates the docs tree and writes PRD.md with \@req:FR-INIT-01:MUST"
@rel:satisfies:FR-ANTISLOP-05->FR-ANTISLOP-05
@rel:satisfies:FR-ANTISLOP-06->FR-ANTISLOP-06
@rel:satisfies:FR-ANTISLOP-07->FR-ANTISLOP-07
@rel:satisfies:FR-ANTISLOP-08->FR-ANTISLOP-08
@rel:satisfies:FR-ANTISLOP-09->FR-ANTISLOP-09
@rel:verifies:FR-ANTISLOP-05->FR-ANTISLOP-05 "audit_anti_slop flags every else token as an Error-severity violation"
@rel:verifies:FR-ANTISLOP-06->FR-ANTISLOP-06 "audit_anti_slop flags any if nested inside another if with max_nesting=0"
@rel:verifies:FR-ANTISLOP-07->FR-ANTISLOP-07 "code_rule reads AS-CODE-ELSE and AS-CODE-NESTING severity from asep-policy.toml"
@rel:verifies:FR-ANTISLOP-08->FR-ANTISLOP-08 "audited repository sources contain no else token and no nested if"
@rel:verifies:FR-ANTISLOP-09->FR-ANTISLOP-09 "asep:allow-antislop does not suppress AS-CODE-ELSE or AS-CODE-NESTING"

## Bukti pilot build

<!-- @test:BUILD-PILOT-01 -->
| Metrik | A lalu B: baseline ke kandidat | B lalu A: baseline ke kandidat |
|---|---|---|
| Median edit fungsi | 4,605 ke 4,261 s (turun 7,5%) | 8,162 ke 6,601 s (turun 19,1%) |
| Median edit test | 2,345 ke 2,102 s (turun 10,4%) | 4,182 ke 3,272 s (turun 21,8%) |
| Disk puncak teramati | 1.307.504.640 ke 1.063.677.952 byte (turun 18,6%) | 1.309.093.888 ke 1.063.706.624 byte (turun 18,7%) |
| Disk tersisa | 1.217.093.632 ke 995.127.296 byte (turun 18,2%) | 1.217.122.304 ke 995.164.160 byte (turun 18,2%) |

<!-- @test:BUILD-PILOT-02 -->
Keempat run memakai cache eksperimen, bukan `target/` utama. Daftar 9 test `harness_e2e_test` sama dan tiap ulangan lulus 9/9. Bukti lokal: `run.json` di `/tmp/opencode/asep-pilot-A/`, `/tmp/opencode/asep-pilot-B/`, `/tmp/opencode/asep-pilot-B2/`, dan `/tmp/opencode/asep-pilot-A2/`.

@rel:satisfies:BUILD-PILOT-01->BUILD-PILOT-01
@rel:satisfies:BUILD-PILOT-02->BUILD-PILOT-02
@rel:verifies:BUILD-PILOT-01->BUILD-PILOT-01 "Dua putaran dengan urutan terbalik mencatat waktu dan disk pada skenario yang sama"
@rel:verifies:BUILD-PILOT-02->BUILD-PILOT-02 "Kedua putaran memenuhi syarat adopsi dengan daftar dan hasil test yang sama"

## Bukti lanjutan optimasi build

<!-- @test:BUILD-NEXT-01 -->
Audit mendapati 1.205.305.344 byte di `target/debug/build` tanpa penghapusan. Benchmark konsolidasi memakai enam kasus identik, dengan `scanner::` dinormalisasi saat membandingkan nama. Semua kasus lulus dan log artefak Cargo mengonfirmasi `lexer_test` dibangun ulang pada tiap edit. Median edit baseline 1,511 s (1,411/1,746/1,511 s); kandidat 1,981 s (1,939/2,242/1,981 s). Artefak tersisa turun dari 781.291.520 ke 760.463.360 byte. Perbandingan valid ini memakai satu urutan baseline lalu kandidat; hasil cukup untuk menahan adopsi, bukan klaim bahwa konsolidasi selalu lebih lambat. Bukti lokal: `/tmp/opencode/asep-build-next-2b-fixed/measure_verified-20261122T064142Z-1092.json`.

@rel:satisfies:BUILD-NEXT-01->BUILD-NEXT-01
@rel:verifies:BUILD-NEXT-01->BUILD-NEXT-01 "Audit disk dan benchmark enam kasus dengan bukti rebuild menentukan keputusan konsolidasi"

<!-- @test:BUILD-NEXT-02 -->
Graf unit `lexer_test` turun dari 342 ke 336; hanya unit `serde_spanned` 0.6.9, `toml` 0.8.23, `toml_datetime` 0.6.11, `toml_edit` 0.22.27, `toml_write` 0.1.2, dan `winnow` 0.7.15 yang hilang. Kedua versi lulus lima kasus lexer dan tiga kasus konfigurasi, termasuk error parsing bertipe. Build awal 273,125 ke 227,059 s. Median edit 1,681 ke 2,300 s. Disk puncak teramati dan tersisa 937.979.904 ke 905.867.264 byte. Satu urutan baseline lalu kandidat; hasil mendasari keputusan untuk tidak adopsi, bukan estimasi efek universal. Bukti lokal: `/tmp/opencode/asep-build-next-3-parity/` dan `/tmp/opencode/asep-build-next-3-benchmark/toml-1795333025519548595.json`.

@rel:satisfies:BUILD-NEXT-02->BUILD-NEXT-02
@rel:verifies:BUILD-NEXT-02->BUILD-NEXT-02 "Graf unit, tes konfigurasi, dan benchmark terisolasi menentukan keputusan penyatuan TOML"

<!-- @test:BUILD-NEXT-03 -->
Build dari cache kosong: 242,014 s (2 job) vs 206,190 s (4 job). Median tiga rebuild edit: 5,222 vs 5,909 s, dengan rentang tumpang tindih. Artefak tersisa sama (620.298.240 byte). Log: `/tmp/opencode/asep-build-next-4/run2.stdout`.

Antrean, 4 job per proses: serial 14,858 dan 11,833 s; serentak 15,339 dan 11,571 s. Kedua ulangan serentak melaporkan tunggu lock; serial tidak. Durasi tunggu lock tidak terukur. Puncak teramati: serial 1.410.342.912–1.425.666.048 byte; serentak 1.456.300.032–1.456.308.224 byte. Artefak tersisa sama (1.409.732.608 byte). Lima kasus lexer lulus di tiap ulangan dan Cargo mengonfirmasi rebuild. Bukti lokal: `/tmp/opencode/asep-build-next-4/queue_only-20261122T083319Z-9728/metrics.json`.

@rel:satisfies:BUILD-NEXT-03->BUILD-NEXT-03
@rel:verifies:BUILD-NEXT-03->BUILD-NEXT-03 "Perbandingan job dan antrean mencatat wall time, disk, dan bukti tunggu lock tanpa mengubah default"
# Traceability relations (@rel:satisfies) — capability and ambient tracks

@rel:satisfies:FR-CAP-01->FR-CAP-01
@rel:satisfies:FR-CAP-02->FR-CAP-02
@rel:satisfies:FR-CAP-03->FR-CAP-03
@rel:satisfies:FR-CAP-04->FR-CAP-04
@rel:satisfies:FR-CAP-05->FR-CAP-05
@rel:satisfies:FR-CAP-06->FR-CAP-06
@rel:satisfies:FR-CAP-07->FR-CAP-07
@rel:satisfies:FR-CAP-08->FR-CAP-08
@rel:satisfies:FR-AMB-01->FR-AMB-01
@rel:satisfies:FR-AMB-02->FR-AMB-02
@rel:satisfies:FR-AMB-03->FR-AMB-03
@rel:satisfies:FR-AMB-04->FR-AMB-04
@rel:satisfies:FR-AMB-05->FR-AMB-05
@rel:satisfies:FR-AMB-06->FR-AMB-06

# Traceability relations (@rel:verifies) — capability and ambient tracks

@rel:verifies:FR-CAP-01->FR-CAP-01 "config_test asserts capability media and threshold defaults and round-trip"
@rel:verifies:FR-CAP-02->FR-CAP-02 "mcp_test asserts asep_agent_send schema and backward-compatible payload"
@rel:verifies:FR-CAP-03->FR-CAP-03 "config_test asserts covers returns BLOCKED for missing capability or media"
@rel:verifies:FR-CAP-04->FR-CAP-04 "config_test asserts delegation_threshold default and persistence"
@rel:verifies:FR-CAP-05->FR-CAP-05 "agent_protocol_test asserts typed optional tool_call_count and duplicate_calls"
@rel:verifies:FR-CAP-06->FR-CAP-06 "targets_test asserts judge prompt carries the trajectory rule"
@rel:verifies:FR-CAP-07->FR-CAP-07 "targets_test asserts capability-domain worker presets are generated"
@rel:verifies:FR-CAP-08->FR-CAP-08 "config_test asserts media routing rejects unsupported media"
@rel:verifies:FR-AMB-01->FR-AMB-01 "diagram_and_init_test asserts pointer block and flat skills scaffold idempotently"
@rel:verifies:FR-AMB-02->FR-AMB-02 "pack_test asserts pack writes no skill files"
@rel:verifies:FR-AMB-03->FR-AMB-03 "embedded_test asserts using-asep bootstrap skill is embedded and bounded"
@rel:verifies:FR-AMB-04->FR-AMB-04 "embedded_test asserts description-first and token budget"
@rel:verifies:FR-AMB-05->FR-AMB-05 "skill_status_test asserts extract_skill_requires parses inline and dash lists"
@rel:verifies:FR-AMB-06->FR-AMB-06 "integration_check_test asserts ambient governance gate reports violations"

@rel:satisfies:FR-CAP-09->FR-CAP-09
@rel:verifies:FR-CAP-09->FR-CAP-09 "pack_test constructs PackOptions without skills_filter and pack writes no skills"

@rel:satisfies:FR-ACC-02->FR-ACC-02
@rel:verifies:FR-ACC-02->FR-ACC-02 "acceptance_scoped_test asserts non-root scoped acceptance writes evidence"
@rel:satisfies:FR-ACC-03->FR-ACC-03
@rel:verifies:FR-ACC-03->FR-ACC-03 "integration_check_test asserts --evidence-dir supplies T2 semantic evidence"
@rel:satisfies:FR-ACC-04->FR-ACC-04
@rel:verifies:FR-ACC-04->FR-ACC-04 "acceptance_scoped_test asserts no full-suite gate and scoped tests"

@rel:satisfies:FR-ACC-05->FR-ACC-05
@rel:verifies:FR-ACC-05->FR-ACC-05 "integration_check_test asserts config evidence_dir makes T2 pass without flag"
