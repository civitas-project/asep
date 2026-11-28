// @impl:FR-AUDIT-01
// @test:FR-AUDIT-01

use std::fs;

#[test]
fn audit_doc_has_required_sections() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .join("docs")
        .join("AUDIT.md");
    let content = fs::read_to_string(&path).expect("docs/AUDIT.md must exist");
    for heading in [
        "## Existing mechanisms",
        "## Consumers",
        "## Overlap with native runtime",
        "## Baseline measurement plan",
    ] {
        assert!(content.contains(heading), "missing section: {heading}");
    }
}
