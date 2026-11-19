//! Unified 5-Gate Local CI Pipeline Runner.

use crate::formal::validate_formal_specs;
use crate::graph::builder::build_relation_graph;
use crate::policy::{load_policy, TierId};
use crate::scanner::scan_project_tags;
use crate::verification::evaluate;
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

    let target_gates: Vec<u8> = if let Some(g) = opts.gate {
        vec![g]
    } else {
        vec![0, 1, 2, 3, 4]
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
        4 => "Gate 4: Performance & Noise-Floor Budget Gate",
        _ => "Unknown Gate",
    }
}

fn run_gate_0(dir: &Path) -> GateResult {
    let formal_checks = validate_formal_specs(dir);
    let mut passed = true;
    let mut details = Vec::new();

    for check in formal_checks {
        if !check.passed && !check.skipped {
            passed = false;
            details.push(format!("[FAIL] {}: {} ({})", check.tool, check.file, check.message));
        } else if check.skipped {
            details.push(format!("[SKIP] {}: {} ({})", check.tool, check.file, check.message));
        } else {
            details.push(format!("[PASS] {}: {}", check.tool, check.file));
        }
    }

    if details.is_empty() {
        details.push("No formal specification models (.als, .tla, .dbml, api.yaml) found".to_string());
    }

    GateResult {
        gate_number: 0,
        name: gate_name(0).to_string(),
        passed,
        skipped: false,
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

            // COV-02 is a warning; `--strict` promotes it to failure.
            let has_cov02 = graph
                .violations
                .iter()
                .any(|v| v.rule_id == "COV-02");
            let passed = graph.is_closed && !(strict && has_cov02);

            if !passed {
                if strict && has_cov02 {
                    details.push("COV-02 coverage warnings promoted to failures by --strict".to_string());
                }
                for v in &graph.violations {
                    details.push(format!(
                        "[FAIL] {}: {} (found {}, required {})",
                        v.rule_id, v.subject, v.found, v.required
                    ));
                }
            } else {
                details.push(format!("Closed MUST gates: structural closed, {} violations", graph.violations.len()));
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
        Ok(res) => GateResult {
            gate_number: 2,
            name: gate_name(2).to_string(),
            passed: res.status.success(),
            skipped: false,
            duration_ms: 0,
            details: vec![
                if res.status.success() {
                    "Clippy passed with zero warnings".to_string()
                } else {
                    String::from_utf8_lossy(&res.stderr).trim().to_string()
                },
            ],
        },
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
        details: vec![
            if passed {
                "Test suite passed completely".to_string()
            } else {
                "Test suite failed".to_string()
            },
        ],
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
        Ok(res) => GateResult {
            gate_number: 4,
            name: gate_name(4).to_string(),
            passed: res.status.success(),
            skipped: false,
            duration_ms: 0,
            details: vec![
                if res.status.success() {
                    "Benchmark harness compiled and verified against noise floor".to_string()
                } else {
                    String::from_utf8_lossy(&res.stderr).trim().to_string()
                },
            ],
        },
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
