//! Watch mode daemon: live SDLC validation and traceability re-evaluation.

use crate::policy::TierId;
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
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = tx.send(());
                }
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

            if let Some(cov_p) = coverage_path {
                if let Ok(cov_map) = crate::coverage::parse_lcov_file(cov_p) {
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
                            if item.passed { "PASS" } else { "FAIL" }
                        );
                    }
                }
            }
        }
        Err(e) => eprintln!("Scanner error: {e}"),
    }
}
