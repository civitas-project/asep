//! Watch mode daemon: live SDLC validation and traceability re-evaluation.

use crate::auditor::audit_project_anti_slop;
use crate::policy::{Severity, TierId};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

/// Starts watching the target project directory and re-validates on changes.
pub fn start_watch_mode(
    dir: &Path,
    tier: TierId,
    coverage_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() => {
                    let _ = tx.send(());
                }
                _ => {}
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(300)),
    )?;

    watcher.watch(dir, RecursiveMode::Recursive)?;

    println!(
        "\n👀 [ASEP Watch Mode] Monitoring '{}' (rigor tier: {}). Press Ctrl+C to exit.",
        dir.display(),
        tier.as_str()
    );

    // Initial run
    run_cycle(dir, tier, coverage_path);

    let mut last_run = Instant::now();
    while rx.recv().is_ok() {
        // Debounce 300ms
        if last_run.elapsed() < Duration::from_millis(300) {
            continue;
        }
        last_run = Instant::now();

        // Drain any pending events
        while rx.try_recv().is_ok() {}

        print!("\x1B[2J\x1B[1;1H"); // Clear screen
        println!("🔄 Changes detected. Re-evaluating SDLC traceability...");
        run_cycle(dir, tier, coverage_path);
    }

    Ok(())
}

fn run_cycle(dir: &Path, _tier: TierId, coverage_path: Option<&Path>) {
    let start = Instant::now();
    match crate::scanner::scan_project_tags(dir) {
        Ok(tags) => {
            let graph = crate::graph::builder::build_trace_graph(&tags);
            println!(
                "📊 Traceability: {} total reqs, {} closed MUST, {} broken MUST, {} pending SHOULD ({:?})",
                graph.total_requirements,
                graph.must_satisfied,
                graph.must_broken.len(),
                graph.should_pending.len(),
                start.elapsed()
            );

            if let Some(cov_map) = coverage_path
                .and_then(|cov_p| crate::coverage::parse_lcov_file(cov_p).ok())
            {
                let cov_rep = crate::coverage::map_coverage_to_requirements(&tags, &cov_map, 80.0);
                println!(
                    "🎯 Requirement Coverage: {} item(s) audited (Threshold: {}%)",
                    cov_rep.items.len(),
                    cov_rep.threshold
                );
                for item in &cov_rep.items {
                    println!(
                        "   - {}: {:.1}% ({}/{} lines) [{}]",
                        item.req_id,
                        item.percentage,
                        item.covered_lines,
                        item.total_lines,
                        match item.passed {
                            true => "PASS",
                            false => "FAIL",
                        }
                    );
                }
            }

            let (anti_errors, anti_warnings) = anti_slop_summary(dir);
            println!("🧹 Anti-slop: {anti_errors} errors, {anti_warnings} warnings");
        }
        Err(e) => eprintln!("Scanner error: {e}"),
    }
}

/// Counts anti-slop violations by severity for `dir` without failing the
/// cycle. Watch mode reports these every cycle but never exits non-zero.
pub fn anti_slop_summary(dir: &Path) -> (usize, usize) {
    let violations = audit_project_anti_slop(dir).unwrap_or_default();
    let errors = violations.iter().filter(|v| v.severity == Severity::Error).count();
    let warnings = violations.iter().filter(|v| v.severity == Severity::Warning).count();
    (errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_slop_summary_counts_errors_and_warnings() {
        let dir = tempfile::tempdir().expect("create temp dir");
        std::fs::write(dir.path().join("note.md"), "This is a very draft.\n")
            .expect("write markdown fixture");
        std::fs::write(dir.path().join("main.rs"), "// ==========\nfn main() {}\n") // asep:allow-antislop
            .expect("write rust fixture");
        let (errors, warnings) = anti_slop_summary(dir.path());
        assert_eq!((errors, warnings), (1, 1));
    }
}
