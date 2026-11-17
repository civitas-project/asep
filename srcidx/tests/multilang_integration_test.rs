//! Multi-language project indexing integration test.
//! Traceability: @impl:ARCH-03

use std::path::Path;

use srcidx::index::{Symbol, SymbolIndex, SymbolKind, Visibility};
use srcidx::parser::ParserRegistry;
use srcwalk::{build_walker, WalkConfig};

fn write_files(root: &Path) {
    std::fs::create_dir_all(root.join("src")).expect("create src dir");
    std::fs::create_dir_all(root.join("frontend")).expect("create frontend dir");
    std::fs::create_dir_all(root.join("backend")).expect("create backend dir");
    std::fs::create_dir_all(root.join("services")).expect("create services dir");

    std::fs::write(
        root.join("src/main.rs"),
        r#"/// @impl:ARCH-03-RS
pub struct AppConfig {
    pub port: u16,
    pub host: String,
}

pub fn run() -> &'static str {
    "running"
}
"#,
    )
    .expect("write main.rs");

    std::fs::write(
        root.join("frontend/index.ts"),
        r#"export function createApp(name: string): string {
    return `app:${name}`;
}

export class ApiClient {
    public baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }

    public get(path: string): Response {
        return fetch(path);
    }
}
"#,
    )
    .expect("write index.ts");

    std::fs::write(
        root.join("backend/app.py"),
        r#"def public_func(value: int) -> int:
    return value + 1


def _internal(value: int) -> int:
    return value - 1
"#,
    )
    .expect("write app.py");

    std::fs::write(
        root.join("services/api.go"),
        r#"package main

func Exported() string {
    return "ok"
}

func unexported() string {
    return "ok"
}
"#,
    )
    .expect("write api.go");
}

fn index_project(root: &Path, registry: &ParserRegistry) -> SymbolIndex {
    let mut index = SymbolIndex::new(root.display().to_string());
    let mut files_processed = 0usize;
    let mut files_skipped = 0usize;

    let walker = build_walker(&WalkConfig::new(root));
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !registry.can_parse(path) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        let result = registry.parse_file(path, &content);
        if !result.errors.is_empty() && result.symbols.is_empty() {
            files_skipped += 1;
            continue;
        }
        for symbol in result.symbols {
            index.add_symbol(symbol);
        }
        files_processed += 1;
    }

    index.header.files = files_processed;
    index.header.skipped = files_skipped;
    index
}

fn find_symbol<'a>(symbols: &'a [&Symbol], name: &str) -> Option<&'a Symbol> {
    symbols
        .iter()
        .find(|symbol| symbol.name == name)
        .copied()
}

fn symbols_in<'a>(index: &'a SymbolIndex, suffix: &str) -> Vec<&'a Symbol> {
    index
        .symbols
        .iter()
        .filter(|symbol| symbol.file.ends_with(suffix))
        .collect()
}

#[test]
fn multilang_indexing_integrates_all_parsers() {
    let dir = tempfile::tempdir().expect("create temp directory");
    let root = dir.path();

    write_files(root);

    let registry = ParserRegistry::default();
    let index = index_project(root, &registry);

    // Zero corrupted files and at least one symbol indexed overall.
    assert_eq!(index.header.skipped, 0, "corrupted or skipped files");
    assert!(!index.symbols.is_empty(), "no symbols indexed");

    // Rust symbols are found and identified as Rust.
    let rust_lang = registry.with_parser(Path::new("src/main.rs"), |parser| {
        parser.language_id()
    });
    assert_eq!(rust_lang, Some("rust"));

    let rust_symbols = symbols_in(&index, "src/main.rs");
    let app_config =
        find_symbol(&rust_symbols, "AppConfig").expect("AppConfig symbol missing");
    assert_eq!(app_config.kind, SymbolKind::Struct);
    assert_eq!(app_config.visibility, Visibility::Pub);
    let run_fn = find_symbol(&rust_symbols, "run").expect("run symbol missing");
    assert_eq!(run_fn.kind, SymbolKind::Fn);
    assert_eq!(run_fn.visibility, Visibility::Pub);

    // TypeScript symbols are found with exported visibility.
    let ts_symbols = symbols_in(&index, "frontend/index.ts");
    let create_app =
        find_symbol(&ts_symbols, "createApp").expect("createApp symbol missing");
    assert_eq!(create_app.kind, SymbolKind::Fn);
    assert_eq!(create_app.visibility, Visibility::Pub);
    let api_client =
        find_symbol(&ts_symbols, "ApiClient").expect("ApiClient symbol missing");
    assert_eq!(api_client.kind, SymbolKind::Class);
    assert_eq!(api_client.visibility, Visibility::Pub);

    // Python symbols are found with convention visibility (_ prefix = Private).
    let py_symbols = symbols_in(&index, "backend/app.py");
    let public_func =
        find_symbol(&py_symbols, "public_func").expect("public_func symbol missing");
    assert_eq!(public_func.kind, SymbolKind::Fn);
    assert_eq!(public_func.visibility, Visibility::Pub);
    let internal =
        find_symbol(&py_symbols, "_internal").expect("_internal symbol missing");
    assert_eq!(internal.kind, SymbolKind::Fn);
    assert_eq!(internal.visibility, Visibility::Private);

    // Go symbols are found and verify capitalization visibility.
    let go_symbols = symbols_in(&index, "services/api.go");
    let exported =
        find_symbol(&go_symbols, "Exported").expect("Exported symbol missing");
    assert_eq!(exported.kind, SymbolKind::Fn);
    assert_eq!(exported.visibility, Visibility::Pub);
    let unexported =
        find_symbol(&go_symbols, "unexported").expect("unexported symbol missing");
    assert_eq!(unexported.kind, SymbolKind::Fn);
    assert_eq!(unexported.visibility, Visibility::Private);
}
