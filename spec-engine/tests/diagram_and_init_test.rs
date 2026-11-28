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

// @test:S2-CORE-PROTOCOL
#[test]
fn test_init_generates_core_protocol_without_mandatory_router_or_bootstrap() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    init_asep_project(root).expect("init succeeds");

    let protocol = fs::read_to_string(root.join("protocol").join("CORE.md"))
        .expect("core protocol must be written on init");
    assert_eq!(
        protocol,
        asep::pack::embedded::get_core_protocol(),
        "generated protocol must reproduce the canonical source"
    );
    assert!(protocol.contains("Activation contract"));
    assert!(protocol.contains("Mandatory constraints"));

    for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        let content = fs::read_to_string(root.join(name)).expect("read entry file");
        assert!(content.contains("protocol: protocol/CORE.md"));
        for retired in ["using-asep", "workflow-router"] {
            assert!(
                !content.contains(&format!("- {retired}")),
                "{name} must not activate retired skill {retired}"
            );
        }
        for ambient in [
            "system-architecture",
            "code-contracts",
            "engineering-specs",
            "repository-governance",
            "design-tokens",
        ] {
            assert!(content.contains(&format!("- {ambient}")));
        }
    }

    for retired in ["using-asep", "workflow-router"] {
        assert!(
            !root.join("skills").join(retired).join("SKILL.md").exists(),
            "init must not install retired skill {retired} as ambient"
        );
    }
}

// @test:S2-UPGRADE
#[test]
fn test_init_upgrade_removes_stale_block_without_deleting_existing_skills() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    let stale = "<!-- asep:start -->\nmode: ask\nskills:\n- using-asep\n- workflow-router\n<!-- asep:end -->\n";
    fs::write(root.join("AGENTS.md"), stale).expect("write stale entry file");
    let legacy_skill = root.join("skills").join("using-asep").join("SKILL.md");
    fs::create_dir_all(legacy_skill.parent().unwrap()).expect("create legacy skill dir");
    fs::write(&legacy_skill, "legacy content").expect("write legacy skill");

    init_asep_project(root).expect("init succeeds");

    let content = fs::read_to_string(root.join("AGENTS.md")).expect("read entry file");
    assert!(!content.contains("- using-asep"));
    assert!(!content.contains("- workflow-router"));
    assert!(content.contains("protocol: protocol/CORE.md"));
    assert!(legacy_skill.exists(), "upgrade must not delete existing files");
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
    assert!(
        skill_count >= 1,
        "skills/ must contain at least one SKILL.md"
    );

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
