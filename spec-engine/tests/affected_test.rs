use asep::affected::{affected, AffectedNode, AffectedReport, AffectedTest};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_tag_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
}

fn assert_empty(report: &AffectedReport) {
    assert!(report.impls.is_empty());
    assert!(report.requirements.is_empty());
    assert!(report.tests.is_empty());
    assert!(report.commands.is_empty());
}

#[test]
fn test_affected_graph_relations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-A:MUST\n@rel:satisfies:impl-a->FR-A\n@rel:verifies:test-a->FR-A \"note\"\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:test-a\nfn verifies_a() {}\n",
    );

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(
        report.impls,
        vec![AffectedNode {
            id: "impl-a".to_string(),
            file: PathBuf::from("src/lib.rs"),
            line: 1,
        }]
    );
    assert_eq!(
        report.requirements,
        vec![AffectedNode {
            id: "FR-A".to_string(),
            file: PathBuf::from("docs/spec.md"),
            line: 1,
        }]
    );
    assert_eq!(
        report.tests,
        vec![AffectedTest {
            id: "test-a".to_string(),
            file: PathBuf::from("tests/a_test.rs"),
            line: 1,
            function: Some("verifies_a".to_string()),
        }]
    );
    assert!(
        report
            .commands
            .iter()
            .any(|c| c.contains("cargo test --test a_test")),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_path_normalization_matches_slash_and_dot() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:IMP-A\n");

    // "src" without trailing slash matches src/lib.rs.
    let report = affected(root, &[PathBuf::from("src")]).unwrap();
    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "IMP-A");

    // "src/" with trailing slash matches the same file.
    let report = affected(root, &[PathBuf::from("src/")]).unwrap();
    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "IMP-A");

    // "." matches everything.
    let report = affected(root, &[PathBuf::from(".")]).unwrap();
    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "IMP-A");
}

#[test]
fn test_affected_test_in_src_emits_lib_command() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @test:FR-B\nfn verifies_b() {}\n");

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert!(report.impls.is_empty());
    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "FR-B");
    assert_eq!(report.tests[0].file, PathBuf::from("src/lib.rs"));
    assert_eq!(report.tests[0].function, Some("verifies_b".to_string()));
    assert!(
        report
            .commands
            .iter()
            .any(|c| c == "cargo test --lib -- verifies_b --exact"),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_changed_req_finds_verifying_test() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-C:MUST\n@rel:verifies:test-c->FR-C \"note\"\n",
    );
    write_tag_file(
        root,
        "tests/c_test.rs",
        "// @test:test-c\nfn verifies_c() {}\n",
    );

    // Changing the requirement file itself must surface its verification test.
    let report = affected(root, &[PathBuf::from("docs/spec.md")]).unwrap();

    assert!(report.impls.is_empty());
    assert_eq!(report.requirements.len(), 1);
    assert_eq!(report.requirements[0].id, "FR-C");
    assert_eq!(report.requirements[0].file, PathBuf::from("docs/spec.md"));
    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "test-c");
    assert_eq!(report.tests[0].file, PathBuf::from("tests/c_test.rs"));
    assert!(
        report
            .commands
            .iter()
            .any(|c| c == "cargo test --test c_test -- verifies_c --exact"),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_extract_function_far_after_tag() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Function is 4 lines after the tag: beyond the old 2-line window.
    write_tag_file(
        root,
        "src/lib.rs",
        "// @test:FR-D\n// line two\n// line three\n// line four\nfn verifies_d() {}\n",
    );

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "FR-D");
    assert_eq!(report.tests[0].function, Some("verifies_d".to_string()));
    assert!(
        report
            .commands
            .iter()
            .any(|c| c == "cargo test --lib -- verifies_d --exact"),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_legacy_same_id_fallback() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // No @rel lines: impl, req, and test share the same id FR-A.
    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:FR-A\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:FR-A\nfn verifies_a() {}\n",
    );

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "FR-A");
    assert_eq!(report.impls[0].file, PathBuf::from("src/lib.rs"));

    assert_eq!(report.requirements.len(), 1);
    assert_eq!(report.requirements[0].id, "FR-A");
    assert_eq!(report.requirements[0].file, PathBuf::from("docs/spec.md"));

    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "FR-A");
    assert_eq!(report.tests[0].file, PathBuf::from("tests/a_test.rs"));
    assert_eq!(report.tests[0].function, Some("verifies_a".to_string()));

    assert!(
        report
            .commands
            .iter()
            .any(|c| c.contains("cargo test --test a_test")),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_no_change_returns_empty() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");

    let report = affected(root, &[PathBuf::from("other.rs")]).unwrap();
    assert_empty(&report);
}

#[test]
fn test_affected_absolute_path_is_stripped() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");

    let report = affected(root, &[root.join("src/lib.rs")]).unwrap();
    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "impl-a");
    assert_eq!(report.impls[0].file, PathBuf::from("src/lib.rs"));
    assert_eq!(report.requirements.len(), 0);
    assert_eq!(report.tests.len(), 0);
    assert_eq!(report.commands.len(), 0);
}

#[test]
fn test_affected_test_without_fn_still_emits_command() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Legacy same-id fallback: impl/req/test all share id FR-X, no @rel edges.
    write_tag_file(root, "docs/spec.md", "@req:FR-X:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:FR-X\n");
    // @test tag with no `fn` in the tag line or the next six lines.
    write_tag_file(
        root,
        "tests/b_test.rs",
        "// @test:FR-X\n// some comment\nlet x = 1;\n",
    );

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "FR-X");
    assert_eq!(report.requirements.len(), 1);
    assert_eq!(report.requirements[0].id, "FR-X");
    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "FR-X");
    assert_eq!(report.tests[0].function, None);
    // No function extracted => plain command without `-- <fn> --exact`.
    assert!(
        report
            .commands
            .iter()
            .any(|c| c == "cargo test --test b_test"),
        "expected plain command, got {:?}",
        report.commands
    );
}

#[test]
fn test_affected_directory_prefix_matches_impls_under_it() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Legacy same-id fallback: impl/req/test share ids MOD-A / MOD-B.
    write_tag_file(root, "docs/spec.md", "@req:MOD-A:MUST\n@req:MOD-B:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:MOD-A\n");
    write_tag_file(root, "src/sub/mod.rs", "// @impl:MOD-B\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:MOD-A\nfn verifies_a() {}\n// @test:MOD-B\nfn verifies_b() {}\n",
    );

    // Passing a directory prefix must match impls under it.
    let report = affected(root, &[PathBuf::from("src/")]).unwrap();

    let mut ids: Vec<String> = report.impls.iter().map(|n| n.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["MOD-A".to_string(), "MOD-B".to_string()]);
    assert_eq!(report.requirements.len(), 2);
    assert_eq!(report.tests.len(), 2);
    assert!(
        report
            .commands
            .iter()
            .any(|c| c.contains("cargo test --test a_test")),
        "expected command, got {:?}",
        report.commands
    );
}

#[test]
fn test_affected_cycle_safety() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Cycle: impl-a satisfies req-a, req-a verifies test-a, test-a verifies req-a.
    write_tag_file(
        root,
        "docs/spec.md",
        "@req:req-a:MUST\n@rel:satisfies:impl-a->req-a\n@rel:verifies:req-a->test-a\n@rel:verifies:test-a->req-a\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:test-a\nfn verifies_a() {}\n",
    );

    // Must return Ok and terminate despite the cycle.
    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert!(report.impls.iter().any(|n| n.id == "impl-a"));
    assert!(report.requirements.iter().any(|n| n.id == "req-a"));
    assert!(report.tests.iter().any(|n| n.id == "test-a"));
}

#[test]
fn test_affected_unmapped_change() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(root, "src/missing.rs", "// no tags here\n");

    let report = affected(root, &[PathBuf::from("src/missing.rs")]).unwrap();

    assert!(
        report
            .unmapped
            .iter()
            .any(|p| p == &PathBuf::from("src/missing.rs")),
        "unmapped: {:?}",
        report.unmapped
    );
}

#[test]
fn test_affected_adr_sink_no_false_propagation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Both req-a and req-b inform adr-x; only req-a's file changes.
    write_tag_file(
        root,
        "docs/req-a.md",
        "@req:req-a:MUST\n@rel:informs:req-a->adr-x\n",
    );
    write_tag_file(
        root,
        "docs/req-b.md",
        "@req:req-b:MUST\n@rel:informs:req-b->adr-x\n",
    );
    write_tag_file(root, "docs/adr-x.md", "@adr:adr-x\n");

    let report = affected(root, &[PathBuf::from("docs/req-a.md")]).unwrap();

    assert!(report.adrs.iter().any(|n| n.id == "adr-x"));
    assert!(
        !report.requirements.iter().any(|n| n.id == "req-b"),
        "req-b must not propagate through shared adr: {:?}",
        report.requirements
    );
}

#[test]
fn test_affected_adr_start_propagates_to_dependents() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(
        root,
        "docs/req-a.md",
        "@req:req-a:MUST\n@rel:informs:req-a->adr-x\n",
    );
    write_tag_file(
        root,
        "docs/req-b.md",
        "@req:req-b:MUST\n@rel:informs:req-b->adr-x\n",
    );
    write_tag_file(root, "docs/adr-x.md", "@adr:adr-x\n");

    // Change the ADR itself: dependent reqs should be reported.
    let report = affected(root, &[PathBuf::from("docs/adr-x.md")]).unwrap();

    assert!(report.adrs.iter().any(|n| n.id == "adr-x"));
    assert!(
        report.requirements.iter().any(|n| n.id == "req-a"),
        "req-a should be reported when adr-x changes: {:?}",
        report.requirements
    );
    assert!(
        report.requirements.iter().any(|n| n.id == "req-b"),
        "req-b should be reported when adr-x changes: {:?}",
        report.requirements
    );
}

#[test]
fn test_affected_same_id_impl_and_req() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Same id "X" as Impl in one file and Req in another.
    write_tag_file(root, "src/impl_x.rs", "// @impl:X\n");
    write_tag_file(root, "docs/req_x.md", "@req:X:MUST\n");

    let report = affected(root, &[PathBuf::from("src/impl_x.rs")]).unwrap();

    assert!(report.impls.iter().any(|n| n.id == "X"));
    assert!(report.requirements.iter().any(|n| n.id == "X"));
    assert!(report.provenance.contains_key("Impl:X"));
    assert!(report.provenance.contains_key("Req:X"));
}

#[test]
fn test_affected_provenance_chain_start_hop() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:impl-x\n");
    write_tag_file(
        root,
        "docs/spec.md",
        "@req:req-x:MUST\n@rel:satisfies:impl-x->req-x\n",
    );
    write_tag_file(
        root,
        "tests/x_test.rs",
        "// @test:test-x\nfn verifies_x() {}\n@rel:verifies:test-x->req-x\n",
    );

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    let chain = report
        .provenance
        .get("Impl:impl-x")
        .expect("provenance must contain Impl:impl-x");
    assert!(!chain.is_empty());
    assert_eq!(chain[0].node, "Impl:impl-x");
    assert_eq!(chain[0].edge, None);
}

// @test:FR-AUD-07
#[test]
fn dangling_reference_is_reported_not_synthesized() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-A:MUST\n@rel:satisfies:impl-a->dangling-id\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert!(
        report
            .invalid_references
            .iter()
            .any(|s| s == "Satisfies:impl-a->dangling-id"),
        "unexpected invalid_references: {:?}",
        report.invalid_references
    );
    assert!(
        !report.impls.iter().any(|n| n.id == "dangling-id"),
        "dangling id must not be synthesized as Impl: {:?}",
        report.impls
    );
    assert!(report.impls.iter().any(|n| n.id == "impl-a"));
}

#[test]
fn same_id_kinds_traverse_only_admissible_edges() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Req "X" and Impl "X" share an ID. Satisfies X->X is admissible only for
    // (Impl, Req); Verifies X->X is inadmissible for every kind pair.
    write_tag_file(root, "src/impl_x.rs", "// @impl:X\n");
    write_tag_file(root, "docs/req_x.md", "@req:X:MUST\n");
    write_tag_file(
        root,
        "docs/rel.md",
        "@rel:satisfies:X->X\n@rel:verifies:X->X \"note\"\n",
    );

    let report = affected(root, &[PathBuf::from("src/impl_x.rs")]).unwrap();

    assert!(report.impls.iter().any(|n| n.id == "X"));
    assert!(report.requirements.iter().any(|n| n.id == "X"));
    let req_chain = report
        .provenance
        .get("Req:X")
        .expect("Req:X must be reached via an admissible edge");
    assert!(
        req_chain
            .iter()
            .any(|h| h.edge.as_deref() == Some("Satisfies:forward")),
        "chain: {:?}",
        req_chain
    );
    assert!(
        report
            .provenance
            .values()
            .flatten()
            .all(|h| h.edge.as_deref() != Some("Verifies:forward")
                && h.edge.as_deref() != Some("Verifies:reverse")),
        "inadmissible verifies edge must not appear in provenance: {:?}",
        report.provenance
    );
}

#[test]
fn test_affected_deterministic() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(
        root,
        "docs/spec.md",
        "@req:req-a:MUST\n@rel:satisfies:impl-a->req-a\n",
    );
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:test-a\nfn verifies_a() {}\n@rel:verifies:test-a->req-a\n",
    );

    let changed = vec![PathBuf::from("src/lib.rs")];
    let first = affected(root, &changed).unwrap();
    let second = affected(root, &changed).unwrap();

    assert_eq!(first, second);
}

// @test:FR-PERF-01
#[test]
fn test_affected_cycle_terminates_and_is_deterministic() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Cycle: impl-a satisfies req-a; req-a constrains impl-b; impl-b satisfies req-a.
    write_tag_file(
        root,
        "docs/spec.md",
        "@req:req-a:MUST\n@rel:satisfies:impl-a->req-a\n@rel:constrains:req-a->impl-b\n@rel:satisfies:impl-b->req-a\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n// @impl:impl-b\n");

    let changed = vec![PathBuf::from("src/lib.rs")];
    let first = affected(root, &changed).unwrap();
    let second = affected(root, &changed).unwrap();
    assert_eq!(first, second, "affected() must be deterministic on cyclic graphs");

    let mut ids: Vec<String> = first.impls.iter().map(|n| n.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["impl-a".to_string(), "impl-b".to_string()]);
    assert!(first.requirements.iter().any(|n| n.id == "req-a"));
}
