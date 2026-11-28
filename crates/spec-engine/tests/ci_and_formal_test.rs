use asep::ci::{run_ci_pipeline, CiOptions};
use asep::formal::validate_formal_specs;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_formal_specs_validation_empty_dir() {
    let tmp = tempdir().unwrap();
    let results = validate_formal_specs(tmp.path());
    // In empty directory, no formal specs found, returns empty vector
    assert!(results.is_empty());
}

#[test]
fn test_formal_specs_detection() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Create mock formal specs files
    fs::write(root.join("spec.als"), "sig Node {}\n").unwrap();
    fs::write(root.join("spec.tla"), "---- MODULE spec ----\n====\n").unwrap();
    fs::write(root.join("api.yaml"), "openapi: 3.0.0\n").unwrap();
    fs::write(root.join("schema.dbml"), "Table users { id int }\n").unwrap();

    let results = validate_formal_specs(root);
    // Should detect all 4 formal spec files
    assert_eq!(results.len(), 4);
    assert!(results.iter().any(|r| r.file.ends_with("spec.als")));
    assert!(results.iter().any(|r| r.file.ends_with("spec.tla")));
    assert!(results.iter().any(|r| r.file.ends_with("api.yaml")));
    assert!(results.iter().any(|r| r.file.ends_with("schema.dbml")));
}

#[test]
fn test_ci_pipeline_runner_gate_selection() {
    use asep::ci::{run_ci_pipeline, CiOptions};

    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let opts = CiOptions {
        gate: Some(0), // Gate 0 only
        skip: Vec::new(),
        rigor: "T1".to_string(),
    };

    let report = run_ci_pipeline(root, &opts);
    assert_eq!(report.gates.len(), 1);
    assert_eq!(report.gates[0].gate_number, 0);
    assert!(report.all_passed());
}

// @test:FR-CI-GATE-01
#[test]
fn test_ci_gate_1_name_is_structural_traceability() {
    let tmp = tempdir().unwrap();
    let opts = CiOptions {
        gate: Some(1),
        skip: Vec::new(),
        rigor: "T1".to_string(),
    };

    let report = run_ci_pipeline(tmp.path(), &opts);
    assert_eq!(report.gates.len(), 1);
    assert_eq!(
        report.gates[0].name,
        "Gate 1: Structural Traceability, Contracts & Skill Drift"
    );
}

// @test:FR-CI-GATE-ANTISLOP-01
#[test]
fn test_ci_gate_1_antislop_warnings_only() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("notes.md"),
        "## Notes\nThis is very important documentation.\n",
    )
    .unwrap();

    // Warning-only anti-slop passes Gate 1 (Warning does not block without --strict).
    let opts = CiOptions {
        gate: Some(1),
        skip: Vec::new(),
        rigor: "T1".to_string(),
    };
    let report = run_ci_pipeline(root, &opts);
    assert!(
        report.gates[0].passed,
        "anti-slop warnings only should pass Gate 1: {:?}",
        report.gates[0].details
    );
}

// @test:FR-CI-GATE-ANTISLOP-02
#[test]
fn test_ci_gate_1_antislop_error_fails() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("main.rs"), "// ==========\nfn main() {}\n").unwrap();

    let opts = CiOptions {
        gate: Some(1),
        skip: Vec::new(),
        rigor: "T1".to_string(),
    };

    let report = run_ci_pipeline(root, &opts);
    assert!(
        !report.gates[0].passed,
        "anti-slop Error violation should fail Gate 1 at T1"
    );
    assert!(
        report.gates[0]
            .details
            .iter()
            .any(|d| d.contains("anti-slop")),
        "expected anti-slop error detail, got {:?}",
        report.gates[0].details
    );
}

// @test:FR-CI-GATE-02
#[test]
fn test_ci_gate_1_cov02_strict_promotion() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("spec.md"), "# Spec\n\n@req:FR-CO-02:SHOULD\n").unwrap();

    let opts = || CiOptions {
        gate: Some(1),
        skip: Vec::new(),
        rigor: "T1".to_string(),
    };

    // COV-02 is a warning: passes Gate 1 without --strict.
    std::env::remove_var("GITHUB_BASE_REF");
    let lenient = run_ci_pipeline(root, &opts());
    assert!(
        lenient.gates[0].passed,
        "COV-02-only violation should pass Gate 1 without --strict"
    );

    // CI enables --strict for PRs to dev/main: COV-02 fails Gate 1.
    std::env::set_var("GITHUB_BASE_REF", "dev");
    let strict = run_ci_pipeline(root, &opts());
    std::env::remove_var("GITHUB_BASE_REF");
    assert!(
        !strict.gates[0].passed,
        "COV-02-only violation should fail Gate 1 with --strict"
    );
}
