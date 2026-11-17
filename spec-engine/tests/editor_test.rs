use asep::config::settings::EditorPreference;
use asep::skill::editor::{dispatch_editor, resolve_editor_command};
use std::fs;
use tempfile::tempdir;

// @test:FR-SKILL-EDITOR-01
#[test]
fn test_resolve_editor_terminal_with_custom_env() {
    std::env::set_var("EDITOR", "custom_test_editor --flag");
    let (cmd, args) = resolve_editor_command(EditorPreference::Terminal);
    assert_eq!(cmd, "custom_test_editor");
    assert_eq!(args, vec!["--flag"]);
    std::env::remove_var("EDITOR");
}

// @test:FR-SKILL-EDITOR-02
#[test]
fn test_resolve_editor_gui() {
    let (cmd, _args) = resolve_editor_command(EditorPreference::Gui);
    assert!(
        cmd == "code" || cmd == "cursor" || cmd == "xdg-open" || cmd == "open" || cmd == "cmd",
        "Resolved GUI editor was: {}",
        cmd
    );
}

// @test:FR-SKILL-EDITOR-03
#[test]
fn test_dispatch_editor_non_interactive_fallback() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join(".spec-engine");
    fs::create_dir_all(&config_dir).unwrap();
    let file_to_edit = tmp.path().join("SKILL.md");
    fs::write(&file_to_edit, "# Test Skill").unwrap();

    // When non_interactive is true, dispatch_editor should not hang or fail
    let res = dispatch_editor(&file_to_edit, &config_dir, true);
    assert!(res.is_ok());
}
