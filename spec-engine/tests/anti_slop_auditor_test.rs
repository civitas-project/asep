use asep::auditor::{audit_anti_slop, audit_project_anti_slop, AntiSlopViolationKind};
use std::fs;
use std::path::Path;

fn audit_rs(content: &str) -> Vec<asep::auditor::AntiSlopViolation> {
    audit_anti_slop(Path::new("test.rs"), content)
}

#[test]
fn test_audit_anti_slop_all_categories() {
    // DecorativeBanner: banner comment line.
    let violations = audit_rs("// =====================\nfn foo() {}\n");
    assert!(violations.iter().any(|v| matches!(
        v.kind,
        AntiSlopViolationKind::DecorativeBanner { .. }
    )));

    // WorkflowNarration: step narration comment.
    let violations = audit_rs("// Step 1: validate input\nfn foo() {}\n");
    assert!(violations.iter().any(|v| matches!(
        v.kind,
        AntiSlopViolationKind::WorkflowNarration { .. }
    )));

    // AiBuzzword: buzzwords in a doc comment line.
    let violations = audit_rs("/// Delve into robust solutions\nfn foo() {}\n");
    let words: Vec<String> = violations
        .iter()
        .filter_map(|v| match &v.kind {
            AntiSlopViolationKind::AiBuzzword { word } => Some(word.clone()),
            _ => None,
        })
        .collect();
    assert!(words.iter().any(|w| w == "Delve"));
    assert!(words.iter().any(|w| w == "robust"));

    // ExcessiveNesting: three levels of nested if blocks.
    let violations = audit_rs(
        "fn foo(x: bool) {\n    if x {\n        if true {\n            if false {\n            }\n        }\n    }\n}\n",
    );
    assert!(violations.iter().any(|v| {
        if let AntiSlopViolationKind::ExcessiveNesting { depth } = &v.kind {
            *depth >= 3
        } else {
            false
        }
    }));
}

#[test]
fn test_audit_project_anti_slop_tempdir() {
    let dir = tempfile::tempdir().expect("create tempdir");

    let rs_path = dir.path().join("main.rs");
    fs::write(&rs_path, "// =====================\nfn main() {}\n").expect("write main.rs");

    let md_path = dir.path().join("README.md");
    fs::write(&md_path, "This is a seamless experience.\n").expect("write README.md");

    let violations = audit_project_anti_slop(dir.path()).expect("audit tempdir");

    let banner: Vec<&_> = violations
        .iter()
        .filter(|v| {
            v.file_path == Path::new("main.rs")
                && v.line_number == 1
                && matches!(
                    v.kind,
                    AntiSlopViolationKind::DecorativeBanner { .. }
                )
        })
        .collect();
    assert!(!banner.is_empty(), "expected banner violation in main.rs:1");

    let buzz: Vec<&_> = violations
        .iter()
        .filter(|v| {
            v.file_path == Path::new("README.md")
                && v.line_number == 1
                && matches!(
                    v.kind,
                    AntiSlopViolationKind::AiBuzzword { .. }
                )
        })
        .collect();
    assert!(!buzz.is_empty(), "expected buzzword violation in README.md:1");
}
