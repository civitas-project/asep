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
