//! Unit tests for the 13 MUST requirements in docs/PRD.md.
//! Each test carries one test-marker comment and asserts real behavior
//! of the module it covers.

use clap::Parser;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

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
use crate::policy::{load_policy, TierId};
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

    let cli =
        Cli::try_parse_from(["asep", "adr", "new", "Record the decision"]).expect("adr new must parse");
    match cli.command {
        Some(Commands::Adr { sub }) => {
            assert!(matches!(sub, AdrCommands::New { title, .. } if title == "Record the decision"));
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
    assert!(!violations.is_empty(), "expected at least one docstring contract violation");
    assert!(
        violations.iter().any(|v| matches!(v.kind, DocContractViolationKind::Parroting { .. })),
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
    let req: JsonRpcRequest = serde_json::from_str(
        r#"{"jsonrpc":"2.0","id":42,"method":"initialize","params":{}}"#,
    )
    .expect("typed JSON-RPC request must deserialize");
    assert_eq!(req.method, "initialize");
    assert_eq!(req.id, Some(Value::from(42)));

    let res =
        process_jsonrpc_request("{broken json", Path::new(".")).expect("must return an error frame");
    let val: Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["error"]["code"], -32700); // parse error
}

// @test:FR-MCP-02
#[test]
fn test_mcp_tool_catalogue_contains_expected_tools() {
    let tools = get_tools_list();
    let arr = tools["tools"].as_array().expect("tools list must be an array");
    assert_eq!(arr.len(), 6);
    let names: Vec<&str> = arr.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "asep_check",
        "spec_check",
        "adr_new",
        "render_diagrams",
        "srcidx",
        "srcdump",
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
    assert!(
        plugin["description"].as_str().unwrap().contains("Deterministic SDLC validator")
    );

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
    assert!(drifts.is_empty(), "empty skills dir must yield no drift records");
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
        skill_file.parent().unwrap().file_name().unwrap().to_str().unwrap(),
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
    let report = evaluate(&closed_graph(), evidence_dir.as_path(), TierId::T2, &behavioral);
    assert_eq!(report.status, VerificationStatus::Unproven);
    assert!(report.structural && report.behavioral && !report.semantic);

    // T1 skips semantic evidence -> Pass.
    let report = evaluate(&closed_graph(), evidence_dir.as_path(), TierId::T1, &behavioral);
    assert_eq!(report.status, VerificationStatus::Pass);
    assert!(report.skipped.contains(&"semantic".to_string()));
}

// @test:FR-INIT-01
#[test]
fn test_init_asep_project_creates_docs_tree_and_prd() {
    let tmp = tempdir().expect("tempdir created");
    init_asep_project(tmp.path()).expect("init must succeed");

    assert!(tmp.path().join("docs/adr").is_dir());
    assert!(tmp.path().join("docs/diagrams/rendered").is_dir());
    assert!(tmp.path().join("docs/invariants").is_dir());

    let prd = tmp.path().join("docs/PRD.md");
    assert!(prd.is_file());
    let content = fs::read_to_string(&prd).unwrap();
    assert!(content.contains("<!-- @req:FR-INIT-01:MUST Baseline project specification -->"));

    // Re-init must not overwrite an existing PRD.
    let first = fs::read_to_string(&prd).unwrap();
    init_asep_project(tmp.path()).expect("re-init must succeed");
    let second = fs::read_to_string(&prd).unwrap();
    assert_eq!(first, second);
}
