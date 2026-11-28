//! Unified 5-Gate Local CI Pipeline Runner.

use crate::auditor::{audit_ambient_skills, audit_project_anti_slop, audit_project_docstrings};
use crate::formal::{
    aggregate_formal_state, formal_state, is_required_formal_check, validate_formal_specs,
    FormalState,
};
use crate::graph::builder::build_relation_graph;
use crate::graph::model::NodeKind;
use crate::policy::{load_policy, Severity, TierId};
use crate::scanner::lexer::{Priority, TagKind};
use crate::scanner::scan_project_tags;
use crate::verification::{evaluate, evaluate_bound, EvidenceBinding, VerificationStatus};
use serde::{Deserialize, Serialize};
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
    pub unproven: bool,
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
}

/// `--strict` is active for PRs targeting `dev` or `main`; local/feature-branch
/// runs leave it off.
pub fn ci_strict() -> bool {
    env::var("GITHUB_BASE_REF")
        .map(|v| v == "dev" || v == "main")
        .unwrap_or(false)
}

/// Default CI never includes the benchmark gate. An explicit `--gate 4` is
/// still routed to an honest unproven result, not to Cargo.
fn ci_target_gates(opts: &CiOptions) -> Vec<u8> {
    match opts.gate {
        Some(g) => vec![g],
        None => vec![0, 1, 2, 3],
    }
}

/// Shared check core used by CLI and MCP. Computes canonical verification
/// semantics once: structural graph, evidence evaluation and selected audits.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Validator {
    Traceability,
    Verification,
    DocContracts,
    AmbientSkills,
    AntiSlop,
    SkillDrift,
}

impl std::str::FromStr for Validator {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "traceability" => Ok(Self::Traceability),
            "verification" => Ok(Self::Verification),
            "doc-contracts" => Ok(Self::DocContracts),
            "ambient-skills" => Ok(Self::AmbientSkills),
            "anti-slop" => Ok(Self::AntiSlop),
            "skill-drift" => Ok(Self::SkillDrift),
            _ => Err(format!("unknown validator: {value}")),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckOptions {
    pub rigor: String,
    pub strict: bool,
    pub deny_drafts: bool,
    pub validators: Vec<Validator>,
    pub evidence_dir: Option<std::path::PathBuf>,
    pub behavioral_report: Option<std::path::PathBuf>,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            rigor: "T1".to_string(),
            strict: ci_strict(),
            deny_drafts: false,
            validators: vec![Validator::Traceability, Validator::Verification],
            evidence_dir: None,
            behavioral_report: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Verdict {
    Pass,
    Fail,
    Unproven,
}

impl Verdict {
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
            Self::Unproven => 3,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckOutcome {
    pub passed: bool,
    pub verdict: Verdict,
    pub status: VerificationStatus,
    pub structural: bool,
    pub behavioral: bool,
    pub semantic: bool,
    pub total_requirements: usize,
    pub must_broken: usize,
    pub should_pending: usize,
    pub draft_total: usize,
    pub doc_contract_violations: usize,
    pub ambient_violations: usize,
    pub anti_slop_errors: usize,
    pub anti_slop_warnings: usize,
    pub skill_drifts: usize,
    pub denied_drafts: bool,
    pub validators: Vec<String>,
    pub skipped: Vec<String>,
    pub details: Vec<String>,
}

/// One canonical verification implementation shared by CLI and MCP. Only
/// explicitly selected validators affect the verdict; unrelated audits are
/// reported by their own commands, never bundled into traceability silently.
pub fn check_core(dir: &Path, opts: &CheckOptions) -> Result<CheckOutcome, String> {
    let tags = scan_project_tags(dir).map_err(|e| e.to_string())?;
    let policy = load_policy();
    let graph = build_relation_graph(&tags, &policy);
    let tier = opts.rigor.parse::<TierId>().unwrap_or(TierId::T1);
    let behavioral = opts
        .behavioral_report
        .clone()
        .unwrap_or_else(|| dir.join(".spec-engine").join("behavioral.json"));
    let evidence_dir = opts
        .evidence_dir
        .clone()
        .unwrap_or_else(|| dir.join("evidence"));
    let binding = EvidenceBinding::current(dir).ok();
    let report = match &binding {
        Some(binding) => evaluate_bound(&graph, &evidence_dir, tier, &behavioral, binding),
        None => evaluate(&graph, &evidence_dir, tier, &behavioral),
    };
    let mut details: Vec<String> = Vec::new();
    let has = |v: &Validator| opts.validators.contains(v);
    let mut passed = true;

    let total_requirements = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Req)
        .count();
    let must_broken = graph
        .violations
        .iter()
        .filter(|v| v.rule_id == "COV-01")
        .count();
    let should_pending = graph
        .violations
        .iter()
        .filter(|v| v.rule_id == "COV-02")
        .count();
    let draft_total = tags
        .iter()
        .filter(|tag| {
            matches!(&tag.kind, TagKind::Req { priority, .. } if *priority == Priority::Draft)
        })
        .count();
    for violation in &graph.violations {
        details.push(format!(
            "{}: {} (found {}, required {})",
            violation.rule_id, violation.subject, violation.found, violation.required
        ));
    }
    if opts.deny_drafts && draft_total > 0 {
        details.push(format!("--deny-drafts: {draft_total} active DRAFT requirement(s)"));
    }
    if has(&Validator::Traceability) {
        passed &= graph.is_closed && !(opts.strict && should_pending > 0);
    }
    if has(&Validator::Verification) {
        passed &= report.status == VerificationStatus::Pass;
    }
    if opts.deny_drafts && draft_total > 0 {
        passed = false;
    }

    let doc_contract_violations = if has(&Validator::DocContracts) {
        let count = audit_project_docstrings(dir).map_err(|e| e.to_string())?.len();
        passed &= tier == TierId::T0 || count == 0;
        count
    } else {
        0
    };
    let ambient_violations = if has(&Validator::AmbientSkills) && dir.join("skills").is_dir() {
        let found = audit_ambient_skills(dir);
        for violation in &found {
            details.push(format!("Ambient Skills Violations: {violation}"));
        }
        let count = found.len();
        passed &= tier == TierId::T0 || count == 0;
        count
    } else {
        0
    };
    let mut anti_slop_warnings = 0usize;
    let anti_slop_errors = if has(&Validator::AntiSlop) {
        match audit_project_anti_slop(dir) {
            Ok(violations) => {
                for violation in &violations {
                    details.push(format!("Anti-slop: {}", violation.message));
                }
                let errors = violations
                    .iter()
                    .filter(|v| v.severity == Severity::Error)
                    .count();
                let warnings = violations
                    .iter()
                    .filter(|v| v.severity == Severity::Warning)
                    .count();
                passed &= tier == TierId::T0 || errors == 0;
                if opts.strict && warnings > 0 {
                    passed = false;
                }
                anti_slop_warnings = warnings;
                errors
            }
            Err(_) => {
                passed = false;
                usize::MAX
            }
        }
    } else {
        0
    };
    let skill_drifts = if has(&Validator::SkillDrift) {
        let dirs = [
            dir.join(".opencode").join("skills"),
            dir.join(".agents").join("plugins").join("asep").join("skills"),
        ];
        let count = dirs.iter().flat_map(|d| crate::skill::inspector::detect_skill_drift(d)).count();
        passed &= count == 0;
        count
    } else {
        0
    };

    let verdict = if passed {
        Verdict::Pass
    } else if report.status == VerificationStatus::Unproven {
        Verdict::Unproven
    } else {
        Verdict::Fail
    };
    Ok(CheckOutcome {
        passed,
        verdict,
        status: report.status,
        structural: graph.is_closed,
        behavioral: report.behavioral,
        semantic: report.semantic,
        total_requirements,
        must_broken,
        should_pending,
        draft_total,
        doc_contract_violations,
        ambient_violations,
        anti_slop_errors,
        anti_slop_warnings,
        skill_drifts,
        denied_drafts: opts.deny_drafts && draft_total > 0,
        validators: opts.validators.iter().map(|v| format!("{v:?}")).collect(),
        skipped: report.skipped,
        details,
    })
}

pub fn run_ci_pipeline(dir: &Path, opts: &CiOptions) -> CiReport {
    let start = Instant::now();
    let mut gates = Vec::new();
    let mut overall_success = true;
    let mut unproven = false;

    // Benchmarks are explicitly deferred. Gate 4 is never a default side
    // effect and is unavailable in this milestone, even when explicitly
    // requested, so no benchmark (including `--no-run`) can run silently.
    let target_gates = ci_target_gates(opts);

    for &g in &target_gates {
        if opts.skip.contains(&g) {
            gates.push(GateResult {
                gate_number: g,
                name: gate_name(g).to_string(),
                passed: true,
                skipped: true,
                unproven: false,
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
            4 => GateResult {
                gate_number: 4,
                name: gate_name(4).to_string(),
                passed: false,
                skipped: true,
                unproven: false,
                duration_ms: 0,
                details: vec![
                    "Benchmark execution is deferred for this milestone".to_string(),
                    "No benchmark or noise-floor evidence can be produced or claimed".to_string(),
                ],
            },
            _ => GateResult {
                gate_number: g,
                name: "Unknown Gate".to_string(),
                passed: false,
                skipped: false,
                unproven: false,
                duration_ms: 0,
                details: vec![format!("Gate {g} does not exist (valid: 0..4)")],
            },
        };

        if g == 1 && !res.passed && !res.skipped {
            // Gate 1 unproven (missing semantic evidence) propagates exit 3.
            unproven = res.unproven;
        }

        if !res.passed && !res.skipped {
            overall_success = false;
        }

        let mut final_res = res;
        if opts.gate.is_some() && final_res.skipped {
            final_res.passed = false;
            final_res.unproven = true;
            final_res
                .details
                .push("Requested gate was unavailable; result is unproven".to_string());
            unproven = true;
        }
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
        4 => "Gate 4: Performance & Noise-Floor Budget Gate",
        _ => "Unknown Gate",
    }
}

const REQUIRED_FORMAL_TOOLS: [&str; 4] = ["alloy", "tlc", "spectral", "dbml"];

fn run_gate_0(dir: &Path) -> GateResult {
    let formal_checks = validate_formal_specs(dir);
    let state = aggregate_formal_state(&formal_checks, &REQUIRED_FORMAL_TOOLS);
    let mut details = Vec::new();

    for check in &formal_checks {
        let required = is_required_formal_check(check, &REQUIRED_FORMAL_TOOLS);
        let check_state = formal_state(check, required);
        details.push(match check_state {
            FormalState::Fail => {
                format!("[FAIL] {}: {} ({})", check.tool, check.file, check.message)
            }
            FormalState::Unproven => format!(
                "[UNPROVEN] {}: {} ({})",
                check.tool, check.file, check.message
            ),
            FormalState::Skipped => {
                format!("[SKIP] {}: {} ({})", check.tool, check.file, check.message)
            }
            FormalState::Pass => format!("[PASS] {}: {}", check.tool, check.file),
        });
    }

    if details.is_empty() {
        details
            .push("No formal specification models (.als, .tla, .dbml, api.yaml) found".to_string());
    }

    GateResult {
        gate_number: 0,
        name: gate_name(0).to_string(),
        passed: matches!(state, FormalState::Pass | FormalState::Skipped),
        skipped: state == FormalState::Skipped,
        unproven: false,
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
            let anti_slop_violations = match audit_project_anti_slop(dir) {
                Ok(violations) => violations,
                Err(e) => {
                    details.push(format!("[UNPROVEN] anti-slop audit unavailable: {e}"));
                    return GateResult {
                        gate_number: 1,
                        name: gate_name(1).to_string(),
                        passed: false,
                        skipped: false,
                        unproven: true,
                        duration_ms: 0,
                        details,
                    };
                }
            };
            let anti_errors: Vec<_> = anti_slop_violations
                .iter()
                .filter(|v| v.severity == Severity::Error)
                .collect();
            let anti_warnings: Vec<_> = anti_slop_violations
                .iter()
                .filter(|v| v.severity == Severity::Warning)
                .collect();
            let has_antislop_failure = tier != TierId::T0
                && (!anti_errors.is_empty() || (strict && !anti_warnings.is_empty()));

            // COV-02 is a warning; `--strict` promotes it to failure.
            let has_cov02 = graph.violations.iter().any(|v| v.rule_id == "COV-02");
            let passed = graph.is_closed && !(strict && has_cov02) && !has_antislop_failure;

            if !passed && strict && has_cov02 {
                details
                    .push("COV-02 coverage warnings promoted to failures by --strict".to_string());
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
                details.push(format!(
                    "Closed MUST gates: structural closed, {} violations",
                    graph.violations.len()
                ));
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
            let binding = EvidenceBinding::current(dir).ok();
            let report = match &binding {
                Some(binding) => evaluate_bound(&graph, &evidence_dir, tier, &behavioral, binding),
                None => evaluate(&graph, &evidence_dir, tier, &behavioral),
            };
            let mut gate_unproven = false;
            if report.status == crate::verification::VerificationStatus::Unproven {
                details.push("Gate 1 unproven: semantic evidence required but missing".to_string());
                gate_unproven = true;
            }

            GateResult {
                gate_number: 1,
                name: gate_name(1).to_string(),
                passed,
                skipped: false,
                unproven: gate_unproven,
                duration_ms: 0,
                details,
            }
        }
        Err(e) => GateResult {
            gate_number: 1,
            name: gate_name(1).to_string(),
            passed: false,
            skipped: false,
            unproven: false,
            duration_ms: 0,
            details: vec![format!("Scanner failed: {e}")],
        },
    }
}

fn run_gate_2(dir: &Path) -> GateResult {
    if !dir.join("Cargo.toml").exists() {
        return GateResult {
            gate_number: 2,
            name: gate_name(2).to_string(),
            passed: true,
            skipped: true,
            unproven: false,
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
                unproven: false,
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
            unproven: false,
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
            unproven: false,
            duration_ms: 0,
            details: vec!["No Cargo.toml found in directory; tests skipped".to_string()],
        };
    }

    let out = Command::new("cargo").arg("test").current_dir(dir).output();

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
            }))
            .unwrap_or_default(),
        )
    });

    GateResult {
        gate_number: 3,
        name: gate_name(3).to_string(),
        passed,
        skipped: false,
        unproven: false,
        duration_ms: 0,
        details: vec![match passed {
            true => "Test suite passed completely".to_string(),
            false => "Test suite failed".to_string(),
        }],
    }
}

/// Benchmark gate is intentionally inert. Kept only so callers that still
/// reference the symbol get an honest unproven result; it never starts Cargo.
#[allow(dead_code)]
fn benchmark_gate_unavailable() -> GateResult {
    GateResult {
        gate_number: 4,
        name: gate_name(4).to_string(),
        passed: false,
        skipped: true,
        unproven: false,
        duration_ms: 0,
        details: vec!["Benchmark execution is deferred for this milestone".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_0_absent_required_solver_fails_gate() {
        let tmp = std::env::temp_dir().join(format!("asep-gate0-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("spec.tla"),
            "---- MODULE Spec ----\nVARIABLES x\nInit == x = 0\nNext == x' = x + 1\nSpec == Init /\\ [][Next]_x\n====\n",
        )
        .unwrap();
        let res = run_gate_0(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            !res.passed,
            "absent required solver must not pass gate 0: {:?}",
            res.details
        );
        assert!(!res.skipped, "unproven check must not mark gate 0 skipped");
        assert!(
            res.details
                .iter()
                .any(|d| d.contains("UNPROVEN") || d.contains("tlc")),
            "expected UNPROVEN/tlc detail, got {:?}",
            res.details
        );
    }

    #[test]
    fn gate_0_skeleton_absent_solver_is_skipped_and_passed() {
        let tmp = std::env::temp_dir().join(format!("asep-gate0-skel-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("skeletons")).unwrap();
        std::fs::create_dir_all(tmp.join("bin")).unwrap();
        std::fs::write(
            tmp.join("skeletons").join("INVARIANT_SKELETON.als"),
            "module skeleton {}\n",
        )
        .unwrap();
        let saved_path = std::env::var_os("PATH");
        std::env::set_var("PATH", tmp.join("bin"));
        let res = run_gate_0(&tmp);
        match saved_path {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(res.passed, "skeleton gate 0 must pass: {:?}", res.details);
        assert!(res.skipped, "skeleton gate 0 must be skipped");
        assert!(
            res.details
                .iter()
                .any(|d| d.contains("[SKIP]") && d.contains("SKELETON")),
            "expected SKIP detail for skeleton, got {:?}",
            res.details
        );
    }

    #[test]
    fn gate_0_empty_dir_is_skipped_and_passed() {
        let tmp = std::env::temp_dir().join(format!("asep-gate0-empty-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let res = run_gate_0(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(res.passed, "empty gate 0 must pass: {:?}", res.details);
        assert!(res.skipped, "empty gate 0 must be skipped");
    }

    #[test]
    // @test:R05
    fn benchmark_gate_is_not_a_default_side_effect() {
        let opts = CiOptions {
            gate: None,
            skip: Vec::new(),
            rigor: "T1".to_string(),
        };
        assert!(!ci_target_gates(&opts).contains(&4));
    }

    #[test]
    // @test:R05
    fn explicit_benchmark_gate_returns_unproven_without_running_cargo() {
        let dir = tempfile::tempdir().unwrap();
        let opts = CiOptions {
            gate: Some(4),
            skip: Vec::new(),
            rigor: "T1".to_string(),
        };
        let report = run_ci_pipeline(dir.path(), &opts);
        let gate = &report.gates[0];
        assert!(gate.skipped && !gate.passed);
        assert!(gate.details.iter().any(|d| d.contains("deferred")));
    }
}
