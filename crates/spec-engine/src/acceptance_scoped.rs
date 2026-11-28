//! Scoped acceptance runner (FR-ACC-02/03/04).
//!
//! Runs a small, always-sandboxed subset of gates against a single commit using
//! a user-owned policy at `<root>/.spec-engine/acceptance.json`, then writes
//! semantic evidence for every `@req:*:MUST` requirement in `<root>/docs/PRD.md`.
// @impl:FR-ACC-02

use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ACCEPTANCE_POLICY: &str = ".spec-engine/acceptance.json";
const BWRAP: &str = "bwrap";

/// Path the current exe is bound to inside the sandbox; gate commands must
/// invoke this in-sandbox path since the host exe path is invisible inside
/// bwrap.
const SANDBOX_EXE: &str = "/asep/bin/asep";

fn bwrap_bin() -> String {
    std::env::var("ASEP_BWRAP").unwrap_or_else(|_| BWRAP.to_string())
}

/// Policy file: `<root>/.spec-engine/acceptance.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptancePolicy {
    approved_commit: String,
    changed_paths: Vec<String>,
    evidence_dir: String,
}

/// Immutable snapshot identity: commit must equal HEAD and tree must be clean.
#[derive(Debug)]
pub struct SnapshotIdentity {
    pub revision: String,
    pub tree_digest: String,
    pub changed_paths_digest: String,
    pub policy_digest: String,
    pub toolchain: String,
}

/// Encodes the digest of the supplied bytes as lowercase hexadecimal.
fn hex(bytes: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(bytes.as_ref());
    format!("{digest:x}")
}

/// Run `git -C <root>` with `args`; failure to spawn, non-exit-success, or
/// non-utf8 output is an error. Stdout is trimmed.
fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git spawn failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git failed (exit {}): {stderr}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("git output not utf8: {e}"))
}

// @impl:FR-AUD-02
fn snapshot_identity(
    root: &Path,
    commit: &str,
    policy_text: &str,
    changed_paths: &[String],
) -> Result<SnapshotIdentity, String> {
    let head = git_output(root, &["rev-parse", "HEAD"])?;
    if head != commit {
        return Err(format!("HEAD {head} is not the approved commit {commit}"));
    }
    let tree_digest = git_output(root, &["rev-parse", &format!("{commit}^{{tree}}")])?;
    let dirty = git_output(root, &["status", "--porcelain"])?;
    if !dirty.is_empty() {
        return Err(format!("working tree is dirty: {dirty}"));
    }
    let policy_digest = hex(policy_text);
    let mut sorted = changed_paths.to_vec();
    sorted.sort();
    let changed_paths_digest = hex(sorted.join("\n"));
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| format!("rustc spawn failed: {e}"))?;
    if !rustc.status.success() {
        return Err("rustc --version failed".to_string());
    }
    let toolchain = String::from_utf8(rustc.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("rustc --version output not utf8: {e}"))?;
    Ok(SnapshotIdentity {
        revision: head,
        tree_digest,
        changed_paths_digest,
        policy_digest,
        toolchain,
    })
}

/// Run scoped acceptance for `commit` against `root`.
///
/// Every gate command runs inside an unprivileged `bwrap` sandbox. If `bwrap`
/// is unavailable the function returns an error; it never runs unsandboxed.
pub fn run_scoped_acceptance(root: &Path, commit: &str) -> Result<PathBuf, String> {
    let policy_path = root.join(ACCEPTANCE_POLICY);
    let policy_text = fs::read_to_string(&policy_path).map_err(|e| {
        format!(
            "failed to read acceptance policy {}: {e}",
            policy_path.display()
        )
    })?;
    let policy: AcceptancePolicy = serde_json::from_str(&policy_text)
        .map_err(|e| format!("invalid acceptance policy: {e}"))?;
    if policy.approved_commit != commit {
        return Err(format!(
            "commit {commit} is not the approved commit {}",
            policy.approved_commit
        ));
    }
    let identity = snapshot_identity(root, commit, &policy_text, &policy.changed_paths)?;

    let sysroot = sysroot()?;
    let scratch = scratch_dir()?;
    let target_dir = scratch.join("target");
    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "failed to create scratch target dir {}: {e}",
            target_dir.display()
        )
    })?;
    let base = bwrap_args(root, &sysroot, &target_dir);

    // Gate (a): Formal.
    run_bwrap(
        &base,
        &[SANDBOX_EXE, "ci", "--gate", "0", "--json"],
        "formal (ci --gate 0 --json)",
    )?;
    // Gate (b): Structural.
    run_bwrap(
        &base,
        &[SANDBOX_EXE, "check", "--rigor", "T1"],
        "structural (check --rigor T1)",
    )?;
    // Gate (c): one `asep affected` call feeds both the test plan and the
    // per-requirement test mapping. Never derive proof from an aggregate count.
    let affected = affected_value(&base, &policy.changed_paths)?;
    let plan = plan_from_value(&affected, root)?;
    let in_scope = in_scope_requirement_ids(&affected)?;
    let requirement_tests = requirement_test_map(&affected);
    let test_aliases = test_alias_map(&affected);
    let test_targets_by_id = test_target_map(&affected);
    let mut total_executed: usize = 0;
    let mut observed_passed: BTreeSet<TestIdentity> = BTreeSet::new();
    let mut observed_failed: BTreeSet<TestIdentity> = BTreeSet::new();
    for cmd in &plan.commands {
        let stdout = run_bwrap_capture(&base, &["sh", "-c", cmd], &format!("scoped test: {cmd}"))?;
        total_executed += executed_test_count(&stdout);
        let (package, target) = command_identity(cmd);
        let filter = command_test_filter(cmd);
        let (passed, failed) = executed_test_names(&stdout);
        for name in passed {
            observed_passed.insert(TestIdentity { package: package.clone(), target: target.clone(), name, filter: filter.clone() });
        }
        for name in failed {
            observed_failed.insert(TestIdentity { package: package.clone(), target: target.clone(), name, filter: filter.clone() });
        }
    }
    if total_executed == 0 {
        return Err("scoped acceptance executed zero tests".to_string());
    }
    // Gate (d): Format.
    run_bwrap(
        &base,
        &["cargo", "fmt", "--all", "--check"],
        "format (cargo fmt --all --check)",
    )?;

    // Evidence: one semantic file per `@req:*:MUST` in docs/PRD.md.
    let evidence_dir = resolve_evidence_dir(root, &policy.evidence_dir)?;
    let req_ids = must_req_ids(root)?;
    let semantic_dir = evidence_dir.join("semantic");
    fs::create_dir_all(&semantic_dir).map_err(|e| {
        format!(
            "failed to create evidence dir {}: {e}",
            semantic_dir.display()
        )
    })?;
    let changed_list = policy.changed_paths.join(", ");
    let observed: BTreeSet<&TestIdentity> = observed_passed.iter().chain(observed_failed.iter()).collect();
    let test_packages: Vec<String> = observed.iter().map(|id| id.package.clone()).collect::<BTreeSet<_>>().into_iter().collect();
    let test_targets: Vec<String> = observed.iter().map(|id| id.target.clone()).collect::<BTreeSet<_>>().into_iter().collect();
    let test_packages = test_packages.join(", ");
    let test_targets = test_targets.join(", ");
    for req_id in &req_ids {
        let associated = requirement_tests.get(req_id).cloned().unwrap_or_default();
        let matched_passed: BTreeSet<String> = associated
            .iter()
            .filter(|id| {
                let aliases = test_aliases.get(*id).cloned().unwrap_or_else(|| {
                    let mut set = BTreeSet::new();
                    set.insert((*id).clone());
                    set
                });
                aliases.iter().any(|alias| {
                    observed_passed.iter().any(|observed| {
                        observed
                            .filter
                            .as_ref()
                            .is_none_or(|filter| test_name_matches(filter, alias))
                            && test_targets_by_id.get(*id).is_none_or(|target| target == &observed.target)
                            && test_name_matches(&observed.name, alias)
                    })
                })
            })
            .cloned()
            .collect();
        let matched_failed: BTreeSet<String> = associated
            .iter()
            .filter(|id| {
                let aliases = test_aliases.get(*id).cloned().unwrap_or_else(|| {
                    let mut set = BTreeSet::new();
                    set.insert((*id).clone());
                    set
                });
                aliases.iter().any(|alias| {
                    observed_failed.iter().any(|observed| {
                        observed
                            .filter
                            .as_ref()
                            .is_none_or(|filter| test_name_matches(filter, alias))
                            && test_targets_by_id.get(*id).is_none_or(|target| target == &observed.target)
                            && test_name_matches(&observed.name, alias)
                    })
                })
            })
            .cloned()
            .collect();
        let proven = in_scope.contains(req_id)
            && !associated.is_empty()
            && matched_failed.is_empty()
            && matched_passed.len() == associated.len();
        let (verdict, evidence_state) = match proven {
            true => ("ACCEPTED", "PASS"),
            false => ("UNPROVEN", "UNPROVEN"),
        };
        let scope = if in_scope.contains(req_id) {
            "in_scope"
        } else {
            "out_of_scope"
        };
        let selected_for_req = associated.iter().cloned().collect::<Vec<_>>().join(", ");
        let passed_for_req = matched_passed.iter().cloned().collect::<Vec<_>>().join(", ");
        let aliases_for: BTreeSet<String> = matched_passed
            .iter()
            .flat_map(|id| {
                test_aliases.get(id).cloned().unwrap_or_else(|| {
                    let mut set = BTreeSet::new();
                    set.insert(id.clone());
                    set
                })
            })
            .collect();
        let matched_identities: Vec<&TestIdentity> = observed_passed
            .iter()
            .filter(|observed| {
                aliases_for.iter().any(|alias| {
                    observed
                        .filter
                        .as_ref()
                        .is_none_or(|filter| test_name_matches(filter, alias))
                        && test_name_matches(&observed.name, alias)
                })
            })
            .collect();
        let qualified_for_req = matched_identities
            .iter()
            .map(|id| id.qualified())
            .collect::<Vec<_>>()
            .join(", ");
        let passed_names: Vec<String> = matched_identities
            .iter()
            .map(|id| id.name.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let passed_for_req = if passed_names.is_empty() {
            passed_for_req
        } else {
            passed_names.join(", ")
        };
        let content = format!(
            "schema_version: 1\nrevision: {}\ntree_digest: {}\nchanged_paths_digest: {}\npolicy_digest: {}\ntoolchain: {}\nreviewer: scoped-trusted-runner\nscope: {scope}\ntest_packages: {test_packages}\ntest_targets: {test_targets}\ntests_qualified_for_requirement: {qualified_for_req}\nevidence_state: {evidence_state}\ntests_selected: {}\ntests_executed: {total_executed}\ntests_selected_for_requirement: {selected_for_req}\ntests_passed_for_requirement: {passed_for_req}\nverdict: {verdict}\nrationale: commit {commit}, scoped gates passed, changed paths {changed_list}\n",
            identity.revision,
            identity.tree_digest,
            identity.changed_paths_digest,
            identity.policy_digest,
            identity.toolchain,
            plan.selected,
        );
        fs::write(semantic_dir.join(format!("{req_id}.md")), content)
            .map_err(|e| format!("failed to write evidence for {req_id}: {e}"))?;
    }

    Ok(evidence_dir)
}

fn current_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "asep".to_string())
}

fn sysroot() -> Result<PathBuf, String> {
    let out = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .map_err(|e| format!("rustc not runnable: {e}"))?;
    if !out.status.success() {
        return Err("rustc --print sysroot failed".to_string());
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// Locate `bwrap` in PATH. Unavailable is a hard error: never fall back to
/// unsandboxed execution.
fn require_bwrap() -> Result<(), String> {
    match Command::new(bwrap_bin()).arg("--version").output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("bwrap not found in PATH; scoped acceptance requires bwrap".to_string())
        }
        Err(e) => Err(format!("bwrap spawn failed: {e}")),
        Ok(_) => Ok(()),
    }
}

pub fn bwrap_args(root: &Path, sysroot: &Path, target_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    args.extend_from_slice(&[
        "--unshare-user".to_string(),
        "--unshare-pid".to_string(),
        "--ro-bind".to_string(),
        root.display().to_string(),
        "/workspace".to_string(),
        "--tmpfs".to_string(),
        "/workspace/.spec-engine".to_string(),
    ]);
    // Re-expose only the evidence files over the tmpfs that hides the rest of
    // <root>/.spec-engine (so acceptance.json stays hidden inside the sandbox).
    // verification.json is written inside the sandbox by `asep check`, so it
    // must not be ro-bind (a read-only bind would make check fail).
    for name in ["behavioral.json"] {
        let src = root.join(".spec-engine").join(name);
        if src.is_file() {
            args.extend_from_slice(&[
                "--ro-bind".to_string(),
                src.display().to_string(),
                format!("/workspace/.spec-engine/{name}"),
            ]);
        }
    }
    args.extend_from_slice(&[
        "--bind".to_string(),
        target_dir.display().to_string(),
        "/workspace/target".to_string(),
        "--ro-bind".to_string(),
        sysroot.display().to_string(),
        "/toolchain".to_string(),
        "--ro-bind".to_string(),
        "/usr".to_string(),
        "/usr".to_string(),
        "--ro-bind".to_string(),
        "/lib".to_string(),
        "/lib".to_string(),
        "--ro-bind".to_string(),
        "/lib64".to_string(),
        "/lib64".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--chdir".to_string(),
        "/workspace".to_string(),
        "--setenv".to_string(),
        "CARGO_HOME".to_string(),
        "/workspace/.cargo".to_string(),
        "--setenv".to_string(),
        "CARGO_TARGET_DIR".to_string(),
        "/workspace/target".to_string(),
        "--setenv".to_string(),
        "PATH".to_string(),
        "/toolchain/bin:/usr/bin:/bin".to_string(),
    ]);
    // Bind the current exe into the sandbox so gate commands can run
    // /asep/bin/asep instead of a host path invisible inside bwrap.
    let exe = current_exe();
    args.extend_from_slice(&[
        "--dir".to_string(),
        "/asep".to_string(),
        "--dir".to_string(),
        "/asep/bin".to_string(),
        "--ro-bind".to_string(),
        exe,
        "/asep/bin/asep".to_string(),
    ]);
    args
}

fn run_bwrap(base: &[String], cmd: &[&str], label: &str) -> Result<(), String> {
    require_bwrap()?;
    let mut command_args: Vec<String> = base.iter().cloned().collect();
    command_args.push("--".to_string());
    command_args.extend(cmd.iter().map(|s| s.to_string()));
    let output = Command::new(bwrap_bin())
        .args(&command_args)
        .output()
        .map_err(|e| format!("failed to run bwrap for {label}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "{label} failed (exit {}): {stdout}{stderr}",
            output.status
        ));
    }
    Ok(())
}

/// Run bwrap like `run_bwrap` but return trimmed stdout on success and Err
/// (with stderr) on failure.
fn run_bwrap_capture(base: &[String], cmd: &[&str], label: &str) -> Result<String, String> {
    require_bwrap()?;
    let mut command_args: Vec<String> = base.iter().cloned().collect();
    command_args.push("--".to_string());
    command_args.extend(cmd.iter().map(|s| s.to_string()));
    let output = Command::new(bwrap_bin())
        .args(&command_args)
        .output()
        .map_err(|e| format!("failed to run bwrap for {label}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{label} failed (exit {}): {stderr}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("bwrap output not utf8 for {label}: {e}"))
}

// @impl:FR-AUD-03
/// Run `asep affected <paths> --json` once inside the sandbox and parse the
/// first JSON value. Fail closed on spawn, exit status, JSON parse or empty output.
fn affected_value(base: &[String], changed_paths: &[String]) -> Result<serde_json::Value, String> {
    let mut cmd: Vec<&str> = vec![SANDBOX_EXE, "affected"];
    cmd.extend(changed_paths.iter().map(|s| s.as_str()));
    cmd.push("--json");
    let stdout = run_bwrap_capture(base, &cmd, "asep affected --json")?;
    let mut values = serde_json::Deserializer::from_str(&stdout).into_iter::<serde_json::Value>();
    match values.next() {
        Some(Ok(value)) => Ok(value),
        Some(Err(e)) => Err(format!("asep affected output is not JSON: {e}")),
        None => Err("asep affected output is empty".to_string()),
    }
}

/// Requirement ids actually reported in scope by the single affected call.
fn in_scope_requirement_ids(value: &serde_json::Value) -> Result<HashSet<String>, String> {
    let requirements = value
        .get("requirements")
        .and_then(|v| v.as_array())
        .ok_or("asep affected output missing requirements array")?;
    let mut ids = HashSet::new();
    for req in requirements {
        let id = req
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("requirement entry missing id string")?;
        ids.insert(id.to_string());
    }
    Ok(ids)
}

/// Map each requirement to the test identifiers whose typed provenance chain
/// reaches it. A test with no typed link is never associated, so an aggregate
/// pass can no longer prove an unrelated requirement.
fn requirement_test_map(value: &serde_json::Value) -> HashMap<String, BTreeSet<String>> {
    let mut result: HashMap<String, BTreeSet<String>> = HashMap::new();
    let Some(provenance) = value.get("provenance").and_then(|v| v.as_object()) else {
        return result;
    };
    for (key, hops) in provenance {
        let Some(test_id) = key.strip_prefix("Test:") else {
            continue;
        };
        let Some(hops) = hops.as_array() else {
            continue;
        };
        for hop in hops {
            let Some(node) = hop.get("node").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(req_id) = node.strip_prefix("Req:") else {
                continue;
            };
            result
                .entry(req_id.to_string())
                .or_default()
                .insert(test_id.to_string());
        }
    }
    result
}

/// Aliases (node id and extracted function name) for each reported test id.
fn test_alias_map(value: &serde_json::Value) -> HashMap<String, BTreeSet<String>> {
    let mut result: HashMap<String, BTreeSet<String>> = HashMap::new();
    let Some(tests) = value.get("tests").and_then(|v| v.as_array()) else {
        return result;
    };
    for test in tests {
        let Some(id) = test.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let aliases = result.entry(id.to_string()).or_default();
        aliases.insert(id.to_string());
        if let Some(function) = test.get("function").and_then(|v| v.as_str()) {
            aliases.insert(function.to_string());
        }
    }
    result
}

/// A test actually observed in a command's output, attributed to the package
/// and target of that command. Names alone are not identities: the same
/// function name can exist in several packages or test targets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TestIdentity {
    package: String,
    target: String,
    name: String,
    filter: Option<String>,
}

impl TestIdentity {
    fn qualified(&self) -> String {
        format!("{}::{}::{}", self.package, self.target, self.name)
    }
}

/// Extracts `-p <package>` and the test target from a planned Cargo command.
/// Defaults are explicit Cargo semantics, not a hardcoded ASEP package.
fn command_identity(command: &str) -> (String, String) {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut package = "cargo".to_string();
    let mut target = "lib".to_string();
    for (i, token) in tokens.iter().enumerate() {
        if *token == "-p" || *token == "--package" {
            if let Some(value) = tokens.get(i + 1) {
                package = value.to_string();
            }
        }
        if *token == "--test" {
            if let Some(value) = tokens.get(i + 1) {
                target = value.to_string();
            }
        }
    }
    (package, target)
}

/// Exact test-name filter of a planned command (`-- <name> --exact`), when
/// present. A test identity must belong to the command that selected it.
fn command_test_filter(command: &str) -> Option<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    for (i, token) in tokens.iter().enumerate() {
        if *token == "--" {
            let name = tokens.get(i + 1).copied()?;
            if name.starts_with('-') {
                continue;
            }
            return Some(name.to_string());
        }
    }
    None
}

/// Expected test target for a traced test id, derived from its source file.
fn test_target_map(value: &serde_json::Value) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let Some(tests) = value.get("tests").and_then(|v| v.as_array()) else {
        return result;
    };
    for test in tests {
        let (Some(id), Some(file)) = (
            test.get("id").and_then(|v| v.as_str()),
            test.get("file").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let target = Path::new(file)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(file)
            .to_string();
        result.insert(id.to_string(), target);
    }
    result
}

/// Names reported by `cargo test` as passed and failed, e.g.
/// `test tests::verifies_a ... ok`.
fn executed_test_names(output: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut passed = BTreeSet::new();
    let mut failed = BTreeSet::new();
    for line in output.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        if let Some(name) = rest.strip_suffix(" ... ok") {
            passed.insert(name.trim().to_string());
        } else if let Some(name) = rest.strip_suffix(" ... FAILED") {
            failed.insert(name.trim().to_string());
        }
    }
    (passed, failed)
}

/// Compare a cargo test name with a traceability test identifier, tolerating
/// module-qualified names (`tests::foo`) and bare function names.
fn test_name_matches(observed: &str, ident: &str) -> bool {
    observed == ident
        || observed.rsplit("::").next() == Some(ident)
        || ident.rsplit("::").next() == Some(observed)
}

/// Sum the `N` in lines matching `test result: ok. N passed`; ignore lines
/// with 0.
fn executed_test_count(output: &str) -> usize {
    // Individual `test <name> ... ok|FAILED` lines are the most precise signal;
    // a failed test is still an executed test. Fall back to the Cargo summary.
    let individual = output
        .lines()
        .filter(|line| {
            let line = line.trim();
            line.starts_with("test ")
                && (line.contains(" ... ok") || line.contains(" ... FAILED"))
        })
        .count();
    if individual > 0 {
        return individual;
    }
    let mut total: usize = 0;
    for line in output.lines() {
        let line = line.trim();
        if !line.starts_with("test result: ok.") {
            continue;
        }
        let after_ok = &line["test result: ok.".len()..];
        let count: usize = match after_ok.split_whitespace().next().unwrap_or("").parse() {
            Ok(n) => n,
            Err(_) => 0,
        };
        if count > 0 {
            total += count;
        }
    }
    total
}

// @impl:FR-AUD-05
#[derive(Debug)]
pub struct TestPlan {
    pub commands: Vec<String>,
    pub selected: usize,
}

/// Run `asep affected <paths> --json` inside the sandbox and build a test plan
/// from the `commands` array of the first JSON value. Fail closed on spawn,
/// JSON parse, or missing/invalid `commands`. Commands beginning with
/// `cargo test` get `-p asep` inserted after `cargo test` unless they already
/// carry a `-p` token. A command naming a `--test <target>` whose
/// `root/spec-engine/tests/<target>.rs` does not exist is replaced with the
/// `cargo test -p asep --lib` fallback; identical fallbacks are deduplicated.
/// Target discovery is dynamic: the tests directory is consulted, no target
/// list is hardcoded.
pub fn build_test_plan(
    base: &[String],
    changed_paths: &[String],
    root: &Path,
) -> Result<TestPlan, String> {
    plan_from_value(&affected_value(base, changed_paths)?, root)
}

/// Build the scoped test plan from an already-parsed affected report so the
/// `asep affected` call is not repeated within one request.
fn plan_from_value(value: &serde_json::Value, root: &Path) -> Result<TestPlan, String> {
    let commands = match value.get("commands").and_then(|v| v.as_array()) {
        Some(array) => array,
        None => return Err("asep affected output missing commands array".to_string()),
    };
    const FALLBACK: &str = "cargo test -p asep --lib";
    let tests_dir = root.join("crates").join("spec-engine").join("tests");
    let mut planned: Vec<String> = Vec::new();
    let mut fallbacks: HashSet<String> = HashSet::new();
    for item in commands {
        let command = match item.as_str() {
            Some(s) => s.to_string(),
            None => return Err("asep affected command is not a string".to_string()),
        };
        let mut final_cmd = command;
        let is_cargo_test = final_cmd.starts_with("cargo test");
        let has_p = final_cmd.split_whitespace().any(|token| token == "-p");
        if is_cargo_test && !has_p {
            let rest = final_cmd.strip_prefix("cargo test").unwrap_or("");
            final_cmd = format!("cargo test -p asep{rest}");
        }
        let missing_target = test_target(&final_cmd)
            .map(|target| !tests_dir.join(format!("{target}.rs")).is_file())
            .unwrap_or(false);
        if missing_target {
            fallbacks.insert(FALLBACK.to_string());
            continue;
        }
        planned.push(final_cmd);
    }
    planned.extend(fallbacks.into_iter());
    let selected = planned.len();
    Ok(TestPlan {
        commands: planned,
        selected,
    })
}

/// Target of a `--test <target>` token in `command`, or None when absent.
fn test_target(command: &str) -> Option<&str> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    for (i, token) in tokens.iter().enumerate() {
        if *token == "--test" {
            return tokens.get(i + 1).copied();
        }
    }
    None
}

fn resolve_evidence_dir(root: &Path, evidence_dir: &str) -> Result<PathBuf, String> {
    let p = Path::new(evidence_dir);
    match p.is_absolute() {
        true => Ok(p.to_path_buf()),
        false => Ok(root.join(p)),
    }
}

/// Every `@req:ID:MUST` id in `prd_path` (file missing → no requirements).
fn must_req_ids(root: &Path) -> Result<Vec<String>, String> {
    let re = Regex::new(r"@req:([A-Za-z0-9_./-]+):MUST").map_err(|e| format!("bad regex: {e}"))?;
    let mut ids: Vec<String> = Vec::new();
    for name in ["PRD.md", "TDD.md"] {
        let path = root.join("docs").join(name);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("failed to read {path:?}: {e}")),
        };
        for cap in re.captures_iter(&text) {
            let id = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            match id.is_empty() {
                true => {}
                false => {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
    }
    Ok(ids)
}

/// Unique scratch dir for bind-mounting the cargo target dir.
fn scratch_dir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    for i in 0..10_000 {
        let dir = base.join(format!("asep-acceptance-scratch-{pid}-{i}"));
        if fs::create_dir_all(&dir).is_ok() {
            return Ok(dir);
        }
    }
    Err("failed to create scratch dir".to_string())
}
