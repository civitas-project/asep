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

## Pilot build evidence

<!-- @test:BUILD-PILOT-01 -->
| Metric | A then B: baseline to candidate | B then A: baseline to candidate |
|---|---|---|
| Median function edit | 4.605 to 4.261 s (down 7.5%) | 8.162 to 6.601 s (down 19.1%) |
| Median test edit | 2.345 to 2.102 s (down 10.4%) | 4.182 to 3.272 s (down 21.8%) |
| Peak disk observed | 1,307,504,640 to 1,063,677,952 bytes (down 18.6%) | 1,309,093,888 to 1,063,706,624 bytes (down 18.7%) |
| Retained disk usage | 1,217,093,632 to 995,127,296 bytes (down 18.2%) | 1,217,122,304 to 995,164,160 bytes (down 18.2%) |

<!-- @test:BUILD-PILOT-02 -->
All four runs use the experiment cache, not the main `target/`. The list of 9 `harness_e2e_test` tests is the same and each run passes 9/9. Local evidence: `run.json` in `/tmp/opencode/asep-pilot-A/`, `/tmp/opencode/asep-pilot-B/`, `/tmp/opencode/asep-pilot-B2/`, and `/tmp/opencode/asep-pilot-A2/`.

@rel:satisfies:BUILD-PILOT-01->BUILD-PILOT-01
@rel:satisfies:BUILD-PILOT-02->BUILD-PILOT-02
@rel:verifies:BUILD-PILOT-01->BUILD-PILOT-01 "Two rounds with reversed order record time and disk on the same scenario"
@rel:verifies:BUILD-PILOT-02->BUILD-PILOT-02 "Both rounds meet adoption criteria with the same test list and results"

## Build follow-up evidence

<!-- @test:BUILD-NEXT-01 -->
Audit found 1,205,305,344 bytes in `target/debug/build` without deletion. Consolidation benchmark uses six identical cases, with `scanner::` normalized when comparing names. All cases pass and the Cargo artifact log confirms `lexer_test` is rebuilt on each edit. Median edit baseline 1.511 s (1.411/1.746/1.511 s); candidate 1.981 s (1.939/2.242/1.981 s). Remaining artifacts down from 781,291,520 to 760,463,360 bytes. This valid comparison uses one baseline-then-candidate sequence; result is enough to withhold adoption, not a claim that consolidation is always slower. Local evidence: `/tmp/opencode/asep-build-next-2b-fixed/measure_verified-20261122T064142Z-1092.json`.

@rel:satisfies:BUILD-NEXT-01->BUILD-NEXT-01
@rel:verifies:BUILD-NEXT-01->BUILD-NEXT-01 "Disk audit and six-case benchmark with rebuild evidence determine the consolidation decision"

<!-- @test:BUILD-NEXT-02 -->
Unit graph `lexer_test` down from 342 to 336; only units `serde_spanned` 0.6.9, `toml` 0.8.23, `toml_datetime` 0.6.11, `toml_edit` 0.22.27, `toml_write` 0.1.2, and `winnow` 0.7.15 are missing. Both versions pass five lexer cases and three config cases, including typed error parsing. Initial build 273.125 to 227.059 s. Median edit 1.681 to 2.300 s. Peak and retained disk usage observed 937,979,904 to 905,867,264 bytes. One baseline-then-candidate sequence; result underlies the decision not to adopt, not a universal effect estimate. Local evidence: `/tmp/opencode/asep-build-next-3-parity/` and `/tmp/opencode/asep-build-next-3-benchmark/toml-1795333025519548595.json`.

@rel:satisfies:BUILD-NEXT-02->BUILD-NEXT-02
@rel:verifies:BUILD-NEXT-02->BUILD-NEXT-02 "Unit graph, config tests, and isolated benchmark determine the TOML unification decision"

<!-- @test:BUILD-NEXT-03 -->
Build from empty cache: 242.014 s (2 jobs) vs 206.190 s (4 jobs). Median of three rebuild edits: 5.222 vs 5.909 s, with overlapping range. Remaining artifacts same (620,298,240 bytes). Log: `/tmp/opencode/asep-build-next-4/run2.stdout`.

Queue, 4 jobs per process: serial 14.858 and 11.833 s; concurrent 15.339 and 11.571 s. Both concurrent runs report lock wait; serial does not. Lock wait duration unmeasured. Peak observed: serial 1,410,342,912–1,425,666,048 bytes; concurrent 1,456,300,032–1,456,308,224 bytes. Remaining artifacts same (1,409,732,608 bytes). Five lexer cases pass in each run and Cargo confirms rebuild. Local evidence: `/tmp/opencode/asep-build-next-4/queue_only-20261122T083319Z-9728/metrics.json`.

@rel:satisfies:BUILD-NEXT-03->BUILD-NEXT-03
@rel:verifies:BUILD-NEXT-03->BUILD-NEXT-03 "Job and queue comparison records wall time, disk, and lock-wait evidence without changing defaults"

## Full Rust P0

@rel:satisfies:FR-LEADER-01->FR-LEADER-01
@rel:verifies:FR-LEADER-01->FR-LEADER-01 "New preset installs leader as primary and does not generate a lead key"
@rel:satisfies:FR-LEADER-02->FR-LEADER-02
@rel:verifies:FR-LEADER-02->FR-LEADER-02 "MCP rejects the lead role; protocol test also checks leader v2 serialization and v1 envelope rejection"

### P0 evidence and deployment limits

<!-- @test:FR-LEADER-01 -->
`targets_test` passes 13/13. New preset uses `leader` as primary and does not create the `lead` key. Global config verified choosing `leader`; model and permissions unchanged.

<!-- @test:FR-LEADER-02 -->
`agent_protocol_test` (14/14), `mcp_test` (12/12), and `integration_check_test` (14/14) pass. Tests cover `leader` v2 serialization, rejection of old roles and old versions, and ACK rules.

<!-- @test:FR-ACCEPT-01 -->
`cargo test -p asep --offline --locked --test acceptance_test test_accept_offline_protected_fixture -- --ignored --exact --nocapture` passes. Valid Git snapshot produces evidence with commit identity, snapshot digest and policy, and seven mandatory gates with exit 0 and no skip. Bad digest rejected before the gate; snapshot with valid SHA and digest but failing test rejected at gate `Tests` (exit 101). Neither rejection writes acceptance evidence. The test uses isolated Linux namespaces without `sudo` or `/etc` host changes. Separate probe confirms read-only snapshot, evidence storage not mounted in the candidate sandbox, and only loopback interface available. This evidence applies to the fixture, not the deployment runner host.

@rel:satisfies:FR-ACCEPT-01->FR-ACCEPT-01
@rel:verifies:FR-ACCEPT-01->FR-ACCEPT-01 "Provisioned offline fixture proves the success path and rejections without acceptance evidence"

- Merge: `FR-ACCEPT-02` deferred outside P0. Draft GitHub Actions removed before deployment. Existing branch protection unchanged; local `ACCEPTED` is not merge permission.

## 3.0.0-alpha release

<!-- @impl:FR-REL-03 -->
Release version set in `spec-engine/Cargo.toml`, `Cargo.lock`, `spec-engine/dart/pubspec.yaml`, and `CHANGELOG.md`. CLI takes version from the Rust manifest. Release branch published without changing tags or protected branches.

<!-- @test:FR-REL-03 -->
Offline build with lockfile produces `asep 3.0.0-alpha`. Publication verification: compare release branch SHA with `git ls-remote` output on both remotes. Full verification was not run for this PR.

<!-- @impl:FR-REL-04 -->
Local OpenCode install uses an absolute path to the built ASEP release binary, not `PATH` lookup.

<!-- @test:FR-REL-04 -->
Install verification by comparing SHA-256 of the built binary and installed binary, reading the version from the configured MCP command, and running MCP `initialize`. Compare global config before and after to ensure only the ASEP command path changed.

@rel:satisfies:FR-REL-03->FR-REL-03
@rel:verifies:FR-REL-03->FR-REL-03 "Metadata, CLI version, and branch SHA on both remotes verify the release candidate"
@rel:satisfies:FR-REL-04->FR-REL-04
@rel:verifies:FR-REL-04->FR-REL-04 "Binary hash, CLI version, MCP initialize, and JSON diff verify global sync"

## English release traceability (FR-REL-05)

<!-- @impl:FR-REL-05 -->
PRD/TDD/TRACEABILITY translated to English; CHANGELOG.md already English; exact old trace IDs and release commands checked against PRD/TDD; TRACEABILITY metrics re-notated for English. Draft PR text prepared in English but not published. Recreated commits and remote replacement still pending. FR-REL-05 not yet fully fulfilled.

<!-- @test:FR-REL-05 -->
graph cross-kind test 1 pass.

@rel:satisfies:FR-REL-05->FR-REL-05
@rel:verifies:FR-REL-05->FR-REL-05 "English PRD/TDD/TRACEABILITY translation, checked trace IDs and commands, re-notated metrics, and graph cross-kind test 1 pass; draft PR not published, commits and remote replacement pending"

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
