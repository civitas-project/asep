//! Impact analysis: compute the impls, requirements, tests, and cargo test
//! commands affected by a set of changed files.
//!
//! Tags are scanned from `root`, the traceability relation graph is built
//! from them and the canonical policy, then `changed` files are mapped to
//! `NodeKind::Impl` nodes. Outgoing `Satisfies` edges lead to requirements;
//! incoming `Verifies` edges lead to tests. Repos without `@rel` tags keep
//! working via the same-id legacy fallback.

use crate::graph::{build_relation_graph, NodeKind, TraceNode};
use crate::policy::{load_policy, EdgeKind};
use crate::scanner::scan_project_tags;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

static FN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"fn\s+([A-Za-z_][A-Za-z0-9_]*)").expect("valid fn regex"));

/// A traced implementation node (impl, design, requirement, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedNode {
    /// Traceability identifier.
    pub id: String,
    /// File containing the node, relative to the project root.
    pub file: PathBuf,
    /// 1-based line of the tag.
    pub line: usize,
}

/// A traced test node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedTest {
    /// Traceability identifier.
    pub id: String,
    /// File containing the test, relative to the project root.
    pub file: PathBuf,
    /// 1-based line of the tag.
    pub line: usize,
    /// Test function name, when it could be extracted from the source.
    pub function: Option<String>,
}

/// Report of everything impacted by a set of changed files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedReport {
    /// Implementations in changed files.
    pub impls: Vec<AffectedNode>,
    /// Requirements satisfied by the impacted impls.
    pub requirements: Vec<AffectedNode>,
    /// Tests verifying the impacted requirements.
    pub tests: Vec<AffectedTest>,
    /// `cargo test` commands to run the impacted tests, deduplicated.
    pub commands: Vec<String>,
}

/// Computes the traceability impact of changed files.
///
/// # Errors
///
/// Returns a string when the project at `root` cannot be scanned.
pub fn affected(root: &Path, changed: &[PathBuf]) -> Result<AffectedReport, String> {
    let tags = scan_project_tags(root).map_err(|e| e.to_string())?;
    let policy = load_policy();
    let graph = build_relation_graph(&tags, &policy);

    let changed: Vec<PathBuf> = changed.iter().map(|p| normalize_path(root, p)).collect();
    let is_changed = |node: &TraceNode| changed.iter().any(|p| matches_path(&node.file_path, p));

    // 3. Nodes living in changed files: impls are the usual starting point,
    //    but changed requirement and test files start tracing too.
    let impl_nodes: Vec<&TraceNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Impl && is_changed(n))
        .collect();
    let changed_req_nodes: Vec<&TraceNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Req && is_changed(n))
        .collect();
    let changed_test_nodes: Vec<&TraceNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Test && is_changed(n))
        .collect();

    let graph_req_ids: HashSet<String> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Req)
        .map(|n| n.id.clone())
        .collect();
    let graph_test_ids: HashSet<String> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Test)
        .map(|n| n.id.clone())
        .collect();

    // 4/6. Outgoing Satisfies edges -> requirements; same-id legacy fallback.
    let mut req_ids: HashSet<String> = HashSet::new();
    for impl_node in &impl_nodes {
        let mut has_satisfies = false;
        for edge in &graph.edges {
            if edge.kind != EdgeKind::Satisfies || edge.from != impl_node.id {
                continue;
            }
            if !edge.to.iter().any(|t| graph_req_ids.contains(t)) {
                continue;
            }
            has_satisfies = true;
            for target in &edge.to {
                if graph_req_ids.contains(target) {
                    req_ids.insert(target.clone());
                }
            }
        }
        if !has_satisfies && graph_req_ids.contains(&impl_node.id) {
            req_ids.insert(impl_node.id.clone());
        }
    }
    // Requirements in changed files are starting points themselves.
    for req_node in &changed_req_nodes {
        req_ids.insert(req_node.id.clone());
    }

    // 5/6. Incoming Verifies edges -> tests; same-id legacy fallback.
    let mut test_ids: HashSet<String> = HashSet::new();
    for req_id in &req_ids {
        let mut has_verifies = false;
        for edge in &graph.edges {
            if edge.kind != EdgeKind::Verifies || !edge.to.contains(req_id) {
                continue;
            }
            has_verifies = true;
            if graph_test_ids.contains(&edge.from) {
                test_ids.insert(edge.from.clone());
            }
        }
        if !has_verifies && graph_test_ids.contains(req_id) {
            test_ids.insert(req_id.clone());
        }
    }
    // Tests in changed files are included directly.
    for test_node in &changed_test_nodes {
        test_ids.insert(test_node.id.clone());
    }

    let mut impls: Vec<AffectedNode> = impl_nodes
        .iter()
        .map(|n| AffectedNode {
            id: n.id.clone(),
            file: n.file_path.clone(),
            line: n.line_number,
        })
        .collect();

    let mut requirements: Vec<AffectedNode> = req_ids
        .iter()
        .filter_map(|id| {
            graph
                .nodes
                .iter()
                .find(|n| n.kind == NodeKind::Req && n.id == *id)
                .map(|n| AffectedNode {
                    id: n.id.clone(),
                    file: n.file_path.clone(),
                    line: n.line_number,
                })
        })
        .collect();

    let mut tests: Vec<AffectedTest> = test_ids
        .iter()
        .filter_map(|id| {
            graph
                .nodes
                .iter()
                .find(|n| n.kind == NodeKind::Test && n.id == *id)
                .map(|n| AffectedTest {
                    id: n.id.clone(),
                    file: n.file_path.clone(),
                    line: n.line_number,
                    function: extract_function(root, n),
                })
        })
        .collect();

    // 8. Commands: one `cargo test --test <stem>` per unique test file, with
    // `-- <fn> --exact` per extracted function name.
    let mut file_functions: Vec<(PathBuf, Vec<String>)> = Vec::new();
    {
        let mut map: std::collections::BTreeMap<PathBuf, HashSet<String>> =
            std::collections::BTreeMap::new();
        for test in &tests {
            // Ensure every test file yields a command, even when no `fn` was
            // extracted (plain `cargo test --test <stem>`).
            let names = map.entry(test.file.clone()).or_default();
            if let Some(name) = &test.function {
                names.insert(name.clone());
            }
        }
        for (file, names) in map {
            let mut names: Vec<String> = names.into_iter().collect();
            names.sort();
            file_functions.push((file, names));
        }
    }
    let mut commands: Vec<String> = Vec::new();
    for (file, functions) in file_functions {
        let base = cargo_test_base(root, &file);
        if functions.is_empty() {
            commands.push(base);
            continue;
        }
        for name in functions {
            commands.push(format!("{base} -- {name} --exact"));
        }
    }
    commands.sort();
    commands.dedup();

    // 9. Deterministic ordering by (file, line, id).
    sort_by_file_line_id(&mut impls, |n| (&n.file, n.line, n.id.as_str()));
    sort_by_file_line_id(&mut requirements, |n| (&n.file, n.line, n.id.as_str()));
    sort_by_file_line_id(&mut tests, |n| (&n.file, n.line, n.id.as_str()));

    Ok(AffectedReport {
        impls,
        requirements,
        tests,
        commands,
    })
}

/// Extracts the first `fn <name>` on the tag line and up to the next six lines.
fn extract_function(root: &Path, node: &TraceNode) -> Option<String> {
    let path = root.join(&node.file_path);
    let content = std::fs::read_to_string(path).ok()?;
    for line in content
        .lines()
        .skip(node.line_number.saturating_sub(1))
        .take(7)
    {
        if let Some(captures) = FN_RE.captures(line) {
            return Some(captures[1].to_string());
        }
    }
    None
}

/// Makes `path` relative to `root` when it lives under `root`.
fn normalize_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// Collapses `.`/`..` components and redundant separators by walking
/// `Path::components()`. `ParentDir` pops the last pushed segment when
/// possible; everything else is copied through verbatim.
fn normalize_components(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// True when `node_file` equals `changed` or lives underneath `changed`.
/// Both paths are normalized through `Path::components()` before comparing,
/// so trailing slashes and `.` components do not matter: `"src"` and `"src/"`
/// both match `src/lib.rs`. Empty path / `.` matches everything.
/// Purely path-based: no filesystem access.
fn matches_path(node_file: &Path, changed: &Path) -> bool {
    let changed = normalize_components(changed);
    if changed.as_os_str().is_empty() || changed == Path::new(".") {
        return true;
    }
    let node_file = normalize_components(node_file);
    node_file == changed || node_file.starts_with(&changed)
}

// Returns the `cargo test` target fragment for a test file: `--test <stem>`
// when a `tests` component appears anywhere in the path, `--bin <name>` for
// `src/bin/...` sources (the `name` from a `[[bin]] entry in `Cargo.toml`,
// falling back to the `src/bin/` convention stem), `--lib` for any other
// `src/...` source, and `--test <stem>` as the fallback.
fn cargo_test_base(root: &Path, file: &Path) -> String {
    let norm = normalize_components(file);
    let stem = norm.file_stem().and_then(|s| s.to_str()).unwrap_or("test");
    let mut has_tests = false;
    let mut has_src = false;
    let mut src_followed_by_bin = false;
    let mut prev_was_src = false;
    for component in norm.components() {
        match component {
            Component::Normal(dir) => {
                if dir == "tests" {
                    has_tests = true;
                }
                if prev_was_src && dir == "bin" {
                    src_followed_by_bin = true;
                }
                prev_was_src = dir == "src";
                if dir == "src" {
                    has_src = true;
                }
            }
            _ => prev_was_src = false,
        }
    }
    if has_tests {
        return format!("cargo test --test {stem}");
    }
    if has_src && src_followed_by_bin {
        let name = bin_name_from_cargo_toml(root, &norm).unwrap_or_else(|| stem.to_string());
        return format!("cargo test --bin {name}");
    }
    if has_src {
        return "cargo test --lib".to_string();
    }
    format!("cargo test --test {stem}")
}

/// Detects a binary crate source from `[[bin]] entries in `Cargo.toml`,
/// returning the declared `name` when a bin `path` matches `file`.
fn bin_name_from_cargo_toml(root: &Path, file: &Path) -> Option<String> {
    let manifest = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&manifest).ok()?;
    let value: toml::Value = content.parse().ok()?;
    let bins = value.get("bin")?.as_array()?;
    for bin in bins {
        let path = bin.get("path")?.as_str()?;
        if normalize_components(Path::new(path)) == *file {
            return bin
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());
        }
    }
    None
}

/// Sorts by (file, line, id) for deterministic output.
fn sort_by_file_line_id<T>(vec: &mut [T], key: fn(&T) -> (&Path, usize, &str)) {
    vec.sort_by(|a, b| key(a).cmp(&key(b)));
}
