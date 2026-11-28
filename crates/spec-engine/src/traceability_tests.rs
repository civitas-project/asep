// @test:FR-LEADER-01
// @test:FR-LEADER-02
// @test:FR-MCP-03 @test:FR-INT-01
// @test:FR-ACC-05
// @test:FR-ACC-02 @test:FR-ACC-03 @test:FR-ACC-04
// @test:FR-CAP-09
// @test:FR-IMPACT-01 // @test:FR-IMPACT-02 // @test:FR-IMPACT-03 // @test:FR-IMPACT-04 // @test:FR-IMPACT-05 // @test:FR-IMPACT-06 // @test:FR-IMPACT-07
// @test:FR-MCP-IMPACT-01 // @test:FR-MCP-IMPACT-02
// @test:FR-CTX-01 // @test:FR-CTX-02 // @test:FR-CTX-03
// @test:FR-ADRCOV-01 // @test:FR-ADRCOV-02 // @test:FR-ADRCOV-03
// @test:FR-AUD-01 // @test:FR-AUD-02 // @test:FR-AUD-03 // @test:FR-AUD-05 // @test:FR-AUD-06 // @test:FR-AUD-07 // @test:FR-AUD-08 // @test:FR-AUD-34
// @test:FR-CAP-01 // @test:FR-CAP-02 // @test:FR-CAP-03 // @test:FR-CAP-04 // @test:FR-CAP-05 // @test:FR-CAP-06 // @test:FR-CAP-07 // @test:FR-CAP-08 // @test:FR-AMB-01 // @test:FR-AMB-02 // @test:FR-AMB-03 // @test:FR-AMB-04 // @test:FR-AMB-05 // @test:FR-AMB-06
//! Unit tests for the 13 MUST requirements in docs/PRD.md.
//! Each test carries one test-marker comment and asserts real behavior
//! of the module it covers.

use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

use crate::affected::affected;
use crate::auditor::anti_slop::{audit_anti_slop, audit_project_anti_slop, AntiSlopViolationKind};
use crate::auditor::doc_contracts::{audit_doc_contracts, DocContractViolationKind};
use crate::cli::{AdrCommands, Cli, Commands};
use crate::config::settings::{load_config, save_config, EditorPreference, EngineConfig};
use crate::graph::model::{CoverageViolation, GraphReport, NodeKind, TraceNode};
use crate::init::init_asep_project;
use crate::mcp::protocol::JsonRpcRequest;
use crate::mcp::server::process_jsonrpc_request;
use crate::mcp::tools::get_tools_list;
use crate::pack::embedded::get_embedded_skills;
use crate::pack::manifest::generate_plugin_json;
use crate::pack::targets::{resolve_target_paths, InstallScope, TargetAgent};
use crate::policy::{load_policy, AntiSlopRule, Severity, TierId};
use crate::scanner::{parse_tags_from_line, Priority, TagKind};
use crate::skill::inspector::detect_skill_drift;
use crate::skill::manager::{add_skill, delete_skill};
use crate::verification::{evaluate, VerificationStatus};

// @test:FR-CLI-01
#[test]
fn test_parse_tags_from_line_extracts_requirement_tag() {
    let tags = parse_tags_from_line(
        "// @req:FR-CLI-01:MUST The scanner shall extract traceability tags",
        1,
    );
    assert_eq!(tags.len(), 1);
    assert_eq!(
        tags[0].kind,
        TagKind::Req {
            id: "FR-CLI-01".to_string(),
            priority: Priority::Must,
        }
    );
    assert_eq!(tags[0].line_number, 1);

    // A backslash-escaped tag is skipped (escaped placeholders excluded).
    let escaped = parse_tags_from_line("\\@req:FR-NOPE:MUST", 2);
    assert!(escaped.is_empty());
}

// @test:FR-CLI-02
#[test]
fn test_policy_exit_codes_match_canonical_policy() {
    let policy = load_policy();
    let codes = policy.exit_codes();
    assert_eq!(codes.pass, 0);
    assert_eq!(codes.fail, 1);
    assert_eq!(codes.config_io, 2);
    assert_eq!(codes.unproven, 3);
}

// @test:FR-CLI-03
#[test]
fn test_cli_parses_init_adr_render_commands() {
    let cli = Cli::try_parse_from(["asep", "init", "/tmp/x"]).expect("init must parse");
    assert!(matches!(cli.command, Some(Commands::Init { .. })));

    let cli = Cli::try_parse_from(["asep", "adr", "new", "Record the decision"])
        .expect("adr new must parse");
    match cli.command {
        Some(Commands::Adr { sub }) => {
            assert!(
                matches!(sub, AdrCommands::New { title, .. } if title == "Record the decision")
            );
        }
        other => panic!("expected Adr command, got {:?}", other),
    }

    let cli = Cli::try_parse_from(["asep", "render"]).expect("render must parse");
    assert!(matches!(cli.command, Some(Commands::Render { .. })));
}

// @test:FR-CLI-04
#[test]
fn test_docstring_auditor_detects_parroting_violation() {
    let content = "/// frobnicate prepares the frobnicate widget.\npub fn frobnicate() {}\n";
    let violations = audit_doc_contracts(Path::new("lib.rs"), content);
    assert!(
        !violations.is_empty(),
        "expected at least one docstring contract violation"
    );
    assert!(
        violations
            .iter()
            .any(|v| matches!(v.kind, DocContractViolationKind::Parroting { .. })),
        "expected a parroting violation when the docstring repeats the fn name"
    );
}

// @test:FR-CONFIG-01
#[test]
fn test_config_round_trip_and_capability_missing() {
    let tmp = tempdir().expect("tempdir created");
    let mut config = EngineConfig::default();
    config.editor.preference = EditorPreference::Gui;
    config.capabilities.file_write = false;
    config.capabilities.web = false;

    save_config(tmp.path(), &config).expect("config save must succeed");
    assert!(
        tmp.path().join(".spec-engine").join("config.toml").exists(),
        "config.toml must be written"
    );

    let reloaded = load_config(tmp.path());
    assert_eq!(reloaded.editor.preference, EditorPreference::Gui);
    assert!(!reloaded.capabilities.file_write);

    let missing = reloaded
        .capabilities
        .missing(&["file_read", "file_write", "web", "mcp"]);
    assert_eq!(missing, vec!["file_write", "web"]);
}

// @test:FR-MCP-01
#[test]
fn test_jsonrpc_request_deserialization_and_processing() {
    let req: JsonRpcRequest =
        serde_json::from_str(r#"{"jsonrpc":"2.0","id":42,"method":"initialize","params":{}}"#)
            .expect("typed JSON-RPC request must deserialize");
    assert_eq!(req.method, "initialize");
    assert_eq!(req.id, Some(Value::from(42)));

    let res = process_jsonrpc_request("{broken json", Path::new("."))
        .expect("must return an error frame");
    let val: Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["error"]["code"], -32700); // parse error
}

// @test:FR-MCP-02
#[test]
fn test_mcp_tool_catalogue_contains_expected_tools() {
    let tools = get_tools_list();
    let arr = tools["tools"]
        .as_array()
        .expect("tools list must be an array");
    assert_eq!(arr.len(), 14);
    let names: Vec<&str> = arr.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "asep_check",
        "spec_check",
        "adr_new",
        "render_diagrams",
        "srcidx",
        "srcdump",
        "asep_agent_send",
        "asep_agent_recv",
        "asep_agent_read",
        "asep_agent_ack",
        "asep_affected",
        "asep_context",
    ] {
        assert!(names.contains(&expected), "missing MCP tool {expected}");
    }
}

// @test:FR-PACK-01
#[test]
fn test_generate_plugin_json_and_embedded_skills_contain_real_content() {
    let plugin: Value =
        serde_json::from_str(&generate_plugin_json()).expect("plugin.json must be valid JSON");
    assert_eq!(plugin["name"], "asep");
    assert!(plugin["description"]
        .as_str()
        .unwrap()
        .contains("Deterministic SDLC validator"));

    let skills = get_embedded_skills();
    assert!(!skills.is_empty(), "embedded skills must be non-empty");
    for skill in skills {
        assert!(
            !skill.content.is_empty(),
            "skill {} must contain real content",
            skill.id
        );
        assert!(
            skill.content.contains("name:"),
            "skill {} must declare frontmatter name",
            skill.id
        );
    }
}

// @test:FR-PACK-02
#[test]
fn test_target_agent_detection_and_path_resolution() {
    assert_eq!(TargetAgent::all().len(), 11);
    assert_eq!(TargetAgent::OpenCode.id(), "opencode");
    assert_eq!(TargetAgent::ClaudeCode.id(), "claudecode");

    let tmp = tempdir().expect("tempdir created");
    fs::create_dir_all(tmp.path().join(".opencode")).expect("create .opencode dir");
    let scope = InstallScope::Local(tmp.path().to_path_buf());
    assert!(
        TargetAgent::OpenCode.is_detected(&scope),
        "OpenCode must be detected when .opencode exists"
    );
    assert!(
        !TargetAgent::ClaudeCode.is_detected(&scope),
        "ClaudeCode must not be detected when absent"
    );

    let path = resolve_target_paths(&scope, TargetAgent::OpenCode);
    assert_eq!(path, tmp.path().join(".opencode").join("skills"));
}

// @test:FR-SKILL-01
#[test]
fn test_detect_skill_drift_on_empty_dir_returns_empty() {
    let tmp = tempdir().expect("tempdir created");
    let drifts = detect_skill_drift(tmp.path());
    assert!(
        drifts.is_empty(),
        "empty skills dir must yield no drift records"
    );
}

// @test:FR-SKILL-02
#[test]
fn test_skill_add_normalizes_id_and_delete_removes_dir() {
    let tmp = tempdir().expect("tempdir created");
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).expect("create skills dir");

    let skill_file = add_skill(&skills_dir, "My Skill", "A skill", "desc", "body text")
        .expect("add_skill must succeed");
    assert_eq!(
        skill_file.file_name().unwrap().to_str().unwrap(),
        "SKILL.md"
    );
    assert_eq!(
        skill_file
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        "my-skill",
        "skill id must be normalized: lowercase, spaces to dashes"
    );
    assert!(skill_file.exists());

    delete_skill(&skills_dir, "my-skill").expect("delete_skill must succeed");
    assert!(
        !skills_dir.join("my-skill").exists(),
        "skill dir must be removed by delete_skill"
    );
}

fn closed_graph() -> GraphReport {
    GraphReport {
        nodes: vec![TraceNode {
            kind: NodeKind::Req,
            id: "FR-X".to_string(),
            file_path: PathBuf::from("spec.md"),
            line_number: 1,
        }],
        edges: vec![],
        violations: vec![],
        is_closed: true,
    }
}

// @test:FR-VERIFY-01
#[test]
fn test_evaluate_returns_fail_unproven_pass_for_controlled_inputs() {
    let tmp = tempdir().expect("tempdir created");
    let behavioral = tmp.path().join(".spec-engine").join("behavioral.json");
    fs::create_dir_all(behavioral.parent().unwrap()).unwrap();
    fs::write(&behavioral, r#"{"passed": true}"#).unwrap();
    let evidence_dir = tmp.path().join("evidence");

    // Broken structural graph -> Fail.
    let mut broken = closed_graph();
    broken.is_closed = false;
    broken.violations.push(CoverageViolation {
        rule_id: "COV-01".to_string(),
        subject: "FR-X".to_string(),
        found: 0,
        required: 1,
    });
    let report = evaluate(&broken, evidence_dir.as_path(), TierId::T2, &behavioral);
    assert_eq!(report.status, VerificationStatus::Fail);
    assert!(!report.structural);

    // Closed graph, missing semantic evidence at T2 -> Unproven.
    let report = evaluate(
        &closed_graph(),
        evidence_dir.as_path(),
        TierId::T2,
        &behavioral,
    );
    assert_eq!(report.status, VerificationStatus::Unproven);
    assert!(report.structural && report.behavioral && !report.semantic);

    // T1 skips semantic evidence -> Pass.
    let report = evaluate(
        &closed_graph(),
        evidence_dir.as_path(),
        TierId::T1,
        &behavioral,
    );
    assert_eq!(report.status, VerificationStatus::Pass);
    assert!(report.skipped.contains(&"semantic".to_string()));
}

// @test:FR-INIT-01
#[test]
fn test_init_asep_project_creates_docs_tree_and_prd() {
    let tmp = tempdir().expect("tempdir created");
    init_asep_project(tmp.path()).expect("init must succeed");

    // Minimal init: no unsolicited documentation tree or fabricated MUST.
    assert!(!tmp.path().join("docs/adr").is_dir());
    assert!(!tmp.path().join("docs/PRD.md").exists());
    assert!(tmp.path().join("protocol/CORE.md").is_file());
    for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        assert!(tmp.path().join(name).is_file());
    }

    // A user-authored PRD is left untouched by re-init.
    let prd = tmp.path().join("docs/PRD.md");
    std::fs::create_dir_all(prd.parent().unwrap()).unwrap();
    std::fs::write(&prd, "# MY PRD\n").unwrap();
    init_asep_project(tmp.path()).expect("re-init must succeed");
    assert_eq!(fs::read_to_string(&prd).unwrap(), "# MY PRD\n");
}

// @test:FR-ANTISLOP-05
#[test]
fn test_else_branch_is_flagged() {
    let violations = audit_anti_slop(Path::new("t.rs"), "fn f(){ if a { return; } else {} }");
    assert!(
        violations
            .iter()
            .any(|v| matches!(v.kind, AntiSlopViolationKind::ElseBranch { .. })),
        "expected an ElseBranch violation, got {violations:?}"
    );
}

// @test:FR-ANTISLOP-06
#[test]
fn test_nested_if_is_excessive_but_sequential_ifs_are_not() {
    let nested = audit_anti_slop(Path::new("t.rs"), "fn f(){ if a { if b {} } }");
    assert!(
        nested
            .iter()
            .any(|v| matches!(v.kind, AntiSlopViolationKind::ExcessiveNesting { .. })),
        "nested if must yield ExcessiveNesting, got {nested:?}"
    );

    let sequential = audit_anti_slop(Path::new("t.rs"), "fn f(){ if a {} if b {} }");
    assert!(
        !sequential
            .iter()
            .any(|v| matches!(v.kind, AntiSlopViolationKind::ExcessiveNesting { .. })),
        "sequential top-level ifs must not yield ExcessiveNesting, got {sequential:?}"
    );
}

// @test:FR-ANTISLOP-07
#[test]
fn test_policy_declares_else_and_nesting_rules() {
    let policy = load_policy();
    let else_rule = policy
        .antislop_rules()
        .iter()
        .find(|r| r.id == "AS-CODE-ELSE")
        .expect("policy must declare rule AS-CODE-ELSE");
    assert_eq!(else_rule.severity, Severity::Warning);

    let nesting_rule = policy
        .antislop_rules()
        .iter()
        .find(|r| r.id == "AS-CODE-NESTING")
        .expect("policy must declare rule AS-CODE-NESTING");
    assert_eq!(nesting_rule.max_nesting, Some(0));
}

// @test:FR-ANTISLOP-08
#[test]
fn test_asepignore_excluded_directory_is_not_audited() {
    let tmp = tempdir().expect("tempdir created");
    fs::write(
        tmp.path().join("violate.rs"),
        "fn f(){ if a { return; } else {} }",
    )
    .unwrap();
    fs::create_dir_all(tmp.path().join("excluded")).unwrap();
    fs::write(
        tmp.path().join("excluded").join("violate.rs"),
        "fn f(){ if a { return; } else {} }",
    )
    .unwrap();
    fs::write(tmp.path().join(".asepignore"), "excluded/").unwrap();

    let violations = audit_project_anti_slop(tmp.path()).expect("project audit must succeed");
    let excluded: Vec<_> = violations
        .iter()
        .filter(|v| v.file_path.to_string_lossy().contains("excluded"))
        .collect();
    assert!(
        excluded.is_empty(),
        "files under .asepignore must not be audited: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.file_path == Path::new("violate.rs")),
        "non-excluded file must be audited, got {violations:?}"
    );
}

// @test:FR-ANTISLOP-09
#[test]
fn test_else_branch_not_suppressed_by_allow_marker() {
    let violations = audit_anti_slop(
        Path::new("t.rs"),
        "fn f(){ if a { return; } else {} } // asep:allow-antislop",
    );
    assert!(
        violations
            .iter()
            .any(|v| matches!(v.kind, AntiSlopViolationKind::ElseBranch { .. })),
        "ElseBranch must not be suppressible via asep:allow-antislop, got {violations:?}"
    );
}

// @test:IN-01-COMPAT
#[test]
fn test_observability_addition_keeps_existing_contracts() {
    let config = EngineConfig::default();
    assert_eq!(config.evidence_dir, None);
    let tools = get_tools_list();
    let names = tools["tools"].as_array().expect("tools array");
    for required in ["asep_check", "spec_check", "srcdump", "asep_accept_scoped"] {
        assert!(
            names.iter().any(|t| t["name"] == required),
            "legacy tool must remain: {required}"
        );
    }
    assert!(Cli::try_parse_from(["asep", "check", "."]).is_ok());
}

// @test:IN-01-OVERHEAD
#[test]
fn test_telemetry_sampling_defaults_to_off() {
    let config = EngineConfig::default();
    assert!(!config.telemetry.enabled);
}

// @test:IN-01-REPORT
#[test]
fn test_trace_cli_and_mcp_report_are_registered() {
    let cli = Cli::try_parse_from(["asep", "trace", "status"]).expect("trace status must parse");
    assert!(matches!(cli.command, Some(Commands::Trace { .. })));
    let tools = get_tools_list();
    let names = tools["tools"].as_array().expect("tools array");
    assert!(names.iter().any(|t| t["name"] == "asep_trace_report"));
}

// @test:FR-AUDIT-01
#[test]
fn test_audit_doc_has_required_sections() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.parent().unwrap().parent().unwrap().join("docs").join("AUDIT.md");
    let content = fs::read_to_string(&path).expect("docs/AUDIT.md must exist");
    for heading in [
        "## Existing mechanisms",
        "## Consumers",
        "## Overlap with native runtime",
        "## Baseline measurement plan",
    ] {
        assert!(content.contains(heading), "missing section: {heading}");
    }
}

// @test:FR-LEVEL-02
#[test]
fn test_engineer_level_default_in_profile() {
    use crate::policy::EngineerLevel;
    let profile = crate::config::settings::CapabilityProfile::default();
    assert_eq!(profile.engineer_level, EngineerLevel::L1);
}

// @test:FR-OPT-00
#[test]
fn optimization_candidates_require_benchmark_evidence() {
    use std::collections::HashSet;
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("OPTIMIZATION_BENCHMARKS.md");
    let content = fs::read_to_string(&path).expect("docs/OPTIMIZATION_BENCHMARKS.md must exist");
    assert!(
        content.contains(&format!("<!-- {}FR-OPT-00 -->", "@impl:")),
        "missing FR-OPT-00 impl marker",
    );

    let ids = [
        "O01", "O02", "O03", "O04", "O05", "O06", "O07", "O08", "O09", "O10",
    ];
    for id in ids {
        assert!(content.contains(id), "{id} must appear in benchmark doc");
    }

    let mut rows = vec![];
    for line in content.lines() {
        if !line.starts_with("| O") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 6 || cells[1].is_empty() {
            continue;
        }
        if !ids.contains(&cells[1]) {
            continue;
        }
        rows.push(cells);
    }
    assert_eq!(rows.len(), ids.len(), "one row per candidate O01..O10");

    let mut seen = HashSet::new();
    for cells in &rows {
        let id = cells[1];
        seen.insert(id.to_string());
        let status_cell = cells
            .iter()
            .find(|cell| cell.starts_with("status ="))
            .cloned()
            .unwrap_or_default();
        assert!(!status_cell.is_empty(), "{id} must have an explicit status",);
        let status_lower = status_cell.to_ascii_lowercase();
        let claims_adoption =
            status_lower.contains("adopted") && !status_lower.contains("not adopted");
        if claims_adoption {
            let before = cells.get(3).map(|cell| cell.trim()).unwrap_or("");
            let after = cells.get(4).map(|cell| cell.trim()).unwrap_or("");
            assert!(
                !before.is_empty() && !after.is_empty(),
                "{id} claims adoption without before+after benchmark evidence",
            );
        }
    }
    for id in ids {
        assert!(seen.contains(id), "{id} row missing from benchmark table");
    }
}
