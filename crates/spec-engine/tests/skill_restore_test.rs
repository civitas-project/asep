use asep::skill::distribution::embedded::get_embedded_skills;
use asep::skill::inspector::inspect_skills;
use asep::skill::model::SkillStatus;
use asep::skill::restore::{restore_all_builtin_skills, restore_skills};
use std::fs;
use tempfile::tempdir;

// @test:FR-SKILL-RESTORE-01
#[test]
fn test_selective_and_bulk_factory_restore() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let embedded = get_embedded_skills();
    assert!(embedded.len() >= 2);

    let skill_a = &embedded[0];
    let skill_b = &embedded[1];

    // Corrupt skill_a
    let dir_a = skills_dir.join(skill_a.directory);
    fs::create_dir_all(&dir_a).unwrap();
    fs::write(dir_a.join("SKILL.md"), "CORRUPTED CONTENT").unwrap();

    // Verify initial inspection: skill_a is Modified, skill_b is Missing
    let entries_before = inspect_skills(&skills_dir);
    assert_eq!(
        entries_before
            .iter()
            .find(|e| e.id == skill_a.id)
            .unwrap()
            .status,
        SkillStatus::Modified
    );
    assert_eq!(
        entries_before
            .iter()
            .find(|e| e.id == skill_b.id)
            .unwrap()
            .status,
        SkillStatus::Missing
    );

    // 1. Selective restore: restore only skill_a
    let restored_single = restore_skills(&skills_dir, &[skill_a.id]).unwrap();
    assert_eq!(restored_single, vec![skill_a.id.to_string()]);

    let entries_after_single = inspect_skills(&skills_dir);
    assert_eq!(
        entries_after_single
            .iter()
            .find(|e| e.id == skill_a.id)
            .unwrap()
            .status,
        SkillStatus::Builtin
    );
    assert_eq!(
        entries_after_single
            .iter()
            .find(|e| e.id == skill_b.id)
            .unwrap()
            .status,
        SkillStatus::Missing
    );

    // Content of skill_a should match canonical embedded content
    let content_a = fs::read_to_string(dir_a.join("SKILL.md")).unwrap();
    assert_eq!(content_a.trim(), skill_a.content.trim());

    // 2. Bulk restore: restore all builtin skills
    let restored_all = restore_all_builtin_skills(&skills_dir).unwrap();
    assert_eq!(restored_all.len(), embedded.len());

    let entries_after_all = inspect_skills(&skills_dir);
    for em in embedded {
        let entry = entries_after_all
            .iter()
            .find(|e| e.id == em.id)
            .expect("All embedded skills must exist after bulk restore");
        assert_eq!(entry.status, SkillStatus::Builtin);
    }
}
