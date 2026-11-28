use asep::affected::AffectedReport;
use asep::context::{compile_context, compile_context_files, ContextBundle};
use asep::config::budget::ResourceBudget;
use serde_json::to_string as to_json;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_tag_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
}

fn spec_fixture(root: &Path) {
    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-A:MUST\n@rel:satisfies:impl-a->FR-A\n@rel:verifies:test-a->FR-A \"note\"\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(root, "tests/t.rs", "// @test:test-a\nfn verifies_a() {}\n");
}

/// Every impacted node id, sorted and deduplicated.
fn impacted_ids(report: &AffectedReport) -> Vec<String> {
    let mut ids: Vec<String> = report.impls.iter().map(|n| n.id.clone()).collect();
    ids.extend(report.requirements.iter().map(|n| n.id.clone()));
    ids.extend(report.tests.iter().map(|n| n.id.clone()));
    ids.sort();
    ids.dedup();
    ids
}

#[test]
// @test:FR-CTX-01
fn unlimited_budget_keeps_all_impacted_ids() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    spec_fixture(root);

    let bundle = compile_context(
        root,
        &[PathBuf::from("src/lib.rs")],
        ResourceBudget::default(),
    )
    .unwrap();

    assert!(!bundle.impacted.impls.is_empty());
    assert!(!bundle.impacted.requirements.is_empty());
    assert!(!bundle.impacted.tests.is_empty());
    assert!(!bundle.excerpts.is_empty());
    assert!(bundle.truncation.is_empty());
    assert!(!bundle.objective.is_empty());
    assert!(!bundle.commands.is_empty());

    let json = to_json(&bundle).unwrap();
    for id in impacted_ids(&bundle.impacted) {
        assert!(
            json.contains(&format!("\"id\":\"{id}\"")),
            "missing impacted id {id} in bundle json"
        );
    }
}

#[test]
// @test:FR-CTX-02
fn tiny_budget_truncates_explicitly() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    spec_fixture(root);

    let budget = ResourceBudget {
        max_tokens: Some(1),
        ..Default::default()
    };
    let bundle = compile_context(root, &[PathBuf::from("src/lib.rs")], budget).unwrap();

    assert!(bundle.truncation.contains("BLOCKED: budget exceeded"));
    assert!(bundle.excerpts.is_empty());

    let unlimited = compile_context(
        root,
        &[PathBuf::from("src/lib.rs")],
        ResourceBudget::default(),
    )
    .unwrap();
    assert_eq!(bundle.impacted.impls, unlimited.impacted.impls);
    assert_eq!(
        bundle.impacted.requirements,
        unlimited.impacted.requirements
    );
    assert_eq!(bundle.impacted.tests, unlimited.impacted.tests);
}

#[test]
// @test:S3-CTX-ROUNDTRIP
fn bundle_round_trip_preserves_impact_data() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    spec_fixture(root);

    let bundle = compile_context(
        root,
        &[PathBuf::from("src/lib.rs")],
        ResourceBudget::default(),
    )
    .unwrap();
    let json = to_json(&bundle).unwrap();
    let restored: ContextBundle =
        serde_json::from_str(&json).expect("bundle must deserialize from its own output");

    assert_eq!(
        impacted_ids(&bundle.impacted),
        impacted_ids(&restored.impacted),
        "round trip must preserve impacted node ids"
    );
    assert_eq!(bundle.commands, restored.commands);
}

#[test]
fn deterministic_bundle() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    spec_fixture(root);

    let paths = &[PathBuf::from("src/lib.rs")];
    let first = compile_context(root, paths, ResourceBudget::default()).unwrap();
    let second = compile_context(root, paths, ResourceBudget::default()).unwrap();

    assert_eq!(to_json(&first).unwrap(), to_json(&second).unwrap());
}

#[test]
// @test:R10
fn explicit_files_compile_without_impact_analysis() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    spec_fixture(root);
    let bundle = compile_context_files(
        root,
        &[PathBuf::from("src/lib.rs")],
        "explicit objective".to_string(),
        ResourceBudget::default(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(bundle.objective, "explicit objective");
    assert!(!bundle.excerpts.is_empty());
    assert!(bundle.impacted.impls.is_empty());
    assert!(bundle.commands.is_empty());
}

#[test]
// @test:R10
fn context_rejects_paths_outside_root() {
    let temp = TempDir::new().unwrap();
    spec_fixture(temp.path());
    let result = compile_context_files(
        temp.path(),
        &[PathBuf::from("../outside.rs")],
        "escape".to_string(),
        ResourceBudget::default(),
        Vec::new(),
    );
    assert!(result.is_err(), "path escape must be rejected");
}

#[test]
// @test:N05
fn cli_explicit_context_does_not_invoke_impact_analysis() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "fn a() {}
").unwrap();
    // No policy or git repo: affected analysis would fail here.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_asep"))
        .arg("context")
        .arg("src/a.rs")
        .arg("--explicit")
        .arg("--root")
        .arg(root)
        .arg("--json")
        .output()
        .expect("run asep context");
    assert!(out.status.success(), "explicit context failed: {}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["excerpts"].as_array().unwrap().len() >= 1);
    assert!(json["objective"].as_str().unwrap().contains("Explicit context"));
}
