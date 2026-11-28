use asep::skill::inspector::inspect_skills;
use asep::skill::manager::{add_skill, delete_skill, export_skill, import_skill};
use asep::skill::model::SkillStatus;
use std::fs;
use tempfile::tempdir;

// @test:FR-SKILL-CRUD-01
#[test]
fn test_add_and_delete_skill() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let created_path = add_skill(
        &skills_dir,
        "team-security-audit",
        "team-security-audit",
        "Enforces internal OWASP security audit rules",
        "# Security Audit Guidelines\n\nRun security scans before merge.",
    )
    .expect("Add skill must succeed");

    assert!(created_path.exists());
    let content = fs::read_to_string(&created_path).unwrap();
    assert!(content.contains("name: team-security-audit"));
    assert!(content.contains("description: Enforces internal OWASP security audit rules"));

    let entries = inspect_skills(&skills_dir);
    let entry = entries
        .iter()
        .find(|e| e.id == "team-security-audit")
        .unwrap();
    assert_eq!(entry.status, SkillStatus::Custom);

    // Now delete
    delete_skill(&skills_dir, "team-security-audit").expect("Delete skill must succeed");
    assert!(!created_path.exists());
}

// @test:FR-SKILL-PORT-01
#[test]
fn test_export_and_import_skill_markdown() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // 1. Add skill
    add_skill(
        &skills_dir,
        "performance-benchmark",
        "performance-benchmark",
        "Measures p99 latency thresholds",
        "# Performance Benchmark\n\nRuns cargo bench.",
    )
    .unwrap();

    // 2. Export to standalone .md file
    let export_file = tmp.path().join("benchmark-export.md");
    export_skill(&skills_dir, "performance-benchmark", &export_file).expect("Export must succeed");

    assert!(export_file.exists());
    let exported_text = fs::read_to_string(&export_file).unwrap();
    assert!(exported_text.contains("name: performance-benchmark"));
    assert!(exported_text.contains("description: Measures p99 latency thresholds"));

    // 3. Delete from original skills_dir
    delete_skill(&skills_dir, "performance-benchmark").unwrap();

    // 4. Import back into a fresh skills directory
    let target_skills_dir = tmp.path().join("target_skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    let imported_id = import_skill(&target_skills_dir, &export_file).expect("Import must succeed");
    assert_eq!(imported_id, "performance-benchmark");

    let imported_file = target_skills_dir
        .join("performance-benchmark")
        .join("SKILL.md");
    assert!(imported_file.exists());

    let imported_content = fs::read_to_string(&imported_file).unwrap();
    assert!(imported_content.contains("name: performance-benchmark"));
}
