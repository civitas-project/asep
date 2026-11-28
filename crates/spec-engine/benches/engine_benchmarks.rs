use asep::affected::affected;
use asep::auditor::doc_contracts::audit_doc_contracts;
use asep::graph::builder::build_relation_graph;
use asep::scanner::lexer::{parse_tags_from_line, Priority, TagKind};
use asep::scanner::ScannedTag;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// @gov:BENCH-LEXER
fn bench_ast_tag_lexing(c: &mut Criterion) {
    // Generate 10,000 lines of realistic code with interspersed traceability tags
    let mut lines = Vec::with_capacity(10_000);
    for i in 0usize..10_000usize {
        if i % 20 == 0 {
            lines.push(format!("// @impl:FR-CORE-{i:04}"));
        } else if i % 20 == 1 {
            let prev = i.saturating_sub(1);
            lines.push(format!("// @test:FR-CORE-{prev:04}"));
        } else if i % 50 == 0 {
            lines.push("<!-- @gov:BENCH-LEXER -->".to_string());
        } else {
            lines.push(format!(
                "pub fn process_step_{i}(val: u64) -> u64 {{ val + 1 }}"
            ));
        }
    }

    c.bench_function("ast_tag_lexing_10k_lines", |b| {
        b.iter(|| {
            let mut total_tags = 0;
            for (idx, line) in lines.iter().enumerate() {
                let tags = parse_tags_from_line(black_box(line), idx + 1);
                total_tags += tags.len();
            }
            black_box(total_tags);
        });
    });
}

// @gov:BENCH-GRAPH
fn bench_traceability_graph_resolution(c: &mut Criterion) {
    // Generate 500 bipartite 1:1 relations (2,000 total tags)
    let dummy_path = PathBuf::from("src/domain.rs");
    let mut tags = Vec::with_capacity(2000);

    for i in 0..500 {
        let id = format!("FR-BENCH-{i:04}");
        tags.push(ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Req {
                id: id.clone(),
                priority: Priority::Must,
            },
            line_number: i * 4 + 1,
            raw_line: format!("<!-- @req:{id}:MUST Benchmark item -->"),
        });
        tags.push(ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Design { id: id.clone() },
            line_number: i * 4 + 2,
            raw_line: format!("@design:{id}"),
        });
        tags.push(ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Impl { id: id.clone() },
            line_number: i * 4 + 3,
            raw_line: format!("// @impl:{id}"),
        });
        tags.push(ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Test { id: id.clone() },
            line_number: i * 4 + 4,
            raw_line: format!("// @test:{id}"),
        });
    }

    c.bench_function("traceability_graph_500_relations", |b| {
        b.iter(|| {
            let report = build_relation_graph(black_box(&tags), &asep::policy::load_policy());
            black_box(report.is_closed);
        });
    });
}

// @gov:BENCH-DOC-CONTRACT
fn bench_doc_contract_parsing(c: &mut Criterion) {
    // Generate 100 modules of source code with docstring contracts
    let mut modules = Vec::with_capacity(100);
    for i in 0..100 {
        let code = format!(
            r#"
/// Computes hash validation for transaction batch {i}.
///
/// # Invariants: The batch length must not exceed maximum frame size.
/// # Errors: Returns `BatchError::InvalidHash` if digest computation fails.
pub fn validate_batch_{i}(data: &[u8]) -> Result<u64, String> {{
    if data.is_empty() {{
        Err("empty".into())
    }} else {{
        Ok(data.len() as u64)
    }}
}}
"#
        );
        modules.push(code);
    }

    let dummy_path = Path::new("src/batch.rs");

    c.bench_function("doc_contract_parsing_100_modules", |b| {
        b.iter(|| {
            let mut total_violations = 0;
            for module in &modules {
                let violations = audit_doc_contracts(dummy_path, black_box(module));
                total_violations += violations.len();
            }
            black_box(total_violations);
        });
    });
}

// @gov:BENCH-AFFECTED
/// Baseline (O01, no optimization): `asep::affected::affected()` over generated
/// graphs of size V with four relation variants:
/// - sparse: chain (satisfies + verifies + constrains, ~3V edges)
/// - dense: 8-way satisfies fanout (~8V edges)
/// - cyclic: closed satisfies/constrains cycle (~2V edges, cycle-safe BFS)
/// - same-ID: one id shared by Req/Impl/Test per file, legacy same-ID fallback (~3V edges).
///
/// Fixtures (V `.md` files with `@req/@impl/@test` tags plus one `.rel` file)
/// are built once per (size, variant) outside `b.iter()`; one changed path
/// (`nodes/0.md`) seeds the traversal. Each (size, variant) is its own
/// criterion benchmark so slow cases do not prevent measuring fast ones.
fn build_affected_fixture(root: &Path, size: usize, variant: &str) {
    let mut rels = String::new();
    for i in 0..size {
        let (req_id, impl_id, test_id) = if variant == "same_id" {
            (format!("SID-{i}"), format!("SID-{i}"), format!("SID-{i}"))
        } else {
            (format!("REQ-{i}"), format!("IMPL-{i}"), format!("TEST-{i}"))
        };
        let content = format!(
            "<!-- @req:{req_id}:MUST -->\n// @impl:{impl_id}\n// @test:{test_id}\n",
        );
        let path = root.join(format!("nodes/{i}.md"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();

        let next = (i + 1) % size;
        if variant == "dense" {
            for k in 1..=8usize {
                let j = (i + k) % size;
                rels.push_str(&format!("@rel:satisfies:IMPL-{i}->REQ-{j}\n"));
            }
        } else if i + 1 < size || variant == "cyclic" {
            rels.push_str(&format!("@rel:satisfies:IMPL-{i}->REQ-{next}\n"));
            rels.push_str(&format!("@rel:constrains:REQ-{i}->IMPL-{next}\n"));
        }
        if variant != "same_id" {
            rels.push_str(&format!("@rel:verifies:TEST-{i}->REQ-{i}\n"));
        } else {
            rels.push_str(&format!("@rel:satisfies:SID-{i}->SID-{next}\n"));
            rels.push_str(&format!("@rel:constrains:SID-{i}->SID-{i}\n"));
            rels.push_str(&format!("@rel:verifies:SID-{i}->SID-{i}\n"));
        }
    }
    std::fs::write(root.join("nodes/relations.rel"), rels).unwrap();
}

macro_rules! gen_affected_bench {
    ($name:ident, $size:expr, $variant:expr) => {
        fn $name(c: &mut Criterion) {
            let size = $size;
            let variant = $variant;
            let root = tempdir().expect("temp dir for bench fixture");
            build_affected_fixture(root.path(), size, variant);
            let changed = vec![PathBuf::from("nodes/0.md")];
            let name = format!("affected_{size}_{variant}");
            c.bench_function(name.as_str(), |b| {
                b.iter(|| {
                    match black_box(affected(root.path(), &changed)) {
                        Ok(report) => black_box(report),
                        Err(error) => panic!("affected() failed: {error}"),
                    }
                });
            });
        }
    };
}

gen_affected_bench!(bench_affected_100_sparse, 100usize, "sparse");
gen_affected_bench!(bench_affected_100_dense, 100usize, "dense");
gen_affected_bench!(bench_affected_100_cyclic, 100usize, "cyclic");
gen_affected_bench!(bench_affected_100_same_id, 100usize, "same_id");
gen_affected_bench!(bench_affected_1000_sparse, 1_000usize, "sparse");
gen_affected_bench!(bench_affected_1000_dense, 1_000usize, "dense");
gen_affected_bench!(bench_affected_1000_cyclic, 1_000usize, "cyclic");
gen_affected_bench!(bench_affected_1000_same_id, 1_000usize, "same_id");
gen_affected_bench!(bench_affected_10000_sparse, 10_000usize, "sparse");
gen_affected_bench!(bench_affected_10000_dense, 10_000usize, "dense");
gen_affected_bench!(bench_affected_10000_cyclic, 10_000usize, "cyclic");
gen_affected_bench!(bench_affected_10000_same_id, 10_000usize, "same_id");

criterion_group!(
    benches,
    bench_ast_tag_lexing,
    bench_traceability_graph_resolution,
    bench_doc_contract_parsing,
    bench_affected_100_sparse,
    bench_affected_100_dense,
    bench_affected_100_cyclic,
    bench_affected_100_same_id,
    bench_affected_1000_sparse,
    bench_affected_1000_dense,
    bench_affected_1000_cyclic,
    bench_affected_1000_same_id,
    bench_affected_10000_sparse,
    bench_affected_10000_dense,
    bench_affected_10000_cyclic,
    bench_affected_10000_same_id
);
criterion_main!(benches);
