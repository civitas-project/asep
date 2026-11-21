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
@rel:verifies:FR-ANTISLOP-05->FR-ANTISLOP-05 "audit_anti_slop flags every else token as an Error-severity violation"
@rel:verifies:FR-ANTISLOP-06->FR-ANTISLOP-06 "audit_anti_slop flags any if nested inside another if with max_nesting=0"
@rel:verifies:FR-ANTISLOP-07->FR-ANTISLOP-07 "code_rule reads AS-CODE-ELSE and AS-CODE-NESTING severity from asep-policy.toml"
@rel:verifies:FR-ANTISLOP-08->FR-ANTISLOP-08 "audited repository sources contain no else token and no nested if"
@rel:verifies:FR-ANTISLOP-09->FR-ANTISLOP-09 "asep:allow-antislop does not suppress AS-CODE-ELSE or AS-CODE-NESTING"
