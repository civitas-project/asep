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
        mode: "velocity".to_string(),
    };

    let report = run_ci_pipeline(root, &opts);
    assert_eq!(report.gates.len(), 1);
    assert_eq!(report.gates[0].gate_number, 0);
    assert!(report.all_passed());
}
