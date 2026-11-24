use asep::pack::targets::TargetAgent;
use asep::skill::fork::fork_skill;
use asep::skill::install::resolve_dependencies;
use asep::skill::merge::three_way_merge;
use asep::skill::search::search_skills;
use asep::skill::state::{export_bundle, get_skill_agents, import_bundle, record_agent_install};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_record_agent_install_and_get_skill_agents() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Create a dummy skill
    let skill_folder = skills_dir.join("test-skill");
    fs::create_dir_all(&skill_folder).unwrap();
    fs::write(
        skill_folder.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test skill description\nversion: 1.0.0\ndepends: []\n---\n\nBody content\n",
    )
    .unwrap();

    let agents = get_skill_agents(&skills_dir, "test-skill");
    assert!(agents.is_empty());

    record_agent_install(&skills_dir, "test-skill", &TargetAgent::OpenCode).unwrap();
    let agents = get_skill_agents(&skills_dir, "test-skill");
    assert_eq!(agents, vec!["opencode"]);

    record_agent_install(&skills_dir, "test-skill", &TargetAgent::Cursor).unwrap();
    let agents = get_skill_agents(&skills_dir, "test-skill");
    assert_eq!(agents, vec!["opencode", "cursor"]);
}

#[test]
fn test_bundle_export_and_import() {
    let tmp = tempdir().unwrap();
    let src_skills_dir = tmp.path().join("src_skills");
    let dest_skills_dir = tmp.path().join("dest_skills");
    let bundle_file = tmp.path().join("bundle.json");

    fs::create_dir_all(&src_skills_dir).unwrap();
    let skill_folder = src_skills_dir.join("bundle-skill");
    fs::create_dir_all(&skill_folder).unwrap();
    fs::write(
        skill_folder.join("SKILL.md"),
        "---\nname: bundle-skill\ndescription: Export and import test\nversion: 1.0.0\ndepends: []\n---\n\nSkill content inside bundle\n",
    )
    .unwrap();

    // Export
    let bundle = export_bundle(&src_skills_dir, &bundle_file).expect("export should succeed");
    assert_eq!(bundle.skills.len(), 1);
    assert_eq!(bundle.skills[0].id, "bundle-skill");
    assert!(bundle_file.exists());

    // Import into fresh directory
    fs::create_dir_all(&dest_skills_dir).unwrap();
    let imported = import_bundle(&dest_skills_dir, &bundle_file).expect("import should succeed");
    assert_eq!(imported.skills.len(), 1);
    assert_eq!(imported.skills[0].id, "bundle-skill");

    let imported_file = dest_skills_dir.join("bundle-skill").join("SKILL.md");
    assert!(imported_file.exists());
    let content = fs::read_to_string(imported_file).unwrap();
    assert!(content.contains("Skill content inside bundle"));
}

#[test]
fn test_skill_search() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let results = search_skills(&skills_dir, "architecture");
    // Should match embedded system-architecture
    assert!(results.iter().any(|r| r.id == "system-architecture"));

    let results = search_skills(&skills_dir, "nonexistent-query-12345");
    assert!(results.is_empty());
}

#[test]
fn test_skill_fork() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Fork an embedded skill
    let new_path = fork_skill(
        &skills_dir,
        "code-contracts",
        "custom-contracts",
        "custom-contracts",
        "Customized contracts for project",
    )
    .expect("fork should succeed");

    assert!(new_path.exists());
    let content = fs::read_to_string(new_path).unwrap();
    assert!(content.contains("name: custom-contracts"));
    assert!(content.contains("description: Customized contracts for project"));
    assert!(content.contains("version: 0.1.0"));
}

#[test]
fn test_dependency_resolution() {
    let deps = resolve_dependencies("workflow-router");
    // workflow-router depends on system-architecture, engineering-specs, code-contracts, repository-governance, design-tokens
    assert!(deps.iter().any(|i| i == "system-architecture"));
    assert!(deps.iter().any(|i| i == "code-contracts"));
    assert!(deps.iter().any(|i| i == "engineering-specs"));
}

#[test]
fn test_three_way_merge() {
    let base = "line 1\nline 2\nline 3\n";
    let local = "line 1\nline 2 local modified\nline 3\n";
    let remote = "line 1\nline 2\nline 3 remote modified\n";

    let (merged, has_conflict) = three_way_merge(base, local, remote);
    assert!(!has_conflict);
    assert!(merged.contains("local modified"));
    assert!(merged.contains("remote modified"));

    // Conflict case
    let local_conflict = "line 1\nconflict local\nline 3\n";
    let remote_conflict = "line 1\nconflict remote\nline 3\n";
    let (merged_conflict, has_conflict) = three_way_merge(base, local_conflict, remote_conflict);
    assert!(has_conflict);
    assert!(merged_conflict.contains("<<<<<<< LOCAL"));
    assert!(merged_conflict.contains("======="));
    assert!(merged_conflict.contains(">>>>>>> REMOTE"));
}
