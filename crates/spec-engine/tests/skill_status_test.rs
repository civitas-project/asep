// @test:FR-AMB-05
use asep::skill::distribution::embedded::get_embedded_skills;
use asep::skill::inspector::{detect_skill_drift, extract_skill_requires, inspect_skills};
use asep::skill::model::SkillStatus;
use std::fs;
use tempfile::tempdir;

// @test:FR-SKILL-INSPECT-01
#[test]
fn test_inspect_skills_identifies_builtin_modified_custom_and_missing() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let embedded = get_embedded_skills();
    assert!(embedded.len() >= 2);

    // 1. Put first embedded skill untouched -> Builtin
    let first = &embedded[0];
    let first_dir = skills_dir.join(first.directory);
    fs::create_dir_all(&first_dir).unwrap();
    fs::write(first_dir.join("SKILL.md"), first.content).unwrap();

    // 2. Put second embedded skill with modified text -> Modified
    let second = &embedded[1];
    let second_dir = skills_dir.join(second.directory);
    fs::create_dir_all(&second_dir).unwrap();
    fs::write(
        second_dir.join("SKILL.md"),
        format!("{}\n\n<!-- customized -->", second.content),
    )
    .unwrap();

    // 3. Put a completely new custom skill -> Custom
    let custom_dir = skills_dir.join("my-custom-skill");
    fs::create_dir_all(&custom_dir).unwrap();
    let custom_content =
        "---\nname: my-custom-skill\ndescription: Custom test skill\n---\n# Custom";
    fs::write(custom_dir.join("SKILL.md"), custom_content).unwrap();

    // 4. Inspect the directory
    let entries = inspect_skills(&skills_dir);

    // Verify first skill is Builtin
    let first_entry = entries
        .iter()
        .find(|e| e.id == first.id)
        .expect("First skill must exist");
    assert_eq!(first_entry.status, SkillStatus::Builtin);

    // Verify second skill is Modified
    let second_entry = entries
        .iter()
        .find(|e| e.id == second.id)
        .expect("Second skill must exist");
    assert_eq!(second_entry.status, SkillStatus::Modified);

    // Verify custom skill is Custom
    let custom_entry = entries
        .iter()
        .find(|e| e.id == "my-custom-skill")
        .expect("Custom skill must exist");
    assert_eq!(custom_entry.status, SkillStatus::Custom);
    assert_eq!(custom_entry.name, "my-custom-skill");
    assert_eq!(custom_entry.description, "Custom test skill");

    // Verify any other embedded skill that was not written has status Missing
    let missing_embedded = &embedded[2];
    let missing_entry = entries
        .iter()
        .find(|e| e.id == missing_embedded.id)
        .expect("Missing skill should be reported");
    assert_eq!(missing_entry.status, SkillStatus::Missing);
}

// @test:FR-SKILL-DRIFT-01
#[test]
fn test_skill_copy_drift_detected() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let embedded = get_embedded_skills();
    assert!(!embedded.is_empty());

    // Copy one embedded skill with a drifted line appended.
    let target = &embedded[0];
    let target_dir = skills_dir.join(target.directory);
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(
        target_dir.join("SKILL.md"),
        format!("{}\n\n<!-- drift probe -->", target.content),
    )
    .unwrap();

    let drifts = detect_skill_drift(&skills_dir);

    // Exactly one drift: the modified installed copy.
    assert_eq!(
        drifts.len(),
        1,
        "expected exactly one drift, got {:?}",
        drifts
    );
    let drift = &drifts[0];
    assert_eq!(drift.skill_id, target.id);
    assert_eq!(drift.path, target_dir.join("SKILL.md"));
    assert_eq!(drift.kind, "modified");
}

// @test:FR-SKILL-REQUIRES-01
#[test]
fn test_extract_skill_requires_inline_list() {
    let content = "---\nname: test\nrequires: [a, b]\n---\n# body";
    assert_eq!(
        extract_skill_requires(content),
        vec!["a".to_string(), "b".to_string()]
    );
}

// @test:FR-SKILL-REQUIRES-01
#[test]
fn test_extract_skill_requires_dash_list() {
    let content = "---\nname: test\nrequires:\n- a\n- b\n---\n# body";
    assert_eq!(
        extract_skill_requires(content),
        vec!["a".to_string(), "b".to_string()]
    );
}

// @test:FR-SKILL-REQUIRES-01
#[test]
fn test_extract_skill_requires_absent() {
    let content = "---\nname: test\n---\n# body";
    assert_eq!(extract_skill_requires(content), Vec::<String>::new());
}
