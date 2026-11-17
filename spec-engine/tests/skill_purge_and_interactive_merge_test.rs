use asep::pack::targets::{InstallScope, TargetAgent};
use asep::skill::merge::resolve_conflict_chunks;
use asep::skill::purge::purge_skill;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_resolve_conflict_chunks_logic() {
    let conflicted_text = r#"line 1
<<<<<<< LOCAL
my local change
=======
their remote change
>>>>>>> REMOTE
line 3
"#;

    // Test resolving by taking local
    let resolved_local = resolve_conflict_chunks(conflicted_text, |local, _remote| local.to_string());
    assert_eq!(resolved_local, "line 1\nmy local change\nline 3\n");

    // Test resolving by taking remote
    let resolved_remote = resolve_conflict_chunks(conflicted_text, |_local, remote| remote.to_string());
    assert_eq!(resolved_remote, "line 1\ntheir remote change\nline 3\n");
}

#[test]
fn test_skill_purge_cascade() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let skills_dir = root.join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_folder = skills_dir.join("test-purge");
    fs::create_dir_all(&skill_folder).unwrap();
    fs::write(
        skill_folder.join("SKILL.md"),
        "---\nname: test-purge\ndescription: Purge me\nversion: 1.0.0\ndepends: []\n---\nBody\n",
    )
    .unwrap();

    // Create mock agent install in local scope
    let agent_target = root.join(".cursor").join("rules").join("test-purge");
    fs::create_dir_all(&agent_target).unwrap();
    fs::write(agent_target.join("SKILL.md"), "installed in cursor").unwrap();

    // Record in state
    asep::skill::state::record_agent_install(&skills_dir, "test-purge", &TargetAgent::Cursor).unwrap();

    let scope = InstallScope::Local(root.to_path_buf());
    let report = purge_skill(&skills_dir, "test-purge", None, &scope).expect("purge should succeed");

    assert!(report.workspace_removed);
    assert!(!skill_folder.exists());
    assert!(!agent_target.exists());

    // Verify removed from state
    let state = asep::skill::state::load_state(&skills_dir).unwrap();
    assert!(!state.skills.iter().any(|s| s.id == "test-purge"));
}
