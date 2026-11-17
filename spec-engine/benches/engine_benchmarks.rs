use asep::auditor::doc_contracts::audit_doc_contracts;
use asep::graph::builder::build_trace_graph;
use asep::scanner::lexer::{parse_tags_from_line, Priority, TagKind};
use asep::scanner::ScannedTag;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::{Path, PathBuf};

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
            lines.push(format!("pub fn process_step_{i}(val: u64) -> u64 {{ val + 1 }}"));
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
            let report = build_trace_graph(black_box(&tags));
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

criterion_group!(
    benches,
    bench_ast_tag_lexing,
    bench_traceability_graph_resolution,
    bench_doc_contract_parsing
);
criterion_main!(benches);

