// @impl:FR-IMPACT-01 // @impl:FR-IMPACT-02 // @impl:FR-IMPACT-03 // @impl:FR-IMPACT-04
// @impl:FR-IMPACT-05 // @impl:FR-IMPACT-06 // @impl:FR-IMPACT-07
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

/// One hop in an impact provenance chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvenanceHop {
    /// Typed node identity, e.g. "Impl:impl-a".
    pub node: String,
    /// Edge kind and direction taken to reach this node, or `None` for a start node.
    pub edge: Option<String>,
}

/// Report of everything impacted by a set of changed files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedReport {
    /// Implementations impacted.
    pub impls: Vec<AffectedNode>,
    /// Requirements impacted.
    pub requirements: Vec<AffectedNode>,
    /// Tests impacted.
    pub tests: Vec<AffectedTest>,
    /// ADRs impacted.
    pub adrs: Vec<AffectedNode>,
    /// Changed paths that mapped to no graph node.
    #[serde(default)]
    pub unmapped: Vec<PathBuf>,
    /// Provenance chains keyed by typed node ID ("Kind:id").
    #[serde(default)]
    pub provenance: std::collections::BTreeMap<String, Vec<ProvenanceHop>>,
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

    // Nodes living in changed files (all kinds are starting points).
    let mut start_nodes: Vec<&TraceNode> = Vec::new();
    for node in &graph.nodes {
        if is_changed(node) {
            start_nodes.push(node);
        }
    }

    // Helper: typed id string.
    let typed = |kind: NodeKind, id: &str| format!("{}:{}", kind_name(kind), id);

    // Unmapped changed paths: those with no matching node.
    let mut unmapped: Vec<PathBuf> = Vec::new();
    for c in &changed {
        let has_node = graph.nodes.iter().any(|n| matches_path(&n.file_path, c));
        if !has_node && !unmapped.contains(c) {
            unmapped.push(c.clone());
        }
    }
    unmapped.sort();
    unmapped.dedup();

    // Terminal kinds: no traversal originates from them.
    let is_terminal = |kind: NodeKind| matches!(kind, NodeKind::Adr | NodeKind::Design);

    // BFS over IDs, cycle-safe.
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: std::collections::VecDeque<(String, Vec<ProvenanceHop>)> =
        std::collections::VecDeque::new();
    // Collect reached node kinds per id.
    let mut reached_ids: HashSet<String> = HashSet::new();

    for node in &start_nodes {
        let tid = typed(node.kind, &node.id);
        if visited.insert(tid.clone()) {
            let chain = vec![ProvenanceHop {
                node: tid.clone(),
                edge: None,
            }];
            queue.push_back((node.id.clone(), chain.clone()));
            // store provenance under typed key
            // (handled below when popping)
            reached_ids.insert(node.id.clone());
            // We'll record provenance when we pop; but need typed key. Store in a side map id->typed?
        }
    }

    // We need provenance keyed by typed id. Re-run with typed keys in queue.
    // Rework: queue holds (typed_id, node_id, kind, chain)
    let mut queue2: std::collections::VecDeque<(String, String, NodeKind, Vec<ProvenanceHop>)> =
        std::collections::VecDeque::new();
    let mut visited2: HashSet<String> = HashSet::new();
    let mut provenance: std::collections::BTreeMap<String, Vec<ProvenanceHop>> =
        std::collections::BTreeMap::new();

    for node in &start_nodes {
        let tid = typed(node.kind, &node.id);
        if visited2.insert(tid.clone()) {
            let chain = vec![ProvenanceHop {
                node: tid.clone(),
                edge: None,
            }];
            provenance.insert(tid.clone(), chain.clone());
            queue2.push_back((tid, node.id.clone(), node.kind, chain));
        }
    }

    // Collections of reached nodes by kind.
    let mut reached_impl: HashSet<String> = HashSet::new();
    let mut reached_req: HashSet<String> = HashSet::new();
    let mut reached_test: HashSet<String> = HashSet::new();
    let mut reached_adr: HashSet<String> = HashSet::new();

    // Seed reached sets from start nodes.
    for node in &start_nodes {
        match node.kind {
            NodeKind::Impl => {
                reached_impl.insert(node.id.clone());
            }
            NodeKind::Req => {
                reached_req.insert(node.id.clone());
            }
            NodeKind::Test => {
                reached_test.insert(node.id.clone());
            }
            NodeKind::Adr => {
                reached_adr.insert(node.id.clone());
            }
            _ => {}
        }
    }

    while let Some((_tid, id, kind, chain)) = queue2.pop_front() {
        if is_terminal(kind) {
            continue;
        }
        // outgoing edges: edge.from == id
        for edge in &graph.edges {
            if edge.from != id {
                continue;
            }
            for target in &edge.to {
                // resolve target to nodes (any kind)
                for (nk, nid) in resolve_id(&graph, target) {
                    let ttid = typed(nk, &nid);
                    if visited2.contains(&ttid) {
                        continue;
                    }
                    visited2.insert(ttid.clone());
                    let mut newchain = chain.clone();
                    newchain.push(ProvenanceHop {
                        node: ttid.clone(),
                        edge: Some(format!("{:?}:forward", edge.kind)),
                    });
                    provenance.insert(ttid.clone(), newchain.clone());
                    queue2.push_back((ttid.clone(), nid.to_string(), nk, newchain));
                    match nk {
                        NodeKind::Impl => {
                            reached_impl.insert(nid.to_string());
                        }
                        NodeKind::Req => {
                            reached_req.insert(nid.to_string());
                        }
                        NodeKind::Test => {
                            reached_test.insert(nid.to_string());
                        }
                        NodeKind::Adr => {
                            reached_adr.insert(nid.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
        // incoming edges: edge.to contains id
        for edge in &graph.edges {
            if !edge.to.contains(&id) {
                continue;
            }
            let src = &edge.from;
            // resolve src kind by finding node(s) with that id
            for (nk, nid) in resolve_id(&graph, src) {
                let ttid = typed(nk, &nid);
                if visited2.contains(&ttid) {
                    continue;
                }
                visited2.insert(ttid.clone());
                let mut newchain = chain.clone();
                newchain.push(ProvenanceHop {
                    node: ttid.clone(),
                    edge: Some(format!("{:?}:reverse", edge.kind)),
                });
                provenance.insert(ttid.clone(), newchain.clone());
                queue2.push_back((ttid.clone(), nid.to_string(), nk, newchain));
                match nk {
                    NodeKind::Impl => {
                        reached_impl.insert(nid.to_string());
                    }
                    NodeKind::Req => {
                        reached_req.insert(nid.to_string());
                    }
                    NodeKind::Test => {
                        reached_test.insert(nid.to_string());
                    }
                    NodeKind::Adr => {
                        reached_adr.insert(nid.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    // Legacy same-ID fallback (preserve).
    for node in &start_nodes {
        let has_req = graph
            .nodes
            .iter()
            .any(|n| n.kind == NodeKind::Req && n.id == node.id);
        let has_test = graph
            .nodes
            .iter()
            .any(|n| n.kind == NodeKind::Test && n.id == node.id);
        let tid_req = typed(NodeKind::Req, &node.id);
        let tid_test = typed(NodeKind::Test, &node.id);
        if node.kind == NodeKind::Impl
            && has_req
            && reached_req.insert(node.id.clone())
            && !visited2.contains(&tid_req)
        {
            visited2.insert(tid_req.clone());
            provenance.insert(
                tid_req.clone(),
                vec![ProvenanceHop {
                    node: tid_req.clone(),
                    edge: Some("legacy-same-id".to_string()),
                }],
            );
        }
        if node.kind == NodeKind::Req
            && has_test
            && reached_test.insert(node.id.clone())
            && !visited2.contains(&tid_test)
        {
            visited2.insert(tid_test.clone());
            provenance.insert(
                tid_test.clone(),
                vec![ProvenanceHop {
                    node: tid_test.clone(),
                    edge: Some("legacy-same-id".to_string()),
                }],
            );
        }
        if node.kind == NodeKind::Test
            && has_req
            && reached_req.insert(node.id.clone())
            && !visited2.contains(&tid_req)
        {
            visited2.insert(tid_req.clone());
            provenance.insert(
                tid_req.clone(),
                vec![ProvenanceHop {
                    node: tid_req.clone(),
                    edge: Some("legacy-same-id".to_string()),
                }],
            );
        }
    }

    // Legacy req -> test fallback.
    let reached_req_snapshot: Vec<String> = reached_req.iter().cloned().collect();
    for rid in &reached_req_snapshot {
        let has_test = graph
            .nodes
            .iter()
            .any(|n| n.kind == NodeKind::Test && n.id == *rid);
        let no_verifies = !graph
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Verifies && e.to.contains(rid));
        let tid = typed(NodeKind::Test, rid);
        if has_test && no_verifies && reached_test.insert(rid.clone()) && !visited2.contains(&tid) {
            visited2.insert(tid.clone());
            provenance.insert(
                tid.clone(),
                vec![ProvenanceHop {
                    node: tid.clone(),
                    edge: Some("legacy-same-id".to_string()),
                }],
            );
        }
    }

    // Build node lists.
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
    let graph_adr_ids: HashSet<String> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Adr)
        .map(|n| n.id.clone())
        .collect();

    let mut impls: Vec<AffectedNode> = reached_impl
        .iter()
        .filter_map(|id| {
            graph
                .nodes
                .iter()
                .find(|n| n.kind == NodeKind::Impl && n.id == *id)
                .map(|n| AffectedNode {
                    id: n.id.clone(),
                    file: n.file_path.clone(),
                    line: n.line_number,
                })
        })
        .collect();
    let mut requirements: Vec<AffectedNode> = reached_req
        .iter()
        .filter(|id| graph_req_ids.contains(*id))
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
    let mut tests: Vec<AffectedTest> = reached_test
        .iter()
        .filter(|id| graph_test_ids.contains(*id))
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
    let mut adrs: Vec<AffectedNode> = reached_adr
        .iter()
        .filter(|id| graph_adr_ids.contains(*id))
        .filter_map(|id| {
            graph
                .nodes
                .iter()
                .find(|n| n.kind == NodeKind::Adr && n.id == *id)
                .map(|n| AffectedNode {
                    id: n.id.clone(),
                    file: n.file_path.clone(),
                    line: n.line_number,
                })
        })
        .collect();

    // Commands (same as before).
    let mut file_functions: Vec<(PathBuf, Vec<String>)> = Vec::new();
    {
        let mut map: std::collections::BTreeMap<PathBuf, HashSet<String>> =
            std::collections::BTreeMap::new();
        for test in &tests {
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

    sort_by_file_line_id(&mut impls, |n| (&n.file, n.line, n.id.as_str()));
    sort_by_file_line_id(&mut requirements, |n| (&n.file, n.line, n.id.as_str()));
    sort_by_file_line_id(&mut tests, |n| (&n.file, n.line, n.id.as_str()));
    sort_by_file_line_id(&mut adrs, |n| (&n.file, n.line, n.id.as_str()));

    Ok(AffectedReport {
        impls,
        requirements,
        tests,
        adrs,
        unmapped,
        provenance,
        commands,
    })
}

fn resolve_id(graph: &crate::graph::GraphReport, id: &str) -> Vec<(NodeKind, String)> {
    // Return all node kinds that have this id.
    let mut out = Vec::new();
    for kind in [
        NodeKind::Impl,
        NodeKind::Req,
        NodeKind::Test,
        NodeKind::Adr,
        NodeKind::Design,
    ] {
        if graph.nodes.iter().any(|n| n.kind == kind && n.id == id) {
            out.push((kind, id.to_string()));
        }
    }
    if out.is_empty() {
        // fallback: unknown kind, treat as Impl
        out.push((NodeKind::Impl, id.to_string()));
    }
    out
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Impl => "Impl",
        NodeKind::Req => "Req",
        NodeKind::Test => "Test",
        NodeKind::Adr => "Adr",
        NodeKind::Design => "Design",
    }
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
