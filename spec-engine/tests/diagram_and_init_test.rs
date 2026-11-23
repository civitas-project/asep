// @test:FR-AMB-01
use asep::diagram::render_plantuml_diagrams;
use asep::init::init_asep_project;
use std::fs;
use tempfile::tempdir;

// @test:FR-CLI-RENDER-01
#[test]
fn test_init_and_diagram_render_flow() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    // 1. Test init_asep_project
    init_asep_project(root).expect("init succeeds");
    assert!(root.join("docs").exists());
    assert!(root.join("docs/adr").exists());
    assert!(root.join("docs/diagrams").exists());
    assert!(root.join("docs/diagrams/rendered").exists());
    assert!(root.join("docs/invariants").exists());
    assert!(root.join("docs/PRD.md").exists());

    // 2. Create sample plantuml file in docs/diagrams/
    let puml_path = root.join("docs/diagrams/test_sequence.puml");
    fs::write(
        &puml_path,
        "@startuml\nactor Alice\nparticipant Bob\nAlice -> Bob: Hello\n@enduml\n",
    )
    .expect("write puml");

    // 3. Test render_plantuml_diagrams
    let count = render_plantuml_diagrams(root).expect("render succeeds");
    assert_eq!(count, 1);
}

// @test:FR-CLI-INIT-02
#[test]
fn test_init_writes_ambient_skill_files_idempotently() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    init_asep_project(root).expect("init succeeds");

    // All three entry files exist and contain the asep markers.
    for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        let path = root.join(name);
        assert!(path.exists(), "{name} must exist after init");
        let content = fs::read_to_string(&path).expect("read entry file");
        assert!(
            content.contains("<!-- asep:start -->"),
            "{name} must contain asep:start marker"
        );
        assert!(
            content.contains("<!-- asep:end -->"),
            "{name} must contain asep:end marker"
        );
    }

    // Flat skills/ directory exists with at least one SKILL.md.
    let skills_dir = root.join("skills");
    assert!(skills_dir.exists(), "skills/ must exist after init");
    let skill_count = fs::read_dir(&skills_dir)
        .expect("read skills dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("SKILL.md").exists())
        .count();
    assert!(skill_count >= 1, "skills/ must contain at least one SKILL.md");

    // Calling init twice is idempotent: no duplicate markers.
    init_asep_project(root).expect("second init succeeds");
    for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        let content = fs::read_to_string(root.join(name)).expect("read entry file");
        assert_eq!(
            content.matches("<!-- asep:start -->").count(),
            1,
            "{name} must have exactly one asep block after repeated init"
        );
    }
}
