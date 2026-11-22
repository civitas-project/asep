use crate::adr::create_new_adr;
use crate::affected::affected;
use crate::auditor::{audit_project_anti_slop, audit_project_docstrings};
use crate::cli::{AdrCommands, Cli, Commands, HarnessCommands, SkillCommands};
use crate::diagram::render_plantuml_diagrams;
use crate::graph::builder::{build_relation_graph, build_trace_graph};
use crate::harness::process::parse_size_bytes;
use crate::init::init_asep_project;
use crate::mcp::server::run_mcp_server;
use crate::pack::installer::{run_pack_installer, PackOptions};
use crate::pack::targets::{default_global_home, InstallScope, TargetAgent};
use crate::policy::{load_policy, Severity, TierId};
use crate::skill::inspector::detect_skill_drift;
use crate::config::EngineConfig;
use crate::scanner::scan_project_tags;
use crate::scanner::ScannedTag;
use crate::verification::{evaluate, VerificationStatus};
use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Serialize)]
struct CheckJsonReport {
    passed: bool,
    rigor: String,
    total_requirements: usize,
    must_satisfied: usize,
    must_broken: usize,
    should_pending: usize,
    draft_total: usize,
    draft_pending: usize,
    doc_contract_violations: usize,
    traceability: crate::graph::model::TraceabilityReport,
    violations: Vec<crate::auditor::DocContractViolation>,
    anti_slop_violations: usize,
    anti_slop_warnings: usize,
}

/// Resolves the rigor tier for a check run.
///
/// Precedence: `--rigor` CLI flag wins; otherwise `@rigor:T<n>` tag wins;
/// otherwise `config.rigor` (parsed via `TierId::from_str`); otherwise `T1`
/// for code scans or `T0` when the scan finds only documentation files.
pub fn resolve_rigor(cli_rigor: Option<&str>, tags: &[ScannedTag], config: &EngineConfig) -> TierId {
    // CLI override has highest precedence.
    let cli_tier = cli_rigor.and_then(|r| r.parse::<TierId>().ok());
    if let Some(t) = cli_tier {
        return t;
    }
    let has_code_file = tags.iter().any(|t| matches!(
        t.file_path.extension().and_then(|s| s.to_str()),
        Some("rs" | "go" | "ts" | "js" | "py" | "cpp" | "c" | "h" | "hpp" | "java" | "cs")
    ));

    #[allow(clippy::match_bool)]
    // `match bool` replaces `if/else` to keep zero `else` tokens (AS-CODE-ELSE).
    let default_tier = match has_code_file {
        true => TierId::T1,  // code files present → T1
        false => TierId::T0, // documentation-only → T0
    };

    // @rigor:T<n> tag wins.
    for tag in tags {
        if let crate::scanner::lexer::TagKind::Rigor { tier } = &tag.kind {
            if let Ok(t) = tier.parse::<TierId>() {
                return t;
            }
        }
    }

    // Config value.
    let config_tier = config.rigor.as_deref().and_then(|r| r.parse::<TierId>().ok());
    if let Some(t) = config_tier {
        return t;
    }

    default_tier
}

// @impl:FR-CLI-SCAN-01-B
// @impl:FR-CLI-CHECK-01 @impl:FR-CLI-02
// @impl:FR-CLI-CHECK-02
pub fn run() -> ExitCode {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Check {
            dir,
            path,
            strict,
            messages,
            behavioral_report,
            deny_drafts,
            watch,
            coverage,
            rigor,
            skills,
        }) => {
            let target_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));

            // --skills: print only the detailed skill drift report and exit.
            if skills {
                return run_skill_drift_report(&target_path);
            }

            let tags = match scan_project_tags(&target_path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{} Failed to scan project tags: {e}", "✖".red());
                    return e.exit_code();
                }
            };
            let config = crate::config::load_config(&target_path);
            let tier = resolve_rigor(rigor.as_deref(), &tags, &config);

            if watch {
                return match crate::watch::start_watch_mode(&target_path, tier, coverage.as_deref()) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{} Watch mode failed: {e}", "✖".red());
                        ExitCode::from(1)
                    }
                };
            }

            if args.verbose {
                println!(
                    "{} scanning {} (strict: {}, deny-drafts: {})",
                    "ASEP".bold().cyan(),
                    target_path.display(),
                    strict,
                    deny_drafts
                );
            }

            let policy = load_policy();
            let graph = build_relation_graph(&tags, &policy);
            let report = build_trace_graph(&tags);

            let evidence_dir = target_path.join("evidence");
            let behavioral_path = behavioral_report
                .unwrap_or_else(|| target_path.join(".spec-engine").join("behavioral.json"));
            let verification_report = evaluate(&graph, &evidence_dir, tier, &behavioral_path);

            // Write `.spec-engine/verification.json` for tooling and later `--rigor` wiring.
            let spec_engine_dir = target_path.join(".spec-engine");
            let verification_out = spec_engine_dir.join("verification.json");
            let json = match serde_json::to_string_pretty(&verification_report) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("{} Failed to serialize verification report: {e}", "✖".red());
                    return ExitCode::from(2);
                }
            };
            if let Err(e) = fs::create_dir_all(&spec_engine_dir).and_then(|_| fs::write(&verification_out, &json)) {
                eprintln!("{} Failed to write verification report: {e}", "✖".red());
                return ExitCode::from(2);
            }

            let violations = match audit_project_docstrings(&target_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{} Failed to audit docstring contracts: {e}", "✖".red());
                    return e.exit_code();
                }
            };

            // Anti-slop audit: Error violations block outside T0; Warning
            // violations block only under --strict.
            let anti_slop_violations = match audit_project_anti_slop(&target_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{} Failed to audit anti-slop: {e}", "✖".red());
                    return e.exit_code();
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
            let has_antislop_failure =
                tier != TierId::T0 && (!anti_errors.is_empty() || (strict && !anti_warnings.is_empty()));

            // Structural verification: installed skill copies must match embedded canonicals.
            let skill_drift_dirs = [
                target_path.join(".opencode").join("skills"),
                target_path.join(".agents").join("plugins").join("asep").join("skills"),
            ];
            let skill_drifts: Vec<_> = skill_drift_dirs
                .iter()
    .flat_map(|d| detect_skill_drift(d))
    .collect();
            let has_skill_drift = !skill_drifts.is_empty();

            let has_must_failure = !report.is_closed;
            let has_doc_contract_failure = tier != TierId::T0 && !violations.is_empty();
            let has_strict_failure = strict && !report.should_pending.is_empty();
            let has_deny_drafts_failure = deny_drafts && report.draft_total > 0;
            let mut passed = !has_must_failure
                && !has_doc_contract_failure
                && !has_strict_failure
                && !has_deny_drafts_failure
                && !has_skill_drift
                && !has_antislop_failure;

            // --messages: validate agent message directory; errors fold into `passed`.
            if let Some(messages_dir) = &messages {
                match crate::agent_protocol::validate_dir(messages_dir) {
                    Ok(()) => {}
                    Err(errors) => {
                        for e in &errors {
                            eprintln!("{} {}", "✖".red(), e);
                        }
                        passed = false;
                    }
                }
            }

            #[allow(clippy::match_bool)]
            // `match bool` replaces `if/else` so the file keeps zero `else` tokens (AS-CODE-ELSE).
            match args.json {
                true => {
                    let json_output = CheckJsonReport {
                        passed,
                        rigor: tier.as_str().to_string(),
                        total_requirements: report.total_requirements,
                        must_satisfied: report.must_satisfied,
                        must_broken: report.must_broken.len(),
                        should_pending: report.should_pending.len(),
                        draft_total: report.draft_total,
                        draft_pending: report.draft_pending.len(),
                        doc_contract_violations: violations.len(),
                        anti_slop_violations: anti_slop_violations.len(),
                        anti_slop_warnings: anti_warnings.len(),
                        traceability: report,
                        violations,
                    };

                    match serde_json::to_string_pretty(&json_output) {
                        Ok(json_str) => println!("{json_str}"),
                        Err(e) => {
                            eprintln!("{} JSON serialization failed: {e}", "✖".red());
                            return ExitCode::from(2);
                        }
                    }
                }
                false => {
                    println!(
                        "{}",
                        "============================================================"
                            .bold()
                            .cyan()
                    );
                    println!(
                        "{}",
                        "              ASEP SDLC Traceability Report                 "
                            .bold()
                            .cyan()
                    );
                    println!(
                        "{}",
                        "============================================================"
                            .bold()
                            .cyan()
                    );

                    #[allow(clippy::match_bool)]
                    // `match bool` replaces `if/else` so the file keeps zero `else` tokens (AS-CODE-ELSE).
                    match report.total_requirements == 0 {
                        true => {
                            println!(
                                "{}",
                                "No @req tags discovered in target repository.".yellow()
                            );
                        }
                        false => {
                            println!(
                                "Discovered {} requirements across workspace.",
                                report.total_requirements.to_string().bold()
                            );
                        }
                    }

                    if !report.must_broken.is_empty() {
                        println!("\n{}", "❌ Broken MUST Traceability Gates:".red().bold());
                        for broken in &report.must_broken {
                            let missing = [
                                (broken.missing_design, "@design"),
                                (broken.missing_impl, "@impl"),
                                (broken.missing_test, "@test"),
                            ]
                            .into_iter()
                            .filter_map(|(cond, label)| cond.then_some(label))
                            .collect::<Vec<_>>();
                            println!(
                                "   - {} ({}:{}) lacks: {}",
                                broken.id.bold().red(),
                                broken.file_path.display(),
                                broken.line_number,
                                missing.join(", ").yellow()
                            );
                        }
                    }

                    if !report.should_pending.is_empty() {
                        println!("\n{}", "⚠️ Pending SHOULD Requirements:".yellow().bold());
                        for pending in &report.should_pending {
                            println!(
                                "   - {} ({}:{})",
                                pending.id.bold().yellow(),
                                pending.file_path.display(),
                                pending.line_number
                            );
                        }
                    }

                    if report.draft_total > 0 {
                        println!("\n{}", "📝 Active DRAFT Requirements (T0 Mode):".cyan().bold());
                        for draft in &report.draft_pending {
                            let missing: Vec<&str> = [
                                (draft.missing_design, "@design"),
                                (draft.missing_impl, "@impl"),
                                (draft.missing_test, "@test"),
                            ]
                            .into_iter()
                            .filter_map(|(cond, label)| cond.then_some(label))
                            .collect();
                            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                            let missing_display = match missing.is_empty() {
                                true => "none".to_string(),
                                false => missing.join(", ").yellow().to_string(),
                            };
                            println!(
                                "   - {} ({}:{}) in-progress (missing: {})",
                                draft.id.bold().cyan(),
                                draft.file_path.display(),
                                draft.line_number,
                                missing_display,
                            );
                        }
                    }
                    if report.draft_total > 0 && report.draft_pending.is_empty() {
                        println!(
                            "   All {} DRAFT requirements have full design, impl, and test scaffolds.",
                            report.draft_total
                        );
                    }

                    if has_deny_drafts_failure {
                        println!(
                            "\n{}",
                            "❌ Production Policy Violation (--deny-drafts):".red().bold()
                        );
                        println!(
                            "   Discovered {} active @req:*:DRAFT tag(s). Graduate all drafts to MUST before merging into main trunk.",
                            report.draft_total
                        );
                    }

                    if !violations.is_empty() {
                        #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                        let header = match tier == TierId::T0 {
                            true => "⚠️ Docstring Contract Warnings (Non-blocking in T0 Mode):".yellow().bold(),
                            false => "❌ Docstring Contract Violations:".red().bold(),
                        };
                        println!("\n{header}");
                        for v in &violations {
                            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                            let message = match tier == TierId::T0 {
                                true => v.message.yellow(),
                                false => v.message.red(),
                            };
                            println!("   - {}:{}: {}", v.file_path.display(), v.line_number, message);
                        }
                    }

                    if !anti_errors.is_empty() {
                        #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                        let header = match tier == TierId::T0 {
                            true => "⚠️ Anti-slop Errors (Non-blocking in T0 Mode):".yellow().bold(),
                            false => "❌ Anti-slop Violations:".red().bold(),
                        };
                        println!("\n{header}");
                        for v in &anti_errors {
                            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                            let message = match tier == TierId::T0 {
                                true => v.message.yellow(),
                                false => v.message.red(),
                            };
                            println!("   - {}:{}: {}", v.file_path.display(), v.line_number, message);
                        }
                    }

                    if !anti_warnings.is_empty() && strict {
                        println!("\n{}", "❌ Anti-slop Warnings (--strict):".red().bold());
                        for v in &anti_warnings {
                            println!(
                                "   - {}:{}: {}",
                                v.file_path.display(),
                                v.line_number,
                                v.message.red()
                            );
                        }
                    }
                    if !anti_warnings.is_empty() && !strict {
                        eprintln!("{}", "⚠️ Anti-slop Warnings (reported, non-blocking):".yellow().bold());
                        for v in &anti_warnings {
                            eprintln!(
                                "   - {}:{}: {}",
                                v.file_path.display(),
                                v.line_number,
                                v.message.yellow()
                            );
                        }
                    }

                    if !skill_drifts.is_empty() {
                        println!("\n{}", "❌ Skill Drift Detected:".red().bold());
                        println!("   Installed skill copies differ from embedded canonicals:");
                        for d in &skill_drifts {
                            println!(
                                "   - {} [{}] {}",
                                d.skill_id.bold(),
                                d.kind.red(),
                                d.path.display()
                            );
                        }
                        println!(
                            "   Restore canonical copies with 'asep skill restore --all' or re-run the pack installer.",
                        );
                    }

                    if let Some(ref cov_path) = coverage {
                        match crate::coverage::parse_lcov_file(cov_path) {
                            Ok(cov_map) => {
                                let cov_rep = crate::coverage::map_coverage_to_requirements(&tags, &cov_map, 80.0);
                                println!("\n{}", "🎯 Requirement Traceability Test Coverage:".bold().cyan());
                                for item in &cov_rep.items {
                                    #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                                    let badge = match item.passed {
                                        true => format!("{:.1}% PASS", item.percentage).green(),
                                        false => format!("{:.1}% FAIL (< 80%)", item.percentage).red(),
                                    };
                                    println!(
                                        "   - {}: {} ({}/{} impl lines covered)",
                                        item.req_id.bold(),
                                        badge,
                                        item.covered_lines,
                                        item.total_lines
                                    );
                                }
                                passed = passed && (cov_rep.all_passed || tier == TierId::T0);
                            }
                            Err(e) => {
                                eprintln!("{} Failed to parse coverage file {}: {e}", "⚠".yellow(), cov_path.display());
                            }
                        }
                    }

                    println!(
                        "{}",
                        "------------------------------------------------------------".dimmed()
                    );
                    println!(
                        "Summary (rigor: {}): {} closed MUST | {} broken MUST | {} pending SHOULD | {} DRAFT | {} docstring contract violations",
                        tier.as_str().cyan().bold(),
                        report.must_satisfied.to_string().green().bold(),
                        report.must_broken.len().to_string().red().bold(),
                        report.should_pending.len().to_string().yellow().bold(),
                        report.draft_total.to_string().cyan().bold(),
                        violations.len().to_string().red().bold(),
                    );

                    #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                    match passed {
                        true => println!("\n{}", "✔ Gate Result: PASSED (Exit Code 0)".green().bold()),
                        false => println!("\n{}", "✖ Gate Result: FAILED (Exit Code 1)".red().bold()),
                    }

                    println!(
                        "Verification: {} | structural={} behavioral={} semantic={} skipped={:?}",
                        match verification_report.status {
                            VerificationStatus::Pass => "PASS".green(),
                            VerificationStatus::Fail => "FAIL".red(),
                            VerificationStatus::Unproven => "UNPROVEN".yellow(),
                        },
                        verification_report.structural,
                        verification_report.behavioral,
                        verification_report.semantic,
                        verification_report.skipped
                    );
                }
            }

            let exit_codes = policy.exit_codes();
            match verification_report.status {
                VerificationStatus::Unproven => ExitCode::from(exit_codes.unproven),
                VerificationStatus::Fail => ExitCode::from(exit_codes.fail),
                VerificationStatus::Pass => {
                    #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                    match passed {
                        true => ExitCode::from(exit_codes.pass),
                        false => ExitCode::from(exit_codes.fail),
                    }
                }
            }
        }

        Some(Commands::Affected {
            paths,
            stdin,
            root,
        }) => {
            let root_path = root.unwrap_or_else(|| PathBuf::from("."));

            let mut changed: Vec<PathBuf> = paths;
            if stdin {
                let stdin = io::stdin();
                for line in stdin.lock().lines() {
                    let line = match line {
                        Ok(line) => line,
                        Err(e) => {
                            eprintln!("{} Failed to read stdin: {e}", "✖".red());
                            return ExitCode::from(1);
                        }
                    };
                    let trimmed = line.trim();
                    changed.push(PathBuf::from(trimmed));
                }
                changed.retain(|p| !p.as_os_str().is_empty());
            }

            let report = match affected(&root_path, &changed) {
                Ok(report) => report,
                Err(e) => {
                    eprintln!("{} {}", "✖".red(), e);
                    return ExitCode::from(1);
                }
            };

            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(e) => {
                        eprintln!(
                            "{} Failed to serialize affected report: {e}",
                            "✖".red()
                        );
                        return ExitCode::from(2);
                    }
                }
                return ExitCode::SUCCESS;
            }

            println!(
                "{} {}",
                "ASEP Affected Impact Analysis".bold().cyan(),
                root_path.display()
            );
            println!("Changed paths: {}", changed.len().to_string().bold());

            println!("\n{}", "Requirements:".bold());
            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
            match report.requirements.is_empty() {
                true => println!("  (none)"),
                false => {
                    for req in &report.requirements {
                        println!(
                            "  - {} ({}:{})",
                            req.id.bold(),
                            req.file.display(),
                            req.line
                        );
                    }
                }
            }

            println!("\n{}", "Tests:".bold());
            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
            match report.tests.is_empty() {
                true => println!("  (none)"),
                false => {
                    for test in &report.tests {
                        match &test.function {
                            Some(function) => {
                                println!(
                                    "  - {}:{} :: {}",
                                    test.file.display(),
                                    test.line,
                                    function.bold()
                                );
                            }
                            None => {
                                println!("  - {}:{}", test.file.display(), test.line);
                            }
                        }
                    }
                }
            }

            println!("\n{}", "Verification commands:".bold());
            #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
            match report.commands.is_empty() {
                true => println!("  (none)"),
                false => {
                    for command in &report.commands {
                        println!("  {command}");
                    }
                }
            }

            ExitCode::SUCCESS
        }

        Some(Commands::Adr { sub }) => match sub {
            AdrCommands::New { title, dir, path } => {
                let target_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));
                match create_new_adr(&target_path, &title) {
                    Ok(adr_path) => {
                        println!("{} Created ADR: {}", "✔".green(), adr_path.display());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{} Failed to scaffold ADR: {e}", "✖".red());
                        e.exit_code()
                    }
                }
            }
        },

        Some(Commands::Ci {
            dir,
            path,
            gate,
            skip,
            rigor,
        }) => {
            let target_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));
            let parsed_skip = skip
                .map(|s| {
                    s.split(',')
                        .filter_map(|x| x.trim().parse::<u8>().ok())
                        .collect::<Vec<u8>>()
                })
                .unwrap_or_default();

            let opts = crate::ci::CiOptions {
                gate,
                skip: parsed_skip,
                rigor: rigor.unwrap_or_else(|| "T1".to_string()),
            };

            let report = crate::ci::run_ci_pipeline(&target_path, &opts);

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_default()
                );
            }
            if !args.json {
                println!("\n{}", "=== 5-Gate Local CI Verification Pipeline ===".bold());
                for g in &report.gates {
                    let status_badge = match (g.skipped, g.passed) {
                        (true, _) => "⊘ SKIPPED".yellow(),
                        (_, true) => "✔ PASSED".green(),
                        (false, false) => "✖ FAILED".red(),
                    };
                    println!(
                        "[{}] {} ({}ms)",
                        status_badge,
                        g.name.bold(),
                        g.duration_ms
                    );
                    for d in &g.details {
                        println!("     {}", d);
                    }
                }
                println!("--------------------------------------------------");
                #[allow(clippy::match_bool)] // guard-clause policy requires matching on a bool
                match report.all_passed() {
                    true => {
                        println!(
                            "{} All CI gates satisfied in {}ms.",
                            "✔".green().bold(),
                            report.total_duration_ms
                        );
                    }
                    false => {
                        println!(
                            "{} CI pipeline failed in {}ms.",
                            "✖".red().bold(),
                            report.total_duration_ms
                        );
                    }
                }
            }

            match (report.unproven, report.all_passed()) {
                (true, _) => ExitCode::from(3),
                (false, true) => ExitCode::SUCCESS,
                (false, false) => ExitCode::from(1),
            }
        },

        Some(Commands::Render { dir, path }) => {
            let target_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));
            match render_plantuml_diagrams(&target_path) {
                Ok(count) => {
                    println!(
                        "{} Compiled {} PlantUML diagram(s) into SVG",
                        "✔".green(),
                        count
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{} Failed to render diagrams: {e}", "✖".red());
                    e.exit_code()
                }
            }
        }

        Some(Commands::Init { dir, path }) => {
            let target_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));
            match init_asep_project(&target_path) {
                Ok(_) => {
                    println!(
                        "{} Initialized canonical ASEP documentation tree at {}",
                        "✔".green(),
                        target_path.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{} Failed to initialize ASEP documentation: {e}", "✖".red());
                    e.exit_code()
                }
            }
        }

        Some(Commands::Pack {
            local,
            global,
            agent,
            yes,
            skills,
            dir,
            path,
            pos_dir,
        }) => {
            let target_path = path
                .or(dir)
                .or(pos_dir)
                .unwrap_or_else(|| PathBuf::from("."));

            let scope = match (local, global) {
                (true, _) => Some(InstallScope::Local(target_path.clone())),
                (false, true) => Some(InstallScope::Global(default_global_home())),
                (false, false) => None,
            };

            let mut agents = Vec::new();
            for item in agent.iter().flat_map(|s| s.split(',')) {
                    let trimmed = item.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match trimmed.to_lowercase().as_str() {
                        "antigravity" | "agy" => {
                            if !agents.contains(&TargetAgent::Antigravity) {
                                agents.push(TargetAgent::Antigravity);
                            }
                        }
                        "opencode" => {
                            if !agents.contains(&TargetAgent::OpenCode) {
                                agents.push(TargetAgent::OpenCode);
                            }
                        }
                        "claude" | "claudecode" => {
                            if !agents.contains(&TargetAgent::ClaudeCode) {
                                agents.push(TargetAgent::ClaudeCode);
                            }
                        }
                        "cursor" => {
                            if !agents.contains(&TargetAgent::Cursor) {
                                agents.push(TargetAgent::Cursor);
                            }
                        }
                        "windsurf" | "codeium" => {
                            if !agents.contains(&TargetAgent::Windsurf) {
                                agents.push(TargetAgent::Windsurf);
                            }
                        }
                        "continue" => {
                            if !agents.contains(&TargetAgent::Continue) {
                                agents.push(TargetAgent::Continue);
                            }
                        }
                        "roo" | "roocode" | "cline" => {
                            if !agents.contains(&TargetAgent::RooCode) {
                                agents.push(TargetAgent::RooCode);
                            }
                        }
                        "copilot" | "github-copilot" => {
                            if !agents.contains(&TargetAgent::Copilot) {
                                agents.push(TargetAgent::Copilot);
                            }
                        }
                        "goose" => {
                            if !agents.contains(&TargetAgent::Goose) {
                                agents.push(TargetAgent::Goose);
                            }
                        }
                        "zed" => {
                            if !agents.contains(&TargetAgent::Zed) {
                                agents.push(TargetAgent::Zed);
                            }
                        }
                        "aider" => {
                            if !agents.contains(&TargetAgent::Aider) {
                                agents.push(TargetAgent::Aider);
                            }
                        }
                        "detected" => {
                            let current_scope = scope
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| InstallScope::Local(target_path.clone()));
                            for a in TargetAgent::ALL {
                                if a.is_detected(&current_scope) && !agents.contains(&a) {
                                    agents.push(a);
                                }
                            }
                            if agents.is_empty() {
                                agents.push(TargetAgent::Antigravity);
                                agents.push(TargetAgent::OpenCode);
                            }
                        }
                        "all" => {
                            for a in TargetAgent::ALL {
                                if !agents.contains(&a) {
                                    agents.push(a);
                                }
                            }
                        }
                        other => {
                            eprintln!("{} Unknown target agent: {}", "✖".red(), other);
                            return ExitCode::from(1);
                        }
                    }
                }

            let opts = PackOptions {
                scope,
                agents,
                non_interactive: yes,
                target_dir: Some(target_path),
                skills_filter: skills,
            };

            match run_pack_installer(opts) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{} Pack installation failed: {e}", "✖".red());
                    ExitCode::from(1)
                }
            }
        }

        Some(Commands::Mcp { dir, path, pos_dir }) => {
            let target_path = path
                .or(dir)
                .or(pos_dir)
                .unwrap_or_else(|| PathBuf::from("."));
            match run_mcp_server(&target_path) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{} MCP server error: {e}", "✖".red());
                    ExitCode::from(1)
                }
            }
        }

        Some(Commands::Update {}) => {
            match crate::self_update::self_update() {
                Ok(tag) => {
                    println!("{} Updated to {}", "✔".green(), tag);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{} Self-update failed: {}", "✖".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        Some(Commands::Skill { sub, dir, path }) => {
            let base_path = path.or(dir).unwrap_or_else(|| PathBuf::from("."));
            let config_dir = base_path.clone();

            match sub {
                None => {
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&base_path);
                    match crate::skill::menu::run_interactive_skill_menu(&skills_dir, &config_dir) {
                        Ok(_) => ExitCode::SUCCESS,
                        Err(e) => {
                            eprintln!("{} Skill manager error: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::List { dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let skills = crate::skill::inspector::inspect_skills(&skills_dir);
                    crate::skill::menu::print_skills_table(&skills);
                    ExitCode::SUCCESS
                }
                Some(SkillCommands::Add {
                    id,
                    name,
                    description,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let display_name = name.unwrap_or_else(|| id.clone());
                    let desc = description.unwrap_or_default();
                    match crate::skill::manager::add_skill(
                        &skills_dir,
                        &id,
                        &display_name,
                        &desc,
                        "",
                    ) {
                        Ok(p) => {
                            println!("{} Created skill at {}", "✔".green(), p.display());
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to create skill: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Edit { id, sync, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let skills = crate::skill::inspector::inspect_skills(&skills_dir);
                    let skill = match skills.iter().find(|s| s.id == *id) {
                        Some(s) => s,
                        None => {
                            eprintln!("{} Skill '{}' not found in workspace", "✖".red(), id);
                            return ExitCode::from(1);
                        }
                    };
                    if skill.status == crate::skill::model::SkillStatus::Missing {
                        eprintln!(
                            "{} Skill '{}' is missing on disk. Restore it first.",
                            "✖".red(),
                            id
                        );
                        return ExitCode::from(1);
                    }
                    match crate::skill::editor::dispatch_editor(
                        &skill.file_path,
                        &config_dir,
                        false,
                    ) {
                        Ok(_) => {
                            if !sync {
                                return ExitCode::SUCCESS;
                            }
                            let scope = crate::pack::targets::InstallScope::Local(target_dir.clone());
                            let agents: Vec<_> = crate::pack::targets::TargetAgent::ALL
                                .iter()
                                .copied()
                                .filter(|a| a.is_detected(&scope))
                                .collect();
                            #[allow(clippy::match_bool)] // avoid `else` branch; keep fetched agents when non-empty
                            let agents = match agents.is_empty() {
                                true => vec![crate::pack::targets::TargetAgent::OpenCode],
                                false => agents,
                            };
                            let report = crate::skill::sync::sync_skills(&skills_dir, &scope, &agents);
                            println!(
                                "{} Synced {} skill(s) after edit",
                                "✔".green(),
                                report.synced
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Editor error: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Delete { id, yes, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);

                    let confirmed = yes || inquire::Confirm::new(&format!(
                        "Are you sure you want to delete skill '{}'?",
                        id
                    ))
                    .with_default(false)
                    .prompt()
                    .unwrap_or(false);
                    if !confirmed {
                        println!("{}", "Deletion cancelled.".yellow());
                        return ExitCode::SUCCESS;
                    }

                    match crate::skill::manager::delete_skill(&skills_dir, &id) {
                        Ok(_) => {
                            println!("{} Deleted skill '{}'", "✔".green(), id);
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to delete skill: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Restore {
                    skills,
                    all,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);

                    if !all && !skills.is_empty() {
                        let refs: Vec<&str> = skills.iter().map(|s| s.as_str()).collect();
                        match crate::skill::restore::restore_skills(&skills_dir, &refs) {
                            Ok(restored) => {
                                println!(
                                    "{} Restored {} skill(s): {}",
                                    "✔".green(),
                                    restored.len(),
                                    restored.join(", ")
                                );
                                return ExitCode::SUCCESS;
                            }
                            Err(e) => {
                                eprintln!("{} Failed to restore skills: {e}", "✖".red());
                                return ExitCode::from(1);
                            }
                        }
                    }
                    match crate::skill::restore::restore_all_builtin_skills(&skills_dir) {
                        Ok(restored) => {
                            println!(
                                "{} Restored {} built-in skill(s)",
                                "✔".green(),
                                restored.len()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to restore skills: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Export { id, out, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    match crate::skill::manager::export_skill(&skills_dir, &id, &out) {
                        Ok(_) => {
                            println!(
                                "{} Exported skill '{}' to {}",
                                "✔".green(),
                                id,
                                out.display()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to export skill: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Import { file, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    match crate::skill::manager::import_skill(&skills_dir, &file) {
                        Ok(id) => {
                            println!(
                                "{} Imported skill '{}' from {}",
                                "✔".green(),
                                id,
                                file.display()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to import skill: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
                Some(SkillCommands::Config {
                    preference,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let mut config = crate::config::load_config(&target_dir);

                    match preference {
                        None => {
                            println!("Current configuration:");
                            println!(
                                "  Editor preference: {}",
                                config.editor.preference.to_string().cyan()
                            );
                            println!(
                                "  Custom command:    {}",
                                config.editor.custom_command.yellow()
                            );
                        }
                        Some(pref) => {
                            match pref.to_lowercase().as_str() {
                                "ask" => {
                                    config.editor.preference = crate::config::EditorPreference::Ask
                                }
                                "terminal" => {
                                    config.editor.preference = crate::config::EditorPreference::Terminal
                                }
                                "gui" => {
                                    config.editor.preference = crate::config::EditorPreference::Gui
                                }
                                other => {
                                    eprintln!(
                                        "{} Unknown preference '{}'. Use 'ask', 'terminal', or 'gui'.",
                                        "✖".red(),
                                        other
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            match crate::config::save_config(&target_dir, &config) {
                                Ok(()) => {}
                                Err(e) => {
                                    eprintln!("{} Failed to save config: {e}", "✖".red());
                                    return ExitCode::from(1);
                                }
                            }
                            println!(
                                "{} Editor preference updated to: {}",
                                "✔".green(),
                                config.editor.preference
                            );
                        }
                    }
                    ExitCode::SUCCESS
                }

                Some(SkillCommands::Validate { id, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let entries = crate::skill::inspector::inspect_skills(&skills_dir);
                    let entry = match entries.iter().find(|e| e.id == id) {
                        Some(e) => e,
                        None => {
                            eprintln!("{} Skill '{}' not found in workspace", "✖".red(), id);
                            return ExitCode::from(1);
                        }
                    };
                    if entry.status == crate::skill::model::SkillStatus::Missing {
                        eprintln!("{} Skill '{}' is missing on disk. Restore it first.", "✖".red(), id);
                        return ExitCode::from(1);
                    }
                    let content = std::fs::read_to_string(&entry.file_path).unwrap_or_default();
                    let errors = crate::skill::validator::validate_skill_content(&content);
                    if errors.is_empty() {
                        println!("{} Skill '{}' passed validation (0 errors, 0 warnings)", "✔".green(), id);
                        return ExitCode::SUCCESS;
                    }
                    let err_count = errors.iter().filter(|e| e.level == crate::skill::validator::ValidationLevel::Error).count();
                    let warn_count = errors.iter().filter(|e| e.level == crate::skill::validator::ValidationLevel::Warning).count();
                    println!("{} Skill '{}' validation results: {} errors, {} warnings", "✖".red(), id, err_count, warn_count);
                    for e in &errors {
                        let level = match e.level {
                            crate::skill::validator::ValidationLevel::Error => "ERROR",
                            _ => "WARN",
                        };
                        println!("   - {} [{}] {}", e.code, level, e.message);
                    }
                    #[allow(clippy::match_bool)] // avoid `else` branch; warning-only validation succeeds
                    match crate::skill::validator::has_errors(&errors) {
                        true => ExitCode::from(1),
                        false => ExitCode::SUCCESS,
                    }
                }

                Some(SkillCommands::Diff { id, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    match crate::skill::diff::diff_skill(&skills_dir, &id) {
                        Ok(out) => {
                            println!("{}", out);
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} {}", "✖".red(), e);
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::Sync { agent, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let scope = crate::pack::targets::InstallScope::Local(target_dir.clone());
                    #[allow(clippy::match_bool)] // avoid `else` branch; empty selection defaults to OpenCode
                    let agents: Vec<crate::pack::targets::TargetAgent> = match agent {
                        Some(a) => {
                            let mut v = Vec::new();
                            for item in a.split(',') {
                                let t = item.trim().to_lowercase();
                                match t.as_str() {
                                    "all" => {
                                        for ag in crate::pack::targets::TargetAgent::ALL {
                                            if !v.contains(&ag) {
                                                v.push(ag);
                                            }
                                        }
                                    }
                                    "detected" => {
                                        for ag in crate::pack::targets::TargetAgent::ALL {
                                            if ag.is_detected(&scope) && !v.contains(&ag) {
                                                v.push(ag);
                                            }
                                        }
                                    }
                                    "opencode" => v.push(crate::pack::targets::TargetAgent::OpenCode),
                                    "cursor" => v.push(crate::pack::targets::TargetAgent::Cursor),
                                    "claude" | "claudecode" => v.push(crate::pack::targets::TargetAgent::ClaudeCode),
                                    "windsurf" => v.push(crate::pack::targets::TargetAgent::Windsurf),
                                    "antigravity" | "agy" => v.push(crate::pack::targets::TargetAgent::Antigravity),
                                    "goose" => v.push(crate::pack::targets::TargetAgent::Goose),
                                    _ => {}
                                }
                            }
                            match v.is_empty() {
                                true => vec![crate::pack::targets::TargetAgent::OpenCode],
                                false => v,
                            }
                        }
                        None => {
                            let detected: Vec<_> = crate::pack::targets::TargetAgent::ALL
                                .iter()
                                .copied()
                                .filter(|a| a.is_detected(&scope))
                                .collect();
                            match detected.is_empty() {
                                true => vec![crate::pack::targets::TargetAgent::OpenCode],
                                false => detected,
                            }
                        }
                    };
                    let report = crate::skill::sync::sync_skills(&skills_dir, &scope, &agents);
                    println!("{} Synced {} skill(s), skipped {}, errors: {}", "✔".green(), report.synced, report.skipped, report.errors.len());
                    for e in &report.errors {
                        eprintln!("   {}", e);
                    }
                    ExitCode::SUCCESS
                }

                Some(SkillCommands::Rollback { id, to, list, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    if list {
                        let backups = crate::skill::rollback::list_backups(&skills_dir, &id);
                        #[allow(clippy::match_bool)] // avoid `else` branch; both paths print then succeed
                        match backups.is_empty() {
                            true => println!("No backups found for '{}'", id),
                            false => {
                                println!("Backups for '{}':", id);
                                for b in &backups {
                                    println!("   - {}", b);
                                }
                            }
                        }
                        return ExitCode::SUCCESS;
                    }
                    match crate::skill::rollback::rollback_skill(&skills_dir, &id, to.as_deref()) {
                        Ok(msg) => {
                            println!("{} {}", "✔".green(), msg);
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} {}", "✖".red(), e);
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::Status { dir, path }) => {
                let target_dir = path.or(dir).unwrap_or(base_path.clone());
                let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                match crate::skill::state::check_drift(&skills_dir) {
                    Ok(drifts) => {
                        #[allow(clippy::match_bool)] // avoid `else` branch; both paths fall through to success
                        match drifts.is_empty() {
                            true => println!("{} No drift detected", "✔".green()),
                            false => {
                                println!("{} Drift detected ({}):", "⚠".yellow(), drifts.len());
                                for d in &drifts {
                                    println!("   - {} [{}] {}", d.id, d.kind, d.detail);
                                }
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        // No state file yet; offer rebuild
                        eprintln!("{} {}", "✖".red(), e);
                        println!("Hint: run 'asep skill state --rebuild' to create initial state.");
                        ExitCode::from(1)
                    }
                }
            }

            Some(SkillCommands::State { rebuild, dir, path }) => {
                let target_dir = path.or(dir).unwrap_or(base_path.clone());
                let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                if !rebuild {
                    match crate::skill::state::load_state(&skills_dir) {
                        Some(s) => {
                            println!("State snapshot: {} skill(s), generated {}", s.skills.len(), s.generated_at);
                            for sk in &s.skills {
                                println!("   - {} v{:?} [{}]", sk.id, sk.version, sk.status);
                            }
                            return ExitCode::SUCCESS;
                        }
                        None => {
                            eprintln!("{} No state file. Use --rebuild.", "✖".red());
                            return ExitCode::from(1);
                        }
                    }
                }
                let state = crate::skill::state::rebuild_state(&skills_dir);
                match crate::skill::state::save_state(&skills_dir, &state) {
                    Ok(_) => {
                        println!("{} State rebuilt with {} skill(s)", "✔".green(), state.skills.len());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{} {}", "✖".red(), e);
                        ExitCode::from(1)
                    }
                }
            }

            Some(SkillCommands::Lock { verify, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    if !verify {
                        match crate::skill::lock::generate_lock(&skills_dir) {
                            Ok(lock) => {
                                println!("{} Generated skills.lock with {} skill(s)", "✔".green(), lock.skills.len());
                                return ExitCode::SUCCESS;
                            }
                            Err(e) => {
                                eprintln!("{} {}", "✖".red(), e);
                                return ExitCode::from(1);
                            }
                        }
                    }
                    match crate::skill::lock::verify_lock(&skills_dir) {
                        Ok(drifts) => {
                            if drifts.is_empty() {
                                println!("{} Lock file verified: no drift", "✔".green());
                                return ExitCode::SUCCESS;
                            }
                            println!("{} Lock drift detected ({}):", "⚠".yellow(), drifts.len());
                            for d in &drifts {
                                println!("   - {} [{}] {}", d.id, d.kind, d.detail);
                            }
                            ExitCode::from(1)
                        }
                        Err(e) => {
                            eprintln!("{} {}", "✖".red(), e);
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::Search { query, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let results = crate::skill::search::search_skills(&skills_dir, &query);
                    #[allow(clippy::match_bool)] // avoid `else` branch; both paths fall through to success
                    match results.is_empty() {
                        true => println!("No skills found matching '{}'", query),
                        false => {
                            println!("Found {} skill(s) matching '{}':", results.len(), query);
                            crate::skill::menu::print_skills_table(&results);
                        }
                    }
                    ExitCode::SUCCESS
                }

                Some(SkillCommands::Fork {
                    src,
                    id,
                    name,
                    description,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let display_name = name.as_deref().unwrap_or(&id);
                    let desc = description.as_deref().unwrap_or("");
                    match crate::skill::fork::fork_skill(&skills_dir, &src, &id, display_name, desc) {
                        Ok(p) => {
                            println!("{} Forked '{}' into '{}' at {}", "✔".green(), src, id, p.display());
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to fork skill: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::Install {
                    id,
                    no_deps,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);

                    let source = crate::skill::remote::parse_skill_specifier(&id);
                    match source {
                        crate::skill::remote::SkillSource::Git { .. }
                        | crate::skill::remote::SkillSource::Http { .. } => {
                            match crate::skill::remote::fetch_remote_skill(&source, &skills_dir) {
                                Ok(p) => {
                                    println!("{} Fetched and installed remote skill at {}", "✔".green(), p.display());
                                    ExitCode::SUCCESS
                                }
                                Err(e) => {
                                    eprintln!("{} Failed to fetch remote skill: {e}", "✖".red());
                                    ExitCode::from(1)
                                }
                            }
                        }
                        crate::skill::remote::SkillSource::Registry { id: ref reg_id, .. } => {
                            match crate::skill::install::install_skill(&skills_dir, reg_id, !no_deps) {
                                Ok(paths) => {
                                    #[allow(clippy::match_bool)] // avoid `else` branch; both paths fall through to success
                                    match paths.is_empty() {
                                        true => println!("Skill '{}' (and its dependencies) already installed.", reg_id),
                                        false => {
                                            println!("{} Installed {} skill(s):", "✔".green(), paths.len());
                                            for p in paths {
                                                println!("   - {}", p.display());
                                            }
                                        }
                                    }
                                    ExitCode::SUCCESS
                                }
                                Err(e) => {
                                    eprintln!("{} Failed to install skill: {e}", "✖".red());
                                    ExitCode::from(1)
                                }
                            }
                        }
                    }
                }

                Some(SkillCommands::Merge {
                    id,
                    remote,
                    interactive,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let entries = crate::skill::inspector::inspect_skills(&skills_dir);
                    let entry = match entries.iter().find(|e| e.id == id.as_str()) {
                        Some(e) => e,
                        None => {
                            eprintln!("{} Skill '{}' not found", "✖".red(), id);
                            return ExitCode::from(1);
                        }
                    };

                    let local_content = match std::fs::read_to_string(&entry.file_path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("{} Failed to read local skill: {e}", "✖".red());
                            return ExitCode::from(1);
                        }
                    };

                    // Embedded original acts as base
                    let embedded = crate::pack::embedded::get_embedded_skills();
                    let orig = embedded.iter().find(|e| e.id == id.as_str() || e.directory == id.as_str());
                    let base_content = match orig {
                        Some(o) => o.content,
                        None => {
                            eprintln!("{} No base embedded version found for '{}'", "✖".red(), id);
                            return ExitCode::from(1);
                        }
                    };

                    let remote_content = match remote {
                        Some(r_path) => match std::fs::read_to_string(r_path) {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("{} Failed to read remote skill: {e}", "✖".red());
                                return ExitCode::from(1);
                            }
                        },
                        None => base_content.to_string(),
                    };

                    let (mut merged, has_conflict) =
                        crate::skill::merge::three_way_merge(base_content, &local_content, &remote_content);

                    if has_conflict && interactive {
                        println!("Starting interactive conflict resolution for '{}'...", id);
                        #[allow(clippy::match_wild_err_arm, clippy::single_match)] // `if let` would nest inside an enclosing `if`; match keeps guard-clause style
                        match crate::skill::merge::interactive_resolve_conflicts(&merged) {
                            Ok(resolved) => {
                                merged = resolved;
                                println!("{} All conflict chunks resolved.", "✔".green());
                            }
                            Err(_) => {}
                        }
                    }

                    if let Err(e) = std::fs::write(&entry.file_path, &merged) {
                        eprintln!("{} Failed to write merged skill: {e}", "✖".red());
                        return ExitCode::from(1);
                    }

                    if has_conflict && !interactive {
                        println!(
                            "{} Merge completed with CONFLICTS in '{}'. Resolve conflict markers manually (or re-run with --interactive).",
                            "⚠".yellow(),
                            entry.file_path.display()
                        );
                    }
                    if !has_conflict || interactive {
                        println!(
                            "{} Successfully merged skill '{}'.",
                            "✔".green(),
                            id
                        );
                    }
                    ExitCode::SUCCESS
                }

                Some(SkillCommands::Purge {
                    id,
                    agents,
                    yes,
                    dir,
                    path,
                }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);

                    let confirmed = yes || inquire::Confirm::new(&format!(
                        "Are you sure you want to completely purge skill '{}' and all its synced copies?",
                        id
                    ))
                    .with_default(false)
                    .prompt()
                    .unwrap_or(false);
                    if !confirmed {
                        println!("Purge cancelled.");
                        return ExitCode::SUCCESS;
                    }

                    let scope = crate::pack::targets::InstallScope::Local(target_dir.clone());
                    let parsed_agents: Option<Vec<crate::pack::targets::TargetAgent>> = agents.map(|a_str| {
                        a_str
                            .split(',')
                            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                                "cursor" => Some(crate::pack::targets::TargetAgent::Cursor),
                                "opencode" => Some(crate::pack::targets::TargetAgent::OpenCode),
                                "antigravity" => Some(crate::pack::targets::TargetAgent::Antigravity),
                                "claude" | "claude-code" => Some(crate::pack::targets::TargetAgent::ClaudeCode),
                                "windsurf" => Some(crate::pack::targets::TargetAgent::Windsurf),
                                "zed" => Some(crate::pack::targets::TargetAgent::Zed),
                                "aider" => Some(crate::pack::targets::TargetAgent::Aider),
                                _ => None,
                            })
                            .collect()
                    });

                    match crate::skill::purge::purge_skill(
                        &skills_dir,
                        &id,
                        parsed_agents.as_deref(),
                        &scope,
                    ) {
                        Ok(rep) => {
                            #[allow(clippy::match_bool)] // avoid `else` branch; empty list prints "none"
                            let agents_cleaned = match rep.agents_cleaned.is_empty() {
                                true => "none".to_string(),
                                false => rep.agents_cleaned.join(", "),
                            };
                            println!(
                                "{} Purged skill '{}': workspace removed: {}, synced agents cleaned: {}",
                                "✔".green(),
                                id,
                                rep.workspace_removed,
                                agents_cleaned
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Failed to purge skill '{}': {e}", "✖".red(), id);
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::BundleExport { out, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    match crate::skill::state::export_bundle(&skills_dir, &out) {
                        Ok(b) => {
                            println!(
                                "{} Exported {} skill(s) to bundle: {}",
                                "✔".green(),
                                b.skills.len(),
                                out.display()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Export bundle failed: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }

                Some(SkillCommands::BundleImport { file, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    match crate::skill::state::import_bundle(&skills_dir, &file) {
                        Ok(b) => {
                            println!(
                                "{} Imported {} skill(s) from bundle: {}",
                                "✔".green(),
                                b.skills.len(),
                                file.display()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{} Import bundle failed: {e}", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }

            }
        }

        Some(Commands::Harness { sub }) => match sub {
            HarnessCommands::Complexity {
                cmd,
                sizes,
                max_slope,
                timeout,
            } => {
                let timeout = match parse_duration(&timeout) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("{} Invalid timeout '{timeout}': {e}", "✖".red());
                        return ExitCode::from(1);
                    }
                };
                let sizes: Result<Vec<usize>, String> = sizes
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.parse::<usize>()
                            .map_err(|e| format!("invalid size '{s}': {e}"))
                    })
                    .collect();
                let sizes = match sizes {
                    Ok(s) if !s.is_empty() => s,
                    Ok(_) => {
                        eprintln!("{} 'sizes' must not be empty", "✖".red());
                        return ExitCode::from(1);
                    }
                    Err(e) => {
                        eprintln!("{} {e}", "✖".red());
                        return ExitCode::from(1);
                    }
                };

                match crate::harness::process::run_cmd_complexity(&cmd, &sizes, max_slope, timeout)
                {
                    Ok(slope) => {
                        println!(
                            "{} complexity check passed: log-log slope {:.4} <= {}",
                            "✔".green(),
                            slope,
                            max_slope
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{} complexity check failed: {e}", "✖".red());
                        ExitCode::from(1)
                    }
                }
            }
            HarnessCommands::Memory {
                cmd,
                peak,
                timeout,
            } => {
                let budget = match parse_size_bytes(&peak) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{} Invalid peak size '{peak}': {e}", "✖".red());
                        return ExitCode::from(1);
                    }
                };
                let timeout = match parse_duration(&timeout) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("{} Invalid timeout '{timeout}': {e}", "✖".red());
                        return ExitCode::from(1);
                    }
                };

                match crate::harness::process::run_cmd_memory(&cmd, budget, timeout) {
                    Ok(peak_rss) => {
                        println!(
                            "{} memory check passed: peak RSS {peak_rss} bytes <= budget {budget} bytes",
                            "✔".green()
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{} memory check failed: {e}", "✖".red());
                        ExitCode::from(1)
                    }
                }
            }
            HarnessCommands::NewRun { run_id, root } => {
                let root = root
                    .or_else(|| std::env::var("RUN_ROOT").ok().map(PathBuf::from))
                    .unwrap_or_else(crate::harness::e2e::default_run_root);
                match crate::harness::e2e::new_run(&root, &run_id) {
                    Ok(run_dir) => {
                        println!("{}", run_dir.display());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{} {e}", "✖".red());
                        ExitCode::from(1)
                    }
                }
            }
            HarnessCommands::VendorLoom {
                run_dir,
                vendor_path,
                offline: _,
            } => {
                let vendor_src = vendor_path
                    .unwrap_or_else(|| crate::harness::e2e::repo_root().join("harness").join("e2e").join("vendor-loom"));
                match crate::harness::e2e::vendor_loom(&run_dir, &vendor_src) {
                    Ok(()) => {
                        println!("VENDOR_OK");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::from(1)
                    }
                }
            }
            HarnessCommands::Verify { run_dir } => {
                let repo_root = crate::harness::e2e::repo_root();
                match crate::harness::e2e::verify_artifacts(&run_dir, &repo_root) {
                    Ok(report) => {
                        println!(
                            "pass={} hash_lock_ok={} test_rerun_exit_code={} check_rerun_exit_code={}",
                            report.pass,
                            report.hash_lock_ok,
                            report.test_rerun.exit_code,
                            report.check_rerun.exit_code,
                        );
                        #[allow(clippy::match_bool)]
                        // Guard-clause policy: match bool instead of `if/else`.
                        match report.pass {
                            true => ExitCode::SUCCESS,
                            false => ExitCode::from(1),
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::from(1)
                    }
                }
            }
        },

        Some(Commands::Clean {
            root,
            max_gb,
            target,
            dry_run,
            measure,
        }) => {
            let root =
                root.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let target_dir = target.clone().unwrap_or_else(|| root.join("target"));
            const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

            if measure {
                let bytes = crate::clean::measure_dir(&target_dir);
                match crate::clean::write_baseline(&root, bytes) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("{} Failed to write baseline: {e}", "✖".red());
                        return ExitCode::from(1);
                    }
                }
                let threshold = crate::clean::threshold_bytes(bytes);
                #[allow(clippy::match_bool)]
                // Policy AS-CODE-ELSE forbids `else`; match the bool explicitly instead.
                match args.json {
                    true => {
                        let report = serde_json::json!({
                            "command": "clean",
                            "measure": true,
                            "target": target_dir,
                            "baseline_bytes": bytes,
                            "threshold_bytes": threshold,
                        });
                        match serde_json::to_string_pretty(&report) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("{} Failed to serialize report: {e}", "✖".red());
                                return ExitCode::from(2);
                            }
                        }
                    }
                    false => {
                        println!(
                            "{} {}: {} bytes (baseline written)",
                            "ASEP clean".bold().cyan(),
                            target_dir.display(),
                            bytes
                        );
                        println!(
                            "  threshold: {} bytes ({:.2} GiB)",
                            threshold,
                            threshold as f64 / GIB
                        );
                    }
                }
                return ExitCode::SUCCESS;
            }

            let default_threshold = || {
                crate::clean::read_baseline(&root)
                    .map(crate::clean::threshold_bytes)
                    .unwrap_or(8 * 1024 * 1024 * 1024)
            };

            let threshold = match max_gb {
                Some(gb) => {
                    if gb <= 0.0 {
                        eprintln!("{} --max-gb must be greater than 0 GiB", "✖".red());
                        return ExitCode::from(1);
                    }
                    (gb * GIB) as u64
                }
                None => default_threshold(),
            };

            match crate::clean::execute(&root, &target_dir, threshold, dry_run) {
                Ok(report) => {
                    #[allow(clippy::match_bool)]
                    // Policy AS-CODE-ELSE forbids `else`; match the bool explicitly instead.
                    match args.json {
                        true => {
                            match serde_json::to_string_pretty(&report) {
                                Ok(j) => println!("{j}"),
                                Err(e) => {
                                    eprintln!("{} Failed to serialize report: {e}", "✖".red());
                                    return ExitCode::from(2);
                                }
                            }
                        }
                        false => {
                            println!("{} {}", "ASEP clean".bold().cyan(), target_dir.display());
                            println!("  before: {} bytes", report.before_bytes);
                            println!("  threshold: {} bytes", report.threshold_bytes);
                            println!("  action: {:?}", report.action);
                            println!("  after: {} bytes", report.after_bytes);
                            println!("  removed: {} bytes", report.removed_bytes);
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{} {}", "✖".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        None => {
            let banner =
                format!("Spec-Engine SDLC Validation Engine v{} (Rust)", env!("CARGO_PKG_VERSION"));
            println!("{}", banner.bold().cyan());
            println!("Run 'spec-engine --help' for available subcommands.");
            ExitCode::SUCCESS
        }
    }
}

/// Prints only the detailed skill drift report for installed skill copies
/// under `.opencode/skills/` and `.agents/plugins/asep/skills/`.
/// Returns exit 1 if any drift is found, otherwise exit 0.
fn run_skill_drift_report(target: &Path) -> ExitCode {
    let drift_dirs = [
        target.join(".opencode").join("skills"),
        target.join(".agents").join("plugins").join("asep").join("skills"),
    ];
    let mut drifts: Vec<crate::skill::inspector::SkillDrift> = drift_dirs
        .iter()
        .flat_map(|d| detect_skill_drift(d))
        .collect();
    drifts.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));

    if drifts.is_empty() {
        println!("{} No skill drift detected", "✔".green());
        return ExitCode::SUCCESS;
    }

    println!("{} Skill drift detected ({}):", "⚠".yellow(), drifts.len());
    for d in &drifts {
        println!("   - {} [{}] {}", d.skill_id.bold(), d.kind, d.path.display());
    }
    ExitCode::from(1)
}

/// Parse a duration string like `10s`, `500ms`, `2m`, `1h` (bare numbers are
/// seconds) into a `std::time::Duration`.
fn parse_duration(s: &str) -> Result<Duration, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty duration string".to_string());
    }

    let split = trimmed
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map_or(trimmed.len(), |(i, _)| i);

    let (num_str, unit) = trimmed.split_at(split);
    let value: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid duration '{s}': {e}"))?;
    let duration = match unit {
        "" | "s" | "S" => Duration::from_secs_f64(value),
        "ms" | "MS" => Duration::from_secs_f64(value / 1_000.0),
        "m" | "M" => Duration::from_secs_f64(value * 60.0),
        "h" | "H" => Duration::from_secs_f64(value * 3600.0),
        _ => return Err(format!("unknown unit in duration '{s}'")),
    };
    Ok(duration)
}
