use asep::auditor::{
    audit_anti_slop, audit_project_anti_slop, AntiSlopViolation, AntiSlopViolationKind,
};
use asep::policy::Severity;
use std::fs;
use std::path::Path;

fn audit_rs(content: &str) -> Vec<asep::auditor::AntiSlopViolation> {
    audit_anti_slop(Path::new("test.rs"), content)
}

fn audit_md(content: &str) -> Vec<asep::auditor::AntiSlopViolation> {
    audit_anti_slop(Path::new("spec.md"), content)
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

#[test]
fn test_policy_rules_detect_design_copy_and_docs_slop() {
    let violations = audit_md(
        "## Design\nTBD: decide later. This should be fast indeed.\n## Copy\nA very game-changer release.\n## Docs\nLorem ipsum draft. It is important to note that nothing is required here.\n",
    );

    let ids: Vec<&str> = violations
        .iter()
        .filter_map(|v| match &v.kind {
            AntiSlopViolationKind::Canonical { rule_id, .. } => Some(rule_id.as_str()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&"AS-DESIGN-TBD"), "missing AS-DESIGN-TBD in {ids:?}");
    assert!(ids.contains(&"AS-COPY-FILLER"), "missing AS-COPY-FILLER in {ids:?}");
    assert!(ids.contains(&"AS-COPY-HYPE"), "missing AS-COPY-HYPE in {ids:?}");
    assert!(
        ids.contains(&"AS-DOCS-PLACEHOLDER"),
        "missing AS-DOCS-PLACEHOLDER in {ids:?}"
    );
    assert!(
        ids.contains(&"AS-DESIGN-VAGUE"),
        "missing AS-DESIGN-VAGUE in {ids:?}"
    );
    assert!(
        ids.contains(&"AS-DOCS-WORDY"),
        "missing AS-DOCS-WORDY in {ids:?}"
    );

    for v in &violations {
        if matches!(&v.kind, AntiSlopViolationKind::Canonical { .. }) {
            assert_eq!(
                v.severity,
                Severity::Warning,
                "canonical violation {} should be Warning",
                v.message
            );
        }
    }
}

#[test]
fn test_box_drawing_produces_decorative_banner_error() {
    let violations = audit_rs("// ┌───┐\nfn foo() {}\n");
    let banners: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v.kind, AntiSlopViolationKind::DecorativeBanner { .. }))
        .collect();
    assert!(!banners.is_empty(), "expected DecorativeBanner for box drawing");
    assert!(
        banners.iter().all(|v| v.severity == Severity::Error),
        "box-drawing banner must be Error"
    );
}

#[test]
fn test_markdown_buzzword_uses_copy_rule() {
    let violations = audit_md("This is a seamless experience.\n");
    let buzz: Vec<_> = violations
        .iter()
        .filter(|v| matches!(v.kind, AntiSlopViolationKind::AiBuzzword { .. }))
        .collect();
    assert!(!buzz.is_empty(), "expected AiBuzzword in Markdown");
    assert!(
        buzz.iter().all(|v| v.severity == Severity::Error),
        "AS-COPY-BUZZWORD must be Error"
    );
    assert!(
        buzz.iter().any(|v| {
            if let AntiSlopViolationKind::AiBuzzword { word } = &v.kind {
                word == "seamless"
            } else {
                false
            }
        }),
        "expected buzzword 'seamless'"
    );
}

#[test]
fn test_nesting_threshold_from_policy() {
    // Depth 2 is below the policy max_nesting (3): must not be flagged.
    let violations = audit_rs(
        "fn foo(x: bool) {\n    if x {\n        if true {\n        }\n    }\n}\n",
    );
    assert!(
        violations
            .iter()
            .all(|v| !matches!(v.kind, AntiSlopViolationKind::ExcessiveNesting { .. })),
        "depth below threshold must not be flagged: {violations:?}"
    );
}

#[test]
fn test_existing_code_slop_kinds_preserved() {
    let violations = audit_rs(
        "// =====================\n// Step 1: validate input\n/// Delve into robust solutions\nfn foo() {}\n",
    );

    let has_banner = violations
        .iter()
        .any(|v| matches!(&v.kind, AntiSlopViolationKind::DecorativeBanner { .. }));
    let has_narration = violations.iter().any(|v| {
        matches!(&v.kind, AntiSlopViolationKind::WorkflowNarration { .. })
    });
    let has_buzzword = violations
        .iter()
        .any(|v| matches!(&v.kind, AntiSlopViolationKind::AiBuzzword { .. }));
    assert!(has_banner, "expected DecorativeBanner");
    assert!(has_narration, "expected WorkflowNarration");
    assert!(has_buzzword, "expected AiBuzzword");

    for v in &violations {
        assert_eq!(v.severity, Severity::Error, "existing detection {:?} should be Error", v.kind);
    }
}

fn py(content: &str) -> Vec<AntiSlopViolation> {
    audit_anti_slop(Path::new("test.py"), content)
}

#[test]
fn python_hash_comments_produce_code_domain_violations() {
    let violations = py("# =====================\n# Step 1: validate\n# delve into robust solutions\ndef foo():\n    pass\n");

    assert!(
        violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::DecorativeBanner { .. }
        )),
        "expected DecorativeBanner for '# =====================', got {violations:?}"
    );

    assert!(
        violations.iter().any(|v| matches!(
            v.kind,
            AntiSlopViolationKind::WorkflowNarration { .. }
        )),
        "expected WorkflowNarration for '# Step 1: validate', got {violations:?}"
    );

    let words: Vec<String> = violations
        .iter()
        .filter_map(|v| match &v.kind {
            AntiSlopViolationKind::AiBuzzword { word } => Some(word.clone()),
            _ => None,
        })
        .collect();
    assert!(
        words.iter().any(|w| w == "delve"),
        "expected buzzword 'delve' for '# delve into robust solutions', got {words:?}"
    );
}

#[test]
fn python_code_outside_comments_is_not_flagged() {
    let violations = py("def foo():\n    return 1\n");
    assert!(violations.is_empty(), "expected no violations, got {violations:?}");
}
