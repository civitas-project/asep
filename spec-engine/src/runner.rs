use crate::adr::create_new_adr;
use crate::auditor::audit_project_docstrings;
use crate::cli::{AdrCommands, Cli, Commands, HarnessCommands, SkillCommands};
use crate::diagram::render_plantuml_diagrams;
use crate::graph::builder::{build_relation_graph, build_trace_graph};
use crate::harness::process::parse_size_bytes;
use crate::init::init_asep_project;
use crate::mcp::server::run_mcp_server;
use crate::pack::installer::{run_pack_installer, PackOptions};
use crate::pack::targets::{default_global_home, InstallScope, TargetAgent};
use crate::policy::{load_policy, TierId};
use crate::skill::inspector::detect_skill_drift;
use crate::config::EngineConfig;
use crate::scanner::scan_project_tags;
use crate::scanner::ScannedTag;
use crate::verification::{evaluate, VerificationStatus};
use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use std::fs;
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
}

/// Resolves the rigor tier for a check run.
///
/// Precedence: `--rigor` CLI flag wins; otherwise `@rigor:T<n>` tag wins;
/// otherwise `config.rigor` (parsed via `TierId::from_str`); otherwise `T1`
/// for code scans or `T0` when the scan finds only documentation files.
pub fn resolve_rigor(cli_rigor: Option<&str>, tags: &[ScannedTag], config: &EngineConfig) -> TierId {
    // CLI override has highest precedence.
    if let Some(r) = cli_rigor {
        if let Ok(t) = r.parse::<TierId>() {
            return t;
        }
    }
    let has_code_file = tags.iter().any(|t| matches!(
        t.file_path.extension().and_then(|s| s.to_str()),
        Some("rs" | "go" | "ts" | "js" | "py" | "cpp" | "c" | "h" | "hpp" | "java" | "cs")
    ));

    let default_tier = if has_code_file { TierId::T1 } else { TierId::T0 };

    // @rigor:T<n> tag wins.
    for tag in tags {
        if let crate::scanner::lexer::TagKind::Rigor { tier } = &tag.kind {
            if let Ok(t) = tier.parse::<TierId>() {
                return t;
            }
        }
    }

    // Config value.
    if let Some(r) = &config.rigor {
        if let Ok(t) = r.parse::<TierId>() {
            return t;
        }
    }

    default_tier
}

// @impl:FR-CLI-SCAN-01
// @impl:FR-CLI-CHECK-01
// @impl:FR-CLI-CHECK-02
pub fn run() -> ExitCode {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Check {
            dir,
            path,
            strict,
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
                if let Err(e) = crate::watch::start_watch_mode(&target_path, tier, coverage.as_deref()) {
                    eprintln!("{} Watch mode failed: {e}", "✖".red());
                    return ExitCode::from(1);
                }
                return ExitCode::SUCCESS;
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
                && !has_skill_drift;

            if args.json {
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
            } else {
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

                if report.total_requirements == 0 {
                    println!(
                        "{}",
                        "No @req tags discovered in target repository.".yellow()
                    );
                } else {
                    println!(
                        "Discovered {} requirements across workspace.",
                        report.total_requirements.to_string().bold()
                    );
                }

                if !report.must_broken.is_empty() {
                    println!("\n{}", "❌ Broken MUST Traceability Gates:".red().bold());
                    for broken in &report.must_broken {
                        let mut missing = Vec::new();
                        if broken.missing_design {
                            missing.push("@design");
                        }
                        if broken.missing_impl {
                            missing.push("@impl");
                        }
                        if broken.missing_test {
                            missing.push("@test");
                        }
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
                        let mut missing = Vec::new();
                        if draft.missing_design {
                            missing.push("@design");
                        }
                        if draft.missing_impl {
                            missing.push("@impl");
                        }
                        if draft.missing_test {
                            missing.push("@test");
                        }
                        println!(
                            "   - {} ({}:{}) in-progress (missing: {})",
                            draft.id.bold().cyan(),
                            draft.file_path.display(),
                            draft.line_number,
                            if missing.is_empty() {
                                "none".to_string()
                            } else {
                                missing.join(", ").yellow().to_string()
                            }
                        );
                    }
                    if report.draft_pending.is_empty() {
                        println!(
                            "   All {} DRAFT requirements have full design, impl, and test scaffolds.",
                            report.draft_total
                        );
                    }
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
                    let header = if tier == TierId::T0 {
                        "⚠️ Docstring Contract Warnings (Non-blocking in T0 Mode):".yellow().bold()
                    } else {
                        "❌ Docstring Contract Violations:".red().bold()
                    };
                    println!("\n{header}");
                    for v in &violations {
                        println!(
                            "   - {}:{}: {}",
                            v.file_path.display(),
                            v.line_number,
                            if tier == TierId::T0 { v.message.yellow() } else { v.message.red() }
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
                                let badge = if item.passed {
                                    format!("{:.1}% PASS", item.percentage).green()
                                } else {
                                    format!("{:.1}% FAIL (< 80%)", item.percentage).red()
                                };
                                println!(
                                    "   - {}: {} ({}/{} impl lines covered)",
                                    item.req_id.bold(),
                                    badge,
                                    item.covered_lines,
                                    item.total_lines
                                );
                            }
                            if !cov_rep.all_passed && tier != TierId::T0 {
                                passed = false;
                            }
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

                if passed {
                    println!("\n{}", "✔ Gate Result: PASSED (Exit Code 0)".green().bold());
                } else {
                    println!("\n{}", "✖ Gate Result: FAILED (Exit Code 1)".red().bold());
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

            let exit_codes = policy.exit_codes();
            match verification_report.status {
                VerificationStatus::Unproven => ExitCode::from(exit_codes.unproven),
                VerificationStatus::Fail => ExitCode::from(exit_codes.fail),
                VerificationStatus::Pass => {
                    if passed {
                        ExitCode::from(exit_codes.pass)
                    } else {
                        ExitCode::from(exit_codes.fail)
                    }
                }
            }
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
            } else {
                println!("\n{}", "=== 5-Gate Local CI Verification Pipeline ===".bold());
                for g in &report.gates {
                    let status_badge = if g.skipped {
                        "⊘ SKIPPED".yellow()
                    } else if g.passed {
                        "✔ PASSED".green()
                    } else {
                        "✖ FAILED".red()
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
                if report.all_passed() {
                    println!(
                        "{} All CI gates satisfied in {}ms.",
                        "✔".green().bold(),
                        report.total_duration_ms
                    );
                } else {
                    println!(
                        "{} CI pipeline failed in {}ms.",
                        "✖".red().bold(),
                        report.total_duration_ms
                    );
                }
            }

            if report.unproven {
                ExitCode::from(3)
            } else if report.all_passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
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

            let scope = if local {
                Some(InstallScope::Local(target_path.clone()))
            } else if global {
                Some(InstallScope::Global(default_global_home()))
            } else {
                None
            };

            let mut agents = Vec::new();
            if let Some(agent_str) = agent {
                for item in agent_str.split(',') {
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
                    if let Some(skill) = skills.iter().find(|s| s.id == *id) {
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
                                if sync {
                                    let scope = crate::pack::targets::InstallScope::Local(target_dir.clone());
                                    let agents: Vec<_> = crate::pack::targets::TargetAgent::ALL
                                        .iter()
                                        .copied()
                                        .filter(|a| a.is_detected(&scope))
                                        .collect();
                                    let agents = if agents.is_empty() {
                                        vec![crate::pack::targets::TargetAgent::OpenCode]
                                    } else {
                                        agents
                                    };
                                    let report = crate::skill::sync::sync_skills(&skills_dir, &scope, &agents);
                                    println!(
                                        "{} Synced {} skill(s) after edit",
                                        "✔".green(),
                                        report.synced
                                    );
                                }
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("{} Editor error: {e}", "✖".red());
                                ExitCode::from(1)
                            }
                        }
                    } else {
                        eprintln!("{} Skill '{}' not found in workspace", "✖".red(), id);
                        ExitCode::from(1)
                    }
                }
                Some(SkillCommands::Delete { id, yes, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path);
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);

                    if !yes {
                        let confirmed = inquire::Confirm::new(&format!(
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

                    if all || skills.is_empty() {
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
                    } else {
                        let refs: Vec<&str> = skills.iter().map(|s| s.as_str()).collect();
                        match crate::skill::restore::restore_skills(&skills_dir, &refs) {
                            Ok(restored) => {
                                println!(
                                    "{} Restored {} skill(s): {}",
                                    "✔".green(),
                                    restored.len(),
                                    restored.join(", ")
                                );
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("{} Failed to restore skills: {e}", "✖".red());
                                ExitCode::from(1)
                            }
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

                    if let Some(pref) = preference {
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
                        if let Err(e) = crate::config::save_config(&target_dir, &config) {
                            eprintln!("{} Failed to save config: {e}", "✖".red());
                            return ExitCode::from(1);
                        }
                        println!(
                            "{} Editor preference updated to: {}",
                            "✔".green(),
                            config.editor.preference
                        );
                    } else {
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
                    ExitCode::SUCCESS
                }

                Some(SkillCommands::Validate { id, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let entries = crate::skill::inspector::inspect_skills(&skills_dir);
                    if let Some(entry) = entries.iter().find(|e| e.id == id) {
                        if entry.status == crate::skill::model::SkillStatus::Missing {
                            eprintln!("{} Skill '{}' is missing on disk. Restore it first.", "✖".red(), id);
                            return ExitCode::from(1);
                        }
                        let content = std::fs::read_to_string(&entry.file_path).unwrap_or_default();
                        let errors = crate::skill::validator::validate_skill_content(&content);
                        if errors.is_empty() {
                            println!("{} Skill '{}' passed validation (0 errors, 0 warnings)", "✔".green(), id);
                            ExitCode::SUCCESS
                        } else {
                            let err_count = errors.iter().filter(|e| e.level == crate::skill::validator::ValidationLevel::Error).count();
                            let warn_count = errors.iter().filter(|e| e.level == crate::skill::validator::ValidationLevel::Warning).count();
                            println!("{} Skill '{}' validation results: {} errors, {} warnings", "✖".red(), id, err_count, warn_count);
                            for e in &errors {
                                println!("   - {} [{}] {}", e.code, if e.level == crate::skill::validator::ValidationLevel::Error { "ERROR" } else { "WARN" }, e.message);
                            }
                            if crate::skill::validator::has_errors(&errors) {
                                ExitCode::from(1)
                            } else {
                                ExitCode::SUCCESS
                            }
                        }
                    } else {
                        eprintln!("{} Skill '{}' not found in workspace", "✖".red(), id);
                        ExitCode::from(1)
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
                    let agents: Vec<crate::pack::targets::TargetAgent> = if let Some(a) = agent {
                        let mut v = Vec::new();
                        for item in a.split(',') {
                            let t = item.trim().to_lowercase();
                            match t.as_str() {
                                "all" => {
                                    for ag in crate::pack::targets::TargetAgent::ALL {
                                        if !v.contains(&ag) { v.push(ag); }
                                    }
                                }
                                "detected" => {
                                    for ag in crate::pack::targets::TargetAgent::ALL {
                                        if ag.is_detected(&scope) && !v.contains(&ag) { v.push(ag); }
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
                        if v.is_empty() { vec![crate::pack::targets::TargetAgent::OpenCode] } else { v }
                    } else {
                        let detected: Vec<_> = crate::pack::targets::TargetAgent::ALL.iter().copied().filter(|a| a.is_detected(&scope)).collect();
                        if detected.is_empty() { vec![crate::pack::targets::TargetAgent::OpenCode] } else { detected }
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
                        if backups.is_empty() {
                            println!("No backups found for '{}'", id);
                        } else {
                            println!("Backups for '{}':", id);
                            for b in &backups {
                                println!("   - {}", b);
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
                        if drifts.is_empty() {
                            println!("{} No drift detected", "✔".green());
                        } else {
                            println!("{} Drift detected ({}):", "⚠".yellow(), drifts.len());
                            for d in &drifts {
                                println!("   - {} [{}] {}", d.id, d.kind, d.detail);
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
                if rebuild {
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
                } else {
                    match crate::skill::state::load_state(&skills_dir) {
                        Some(s) => {
                            println!("State snapshot: {} skill(s), generated {}", s.skills.len(), s.generated_at);
                            for sk in &s.skills {
                                println!("   - {} v{:?} [{}]", sk.id, sk.version, sk.status);
                            }
                            ExitCode::SUCCESS
                        }
                        None => {
                            eprintln!("{} No state file. Use --rebuild.", "✖".red());
                            ExitCode::from(1)
                        }
                    }
                }
            }

            Some(SkillCommands::Lock { verify, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    if verify {
                        match crate::skill::lock::verify_lock(&skills_dir) {
                            Ok(drifts) => {
                                if drifts.is_empty() {
                                    println!("{} Lock file verified: no drift", "✔".green());
                                    ExitCode::SUCCESS
                                } else {
                                    println!("{} Lock drift detected ({}):", "⚠".yellow(), drifts.len());
                                    for d in &drifts {
                                        println!("   - {} [{}] {}", d.id, d.kind, d.detail);
                                    }
                                    ExitCode::from(1)
                                }
                            }
                            Err(e) => {
                                eprintln!("{} {}", "✖".red(), e);
                                ExitCode::from(1)
                            }
                        }
                    } else {
                        match crate::skill::lock::generate_lock(&skills_dir) {
                            Ok(lock) => {
                                println!("{} Generated skills.lock with {} skill(s)", "✔".green(), lock.skills.len());
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("{} {}", "✖".red(), e);
                                ExitCode::from(1)
                            }
                        }
                    }
                }

                Some(SkillCommands::Search { query, dir, path }) => {
                    let target_dir = path.or(dir).unwrap_or(base_path.clone());
                    let skills_dir = crate::skill::menu::resolve_skills_dir(&target_dir);
                    let results = crate::skill::search::search_skills(&skills_dir, &query);
                    if results.is_empty() {
                        println!("No skills found matching '{}'", query);
                    } else {
                        println!("Found {} skill(s) matching '{}':", results.len(), query);
                        crate::skill::menu::print_skills_table(&results);
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
                                    if paths.is_empty() {
                                        println!("Skill '{}' (and its dependencies) already installed.", reg_id);
                                    } else {
                                        println!("{} Installed {} skill(s):", "✔".green(), paths.len());
                                        for p in paths {
                                            println!("   - {}", p.display());
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

                    let remote_content = if let Some(r_path) = remote {
                        match std::fs::read_to_string(r_path) {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("{} Failed to read remote skill: {e}", "✖".red());
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        base_content.to_string()
                    };

                    let (mut merged, has_conflict) =
                        crate::skill::merge::three_way_merge(base_content, &local_content, &remote_content);

                    if has_conflict && interactive {
                        println!("Starting interactive conflict resolution for '{}'...", id);
                        if let Ok(resolved) = crate::skill::merge::interactive_resolve_conflicts(&merged) {
                            merged = resolved;
                            println!("{} All conflict chunks resolved.", "✔".green());
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
                    } else {
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

                    if !yes {
                        let confirmed = inquire::Confirm::new(&format!(
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
                            println!(
                                "{} Purged skill '{}': workspace removed: {}, synced agents cleaned: {}",
                                "✔".green(),
                                id,
                                rep.workspace_removed,
                                if rep.agents_cleaned.is_empty() {
                                    "none".to_string()
                                } else {
                                    rep.agents_cleaned.join(", ")
                                }
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
        },

        None => {
            println!(
                "{}",
                "Spec-Engine SDLC Validation Engine v1.0.0 (Rust)"
                    .bold()
                    .cyan()
            );
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
