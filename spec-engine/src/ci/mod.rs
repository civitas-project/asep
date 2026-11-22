//! Unified 5-Gate Local CI Pipeline Runner.

use crate::auditor::{audit_project_anti_slop, audit_project_docstrings};
use crate::formal::validate_formal_specs;
use crate::graph::builder::{build_relation_graph, build_trace_graph};
use crate::policy::{load_policy, Severity, TierId};
use crate::scanner::scan_project_tags;
use crate::skill::inspector::detect_skill_drift;
use crate::verification::evaluate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiOptions {
    pub gate: Option<u8>,
    pub skip: Vec<u8>,
    pub rigor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_number: u8,
    pub name: String,
    pub passed: bool,
    pub skipped: bool,
    pub duration_ms: u128,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiReport {
    pub all_passed: bool,
    pub total_duration_ms: u128,
    pub gates: Vec<GateResult>,
    pub unproven: bool,
}

impl CiReport {
    pub fn all_passed(&self) -> bool {
        self.all_passed
    }

    /// Fail-closed gate audit. Returns true only when every gate id in
    /// `required` appears exactly once in this report with `passed == true`
    /// and `skipped == false`, the report has no unproven evidence,
    /// `all_passed == true`, every requested id is a known gate (0..=4), and
    /// `required` is nonempty with no duplicates. Any violation returns false
    /// regardless of the aggregate; it does not trust `all_passed` alone.
    pub fn passes_required(&self, required: &[u8]) -> bool {
        if required.is_empty() {
            return false;
        }

        let mut seen_req = HashSet::new();
        for &g in required {
            if g > 4 {
                return false; // unknown gate id
            }
            if !seen_req.insert(g) {
                return false; // duplicate request
            }
        }

        if self.unproven {
            return false;
        }
        if !self.all_passed {
            return false;
        }

        for &g in required {
            let results: Vec<&GateResult> = self
                .gates
                .iter()
                .filter(|r| r.gate_number == g)
                .collect();
            if results.len() != 1 {
                return false; // missing or duplicate result
            }
            let r = results[0];
            if !r.passed || r.skipped {
                return false;
            }
        }

        true
    }
}

/// `--strict` is active for PRs targeting `dev` or `main`; local/feature-branch
/// runs leave it off.
fn ci_strict() -> bool {
    env::var("GITHUB_BASE_REF")
        .map(|v| v == "dev" || v == "main")
        .unwrap_or(false)
}

pub fn run_ci_pipeline(dir: &Path, opts: &CiOptions) -> CiReport {
    let start = Instant::now();
    let mut gates = Vec::new();
    let mut overall_success = true;
    let mut unproven = false;

    let target_gates: Vec<u8> = match opts.gate {
        Some(g) => vec![g],
        None => vec![0, 1, 2, 3, 4],
    };

    for &g in &target_gates {
        if opts.skip.contains(&g) {
            gates.push(GateResult {
                gate_number: g,
                name: gate_name(g).to_string(),
                passed: true,
                skipped: true,
                duration_ms: 0,
                details: vec!["Skipped by user flag".to_string()],
            });
            continue;
        }

        let g_start = Instant::now();
        let res = match g {
            0 => run_gate_0(dir),
            1 => run_gate_1(dir, &opts.rigor),
            2 => run_gate_2(dir),
            3 => run_gate_3(dir),
            4 => run_gate_4(dir),
            _ => GateResult {
                gate_number: g,
                name: "Unknown Gate".to_string(),
                passed: false,
                skipped: false,
                duration_ms: 0,
                details: vec![format!("Gate {g} does not exist (valid: 0..4)")],
            },
        };

        if g == 1 && !res.passed && !res.skipped {
            // Gate 1 unproven (missing semantic evidence) propagates exit 3.
            unproven = res.details.iter().any(|d| d.contains("unproven"));
        }

        if !res.passed && !res.skipped {
            overall_success = false;
        }

        let mut final_res = res;
        final_res.duration_ms = g_start.elapsed().as_millis();
        gates.push(final_res);

        // Early exit on failure in strict CI
        if !overall_success {
            break;
        }
    }

    CiReport {
        all_passed: overall_success,
        total_duration_ms: start.elapsed().as_millis(),
        gates,
        unproven,
    }
}

fn gate_name(gate: u8) -> &'static str {
    match gate {
        0 => "Gate 0: Formal Specifications",
        1 => "Gate 1: Structural Traceability, Contracts & Skill Drift",
        2 => "Gate 2: Static Analysis (Clippy/Linter)",
        3 => "Gate 3: Automated Test & Invariant Verification",
        4 => "Gate 4: Benchmark Compilation/Preflight Gate (no runtime performance proof)",
        _ => "Unknown Gate",
    }
}

fn run_gate_0(dir: &Path) -> GateResult {
    let formal_checks = validate_formal_specs(dir);
    let mut passed = true;
    let mut skipped = false;
    let mut details = Vec::new();

    for check in formal_checks {
        if !check.passed && !check.skipped {
            passed = false;
            details.push(format!("[FAIL] {}: {} ({})", check.tool, check.file, check.message));
            continue;
        }
        if check.skipped {
            // Detected model could not be proven; surface as skipped rather
            // than silently treating it as passed.
            skipped = true;
            details.push(format!("[SKIP] {}: {} ({})", check.tool, check.file, check.message));
            continue;
        }
        details.push(format!("[PASS] {}: {}", check.tool, check.file));
    }

    if details.is_empty() {
        // Empty model set keeps existing behavior: passed, not skipped.
        details.push("No formal specification models (.als, .tla, .dbml, api.yaml) found".to_string());
    }

    GateResult {
        gate_number: 0,
        name: gate_name(0).to_string(),
        passed,
        skipped,
        duration_ms: 0,
        details,
    }
}

fn run_gate_1(dir: &Path, rigor: &str) -> GateResult {
    let scanner_result = scan_project_tags(dir);
    let mut details = Vec::new();

    match scanner_result {
        Ok(tags) => {
            let tier: TierId = rigor.parse().unwrap_or(TierId::T1);
            let policy = load_policy();
            let graph = build_relation_graph(&tags, &policy);
            let strict = ci_strict();

            // Anti-slop audit: Error violations block outside T0; Warning
            // violations block only under --strict (ci_strict).
            let anti_slop_violations = audit_project_anti_slop(dir).unwrap_or_default();
            let anti_errors: Vec<_> = anti_slop_violations
                .iter()
                .filter(|v| v.severity == Severity::Error)
                .collect();
            let anti_warnings: Vec<_> = anti_slop_violations
                .iter()
                .filter(|v| v.severity == Severity::Warning)
                .collect();
            let has_antislop_failure =
                tier != TierId::T0 && (!anti_errors.is_empty() || (strict && !anti_warnings.is_empty()));

            // COV-02 is a warning; `--strict` promotes it to failure.
            let has_cov02 = graph
                .violations
                .iter()
                .any(|v| v.rule_id == "COV-02");
            let passed = graph.is_closed && !(strict && has_cov02) && !has_antislop_failure;

            if !passed && strict && has_cov02 {
                details.push("COV-02 coverage warnings promoted to failures by --strict".to_string());
            }
            if !passed {
                for v in &graph.violations {
                    details.push(format!(
                        "[FAIL] {}: {} (found {}, required {})",
                        v.rule_id, v.subject, v.found, v.required
                    ));
                }
            }
            if passed {
                details.push(format!("Closed MUST gates: structural closed, {} violations", graph.violations.len()));
            }

            for v in &anti_errors {
                let blocked = tier != TierId::T0;
                let status = match blocked {
                    true => "FAIL",
                    false => "WARN",
                };
                details.push(format!(
                    "[{}] anti-slop {}:{} {}",
                    status,
                    v.file_path.display(),
                    v.line_number,
                    v.message
                ));
            }
            for v in &anti_warnings {
                let blocked = strict && tier != TierId::T0;
                let status = match blocked {
                    true => "FAIL",
                    false => "WARN",
                };
                details.push(format!(
                    "[{}] anti-slop warning {}:{} {}",
                    status,
                    v.file_path.display(),
                    v.line_number,
                    v.message
                ));
            }

            let behavioral = dir.join(".spec-engine").join("behavioral.json");
            let evidence_dir = dir.join("evidence");
            let report = evaluate(&graph, &evidence_dir, tier, &behavioral);
            if report.status == crate::verification::VerificationStatus::Unproven {
                details.push("Gate 1 unproven: semantic evidence required but missing".to_string());
            }

            GateResult {
                gate_number: 1,
                name: gate_name(1).to_string(),
                passed,
                skipped: false,
                duration_ms: 0,
                details,
            }
        }
        Err(e) => GateResult {
            gate_number: 1,
            name: gate_name(1).to_string(),
            passed: false,
            skipped: false,
            duration_ms: 0,
            details: vec![format!("Scanner failed: {e}")],
        },
    }
}

/// Trusted read-only structural acceptance check for a caller-pinned snapshot.
///
/// The caller owns snapshot integrity/pinning; this function only inspects
/// the tree and never reads `behavioral.json`, writes files, or executes
/// candidate programs. It mirrors `runner.rs` gate 1 conventions:
/// `scan_project_tags`, `build_relation_graph`, `build_trace_graph`,
/// `audit_project_docstrings`, `audit_project_anti_slop`, and
/// `detect_skill_drift`.
///
/// This helper is fixed at strict T1: neither the caller's `GITHUB_BASE_REF`,
/// a candidate `@rigor:T0` tag, nor workspace `config.rigor` can downgrade
/// the guard clauses. Pending SHOULD requirements always fail, all anti-slop
/// errors and warnings fail, and drafts are always rejected.
///
/// Rejections use bounded diagnostics and guard clauses:
///   * tag scan failure or relation graph not closed (error-severity coverage
///     violation)
///   * broken MUST traceability (trace graph not closed)
///   * empty requirement set: vacuous closure is not acceptance
///   * pending SHOULD requirements (always strict in this helper)
///   * any draft requirement (drafts are rejected for local acceptance)
///   * docstring contract violations
///   * all anti-slop errors and warnings (fixed strict T1)
///   * installed skill drift in `.opencode/skills` or
///     `.agents/plugins/asep/skills`
pub(crate) fn check_acceptance_structure(root: &Path) -> Result<(), String> {
    // Guard clause: root must be an existing directory.
    if !root.is_dir() {
        return Err(format!(
            "acceptance structure check failed: root is not a directory: {}",
            root.display()
        ));
    }

    // Per-category diagnostic cap keeps rejection messages bounded.
    const LIMIT: usize = 6;

    let mut failures: Vec<String> = Vec::new();

    // Graph stage: scan tags, then relation graph
    let tags = match scan_project_tags(root) {
        Ok(tags) => tags,
        Err(e) => {
            return Err(format!(
                "acceptance structure check failed: tag scan rejected: {e}"
            ));
        }
    };
    let policy = load_policy();
    let graph = build_relation_graph(&tags, &policy);
    if !graph.is_closed {
        failures.push(format!(
            "relation graph is not closed: {} coverage violation(s)",
            graph.violations.len()
        ));
        for v in graph
            .violations
            .iter()
            .filter(|v| {
                policy
                    .coverage_rules()
                    .iter()
                    .any(|r| r.id == v.rule_id && r.severity == Severity::Error)
            })
            .take(LIMIT)
        {
            failures.push(format!(
                "[{}] {} (found {}, required {})",
                v.rule_id, v.subject, v.found, v.required
            ));
        }
    }

    // Trace stage: MUST closure, SHOULD pending, drafts
    let report = build_trace_graph(&tags);

    if !report.is_closed {
        failures.push(format!(
            "trace graph is not closed: {} broken MUST requirement(s)",
            report.must_broken.len()
        ));
        for b in report.must_broken.iter().take(LIMIT) {
            failures.push(format!(
                "[MUST] {} ({}:{})",
                b.id,
                b.file_path.display(),
                b.line_number
            ));
        }
    }

    // Vacuous closure is not acceptance: an approved snapshot anchors at least
    // one approved requirement.
    if report.total_requirements == 0 {
        failures.push(
            "no approved requirements found: empty trace graph fails acceptance".to_string(),
        );
    }

    // Trusted local acceptance is always strict: `GITHUB_BASE_REF` cannot
    // downgrade guard clauses in this helper.
    let strict = true;
    if strict && !report.should_pending.is_empty() {
        failures.push(format!(
            "pending SHOULD requirement(s) under --strict: {}",
            report.should_pending.len()
        ));
        for p in report.should_pending.iter().take(LIMIT) {
            failures.push(format!(
                "[SHOULD] {} ({}:{})",
                p.id,
                p.file_path.display(),
                p.line_number
            ));
        }
    }

    if report.draft_total > 0 {
        failures.push(format!(
            "draft requirement(s) rejected for acceptance: {}",
            report.draft_total
        ));
        for d in report.draft_pending.iter().take(LIMIT) {
            failures.push(format!(
                "[DRAFT] {} ({}:{})",
                d.id,
                d.file_path.display(),
                d.line_number
            ));
        }
    }

    // Project docstring contracts
    let doc_violations = match audit_project_docstrings(root) {
        Ok(violations) => violations,
        Err(e) => {
            return Err(format!(
                "acceptance structure check failed: docstring audit rejected: {e}"
            ));
        }
    };
    if !doc_violations.is_empty() {
        failures.push(format!(
            "docstring contract violation(s): {}",
            doc_violations.len()
        ));
        for v in doc_violations.iter().take(LIMIT) {
            failures.push(format!(
                "[doc] {}:{}: {}",
                v.file_path.display(),
                v.line_number,
                v.message
            ));
        }
    }

    // Anti-slop (runner.rs gate 1 convention: Error blocks outside T0,
    // Warning blocks only under --strict)
    // Fixed strict T1 for trusted local acceptance: a candidate `@rigor:T0`
    // tag or workspace `config.rigor` cannot downgrade guard clauses here.
    let tier = TierId::T1;
    let anti_violations = match audit_project_anti_slop(root) {
        Ok(violations) => violations,
        Err(e) => {
            return Err(format!(
                "acceptance structure check failed: anti-slop audit rejected: {e}"
            ));
        }
    };
    let anti_errors: Vec<_> = anti_violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .collect();
    let anti_warnings: Vec<_> = anti_violations
        .iter()
        .filter(|v| v.severity == Severity::Warning)
        .collect();
    let has_antislop_failure =
        tier != TierId::T0 && (!anti_errors.is_empty() || (strict && !anti_warnings.is_empty()));
    if has_antislop_failure {
        failures.push(format!(
            "anti-slop violation(s) under strict/T1: {} error(s), {} warning(s)",
            anti_errors.len(),
            anti_warnings.len()
        ));
        for v in anti_errors
            .iter()
            .chain(anti_warnings.iter())
            .take(LIMIT)
        {
            failures.push(format!(
                "[anti-slop] {}:{}: {}",
                v.file_path.display(),
                v.line_number,
                v.message
            ));
        }
    }

    // Installed skill drift (runner.rs convention)
    let skill_drift_dirs = [
        root.join(".opencode").join("skills"),
        root.join(".agents").join("plugins").join("asep").join("skills"),
    ];
    let skill_drifts: Vec<_> = skill_drift_dirs
        .iter()
        .flat_map(|d| detect_skill_drift(d))
        .collect();
    if !skill_drifts.is_empty() {
        failures.push(format!(
            "installed skill drift detected: {}",
            skill_drifts.len()
        ));
        for d in skill_drifts.iter().take(LIMIT) {
            failures.push(format!(
                "[skill drift] {} ({}): {}",
                d.skill_id, d.kind, d.path.display()
            ));
        }
    }

    #[allow(clippy::match_bool)]
    // `match bool` replaces `if/else` to keep zero `else` tokens (AS-CODE-ELSE).
    match failures.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "acceptance structure check failed:\n{}",
            failures.join("\n")
        )),
    }
}

fn run_gate_2(dir: &Path) -> GateResult {
    if !dir.join("Cargo.toml").exists() {
        return GateResult {
            gate_number: 2,
            name: gate_name(2).to_string(),
            passed: true,
            skipped: true,
            duration_ms: 0,
            details: vec!["No Cargo.toml found in directory; linter skipped".to_string()],
        };
    }

    let out = Command::new("cargo")
        .arg("clippy")
        .arg("--")
        .arg("-D")
        .arg("warnings")
        .current_dir(dir)
        .output();

    match out {
        Ok(res) => {
            let passed = res.status.success();
            GateResult {
                gate_number: 2,
                name: gate_name(2).to_string(),
                passed,
                skipped: false,
                duration_ms: 0,
                details: vec![match passed {
                    true => "Clippy passed with zero warnings".to_string(),
                    false => String::from_utf8_lossy(&res.stderr).trim().to_string(),
                }],
            }
        }
        Err(e) => GateResult {
            gate_number: 2,
            name: gate_name(2).to_string(),
            passed: false,
            skipped: false,
            duration_ms: 0,
            details: vec![e.to_string()],
        },
    }
}

fn run_gate_3(dir: &Path) -> GateResult {
    if !dir.join("Cargo.toml").exists() {
        return GateResult {
            gate_number: 3,
            name: gate_name(3).to_string(),
            passed: true,
            skipped: true,
            duration_ms: 0,
            details: vec!["No Cargo.toml found in directory; tests skipped".to_string()],
        };
    }

    let out = Command::new("cargo")
        .arg("test")
        .current_dir(dir)
        .output();

    let passed = match out {
        Ok(res) => res.status.success(),
        Err(_) => false,
    };

    // Write `.spec-engine/behavioral.json` with `passed` and `skipped`.
    let spec_engine_dir = dir.join(".spec-engine");
    let _ = fs::create_dir_all(&spec_engine_dir).and_then(|_| {
        fs::write(
            spec_engine_dir.join("behavioral.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "passed": passed,
                "skipped": Vec::<String>::new()
            })).unwrap_or_default()
        )
    });

    GateResult {
        gate_number: 3,
        name: gate_name(3).to_string(),
        passed,
        skipped: false,
        duration_ms: 0,
        details: vec![match passed {
            true => "Test suite passed completely".to_string(),
            false => "Test suite failed".to_string(),
        }],
    }
}

fn run_gate_4(dir: &Path) -> GateResult {
    let bench_dir = dir.join("benches");
    if !bench_dir.exists() {
        return GateResult {
            gate_number: 4,
            name: gate_name(4).to_string(),
            passed: true,
            skipped: true,
            duration_ms: 0,
            details: vec!["No benches directory found; benchmark gate skipped".to_string()],
        };
    }

    let out = Command::new("cargo")
        .arg("bench")
        .arg("--no-run")
        .current_dir(dir)
        .output();

    match out {
        Ok(res) => {
            let passed = res.status.success();
            GateResult {
                gate_number: 4,
                name: gate_name(4).to_string(),
                passed,
                skipped: false,
                duration_ms: 0,
                details: vec![match passed {
                    true => "Benchmark harness compiled successfully (preflight; no runtime performance proof)".to_string(),
                    false => String::from_utf8_lossy(&res.stderr).trim().to_string(),
                }],
            }
        }
        Err(e) => GateResult {
            gate_number: 4,
            name: gate_name(4).to_string(),
            passed: false,
            skipped: false,
            duration_ms: 0,
            details: vec![e.to_string()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn gate_result(gate_number: u8, passed: bool, skipped: bool) -> GateResult {
        GateResult {
            gate_number,
            name: gate_name(gate_number).to_string(),
            passed,
            skipped,
            duration_ms: 0,
            details: Vec::new(),
        }
    }

    fn report(gates: Vec<GateResult>, unproven: bool, all_passed: bool) -> CiReport {
        CiReport {
            all_passed,
            total_duration_ms: 0,
            gates,
            unproven,
        }
    }

    #[test]
    fn passes_required_empty_request_fails() {
        let r = report(vec![gate_result(0, true, false)], false, true);
        assert!(!r.passes_required(&[]));
    }

    #[test]
    fn passes_required_duplicate_request_fails() {
        let r = report(vec![gate_result(0, true, false)], false, true);
        assert!(!r.passes_required(&[0, 0]));
    }

    #[test]
    fn passes_required_unknown_id_fails() {
        let r = report(vec![gate_result(0, true, false)], false, true);
        assert!(!r.passes_required(&[9]));
    }

    #[test]
    fn passes_required_missing_result_fails() {
        let r = report(vec![gate_result(0, true, false)], false, true);
        assert!(!r.passes_required(&[1]));
    }

    #[test]
    fn passes_required_duplicate_result_fails() {
        let r = report(vec![gate_result(0, true, false), gate_result(0, true, false)], false, true);
        assert!(!r.passes_required(&[0]));
    }

    #[test]
    fn passes_required_skipped_result_fails() {
        let r = report(vec![gate_result(0, true, true)], false, true);
        assert!(!r.passes_required(&[0]));
    }

    #[test]
    fn passes_required_failed_result_fails() {
        let r = report(vec![gate_result(0, false, false)], false, true);
        assert!(!r.passes_required(&[0]));
    }

    #[test]
    fn passes_required_unproven_fails() {
        let r = report(vec![gate_result(0, true, false)], true, true);
        assert!(!r.passes_required(&[0]));
    }

    #[test]
    fn passes_required_aggregate_false_fails() {
        let r = report(vec![gate_result(0, true, false)], false, false);
        assert!(!r.passes_required(&[0]));
    }

    #[test]
    fn passes_required_passing() {
        let r = report(
            vec![gate_result(0, true, false), gate_result(1, true, false), gate_result(4, true, false)],
            false,
            true,
        );
        assert!(r.passes_required(&[0, 1, 4]));
        assert!(r.passes_required(&[4, 0, 1]));
        assert!(r.passes_required(&[1]));
    }

        #[test]
    fn test_check_acceptance_structure_empty_repo_rejected() {
        // Vacuous closure (zero requirements) must not pass acceptance.
        std::env::remove_var("GITHUB_BASE_REF");
        let tmp = tempdir().unwrap();
        let result = check_acceptance_structure(tmp.path());
        assert!(
            result.is_err(),
            "empty repo must be rejected: {:?}",
            result
        );
        assert!(
            result.unwrap_err().contains("no approved requirements"),
            "expected empty-requirement rejection"
        );
    }

        #[test]
    fn test_check_acceptance_structure_draft_rejected() {
        std::env::remove_var("GITHUB_BASE_REF");
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("spec.md"),
            "# Draft acceptance spec\n\n@req:FR-ACCEPT-DRAFT-01:DRAFT\n",
        )
        .unwrap();
        let result = check_acceptance_structure(root);
        assert!(result.is_err(), "draft requirement must be rejected: {:?}", result);
        assert!(
            result.unwrap_err().contains("draft"),
            "expected draft rejection"
        );
    }

    #[test]
    fn test_check_acceptance_structure_t0_cannot_bypass_known_error() {
        // A candidate `@rigor:T0` tag must not disable guard clauses: an
        // anti-slop Error still fails acceptance under fixed strict T1.
        // Fixture tags are assembled with a separated `@` so the workspace
        // lexer never sees a complete trace tag in this test source.
        std::env::remove_var("GITHUB_BASE_REF");
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        let at = "@";
        let id = "FR-T0-BYPASS";
        fs::write(
            root.join("spec.md"),
            format!(
                "# Candidate lowers rigor to T0\n\n{at}rigor:T0\n{at}req:{id}\n{at}rel:satisfies:{id}->{id}\n{at}rel:verifies:{id}->{id} \"covers the gate\"\n",
            ),
        )
        .unwrap();
        fs::write(
            root.join("gate.rs"),
            format!(
                "// {at}impl:{id}\npub fn gate(x: bool) {{ if x {{ return; }} else {{ }} }}\n",
            ),
        )
        .unwrap();
        fs::write(
            root.join("gate_test.rs"),
            format!("// {at}test:{id}\n#[test]\nfn gate_ok() {{}}\n"),
        )
        .unwrap();
        let result = check_acceptance_structure(root);
        assert!(
            result.is_err(),
            "@rigor:T0 must not avoid the known anti-slop error: {:?}",
            result
        );
        assert!(
            result.unwrap_err().contains("anti-slop"),
            "expected anti-slop failure despite @rigor:T0"
        );
    }
}
