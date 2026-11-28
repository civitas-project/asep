//! Integration tests for the ASEP e2e harness scaffold.

use std::env;
use std::path::Path;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("spec-engine has a workspace root")
        .to_path_buf()
}

fn git_short_commit(repo: &Path) -> String {
    String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(repo)
            .output()
            .expect("git rev-parse --short HEAD")
            .stdout,
    )
    .expect("git output is utf-8")
    .trim()
    .to_string()
}

const STRESS_FIXTURE: &str = "harness/e2e/fixtures/stress_test.rs";

fn stress_fixture_source() -> String {
    let path = workspace_root().join(STRESS_FIXTURE);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("stress fixture {STRESS_FIXTURE} must exist: {e}"))
}

const PROMPT_DIR: &str = "harness/e2e/prompts";

const PHASE_PROMPTS: &[&str] = &[
    "p1-requirements.md",
    "p2-design.md",
    "p3-tdd.md",
    "p4-implement.md",
    "p5-bug-hunt.md",
    "p6-optimization.md",
    "p7-final.md",
];

#[test]
fn test_stress_fixture_has_timeout_and_iterations() {
    let text = stress_fixture_source();
    assert!(
        text.contains("100_000"),
        "fixture must fix 100_000 operations per configuration"
    );
    assert!(
        text.contains("TIMEOUT_SECS") && text.contains("30"),
        "fixture must fix a 30-second timeout constant"
    );
    assert!(
        text.contains("manifest.json"),
        "fixture must read its seed from manifest.json"
    );
    assert!(
        text.contains("yield_now()"),
        "fixture must hook thread::yield_now() at seed-given points"
    );
}

#[test]
fn test_stress_fixture_has_no_unsafe() {
    let text = stress_fixture_source();
    assert!(!text.contains("unsafe"), "fixture must contain no `unsafe`");
}

#[test]
fn test_phase_prompts_cover_all_phases() {
    let root = workspace_root();
    let prompts_dir = root.join(PROMPT_DIR);
    assert!(
        prompts_dir.is_dir(),
        "prompts dir exists at {}",
        prompts_dir.display()
    );

    for name in PHASE_PROMPTS {
        let path = prompts_dir.join(name);
        assert!(path.is_file(), "phase prompt {} exists", path.display());
    }

    let judge = prompts_dir.join("judge.md");
    assert!(judge.is_file(), "judge.md exists at {}", judge.display());

    let judge_text = std::fs::read_to_string(&judge).expect("read judge.md");
    for key in [
        "verdict",
        "blocking",
        "non_blocking",
        "missing_evidence",
        "next_action",
    ] {
        assert!(
            judge_text.contains(key),
            "judge.md contains verdict key {key}"
        );
    }
    assert!(
        judge_text.contains("cargo test"),
        "judge.md states judge re-runs cargo test"
    );
    assert!(
        judge_text.contains("asep check"),
        "judge.md states judge re-runs asep check"
    );
}

#[test]
fn test_p3_prompt_locks_tests() {
    let p3 = workspace_root().join(PROMPT_DIR).join("p3-tdd.md");
    assert!(p3.is_file(), "p3-tdd.md exists at {}", p3.display());

    let text = std::fs::read_to_string(&p3).expect("read p3-tdd.md");
    assert!(
        text.contains("03-hash.txt"),
        "p3-tdd.md mentions 03-hash.txt test lock"
    );
}
