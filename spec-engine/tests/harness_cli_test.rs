//! Integration tests for the ARCH-10 multi-language CLI harness.

use asep::harness::process::{parse_size_bytes, run_cmd_complexity, run_cmd_memory};
use std::time::Duration;

#[test]
fn test_parse_size_bytes() {
    assert_eq!(parse_size_bytes("0").unwrap(), 0);
    assert_eq!(parse_size_bytes("1024").unwrap(), 1024);
    assert_eq!(parse_size_bytes("64KB").unwrap(), 64 * 1_024);
    assert_eq!(parse_size_bytes("512kb").unwrap(), 512 * 1_024);
    assert_eq!(parse_size_bytes("4mb").unwrap(), 4 * 1_024 * 1_024);
    assert_eq!(parse_size_bytes("2GB").unwrap(), 2 * 1_024 * 1_024 * 1_024);
    assert_eq!(parse_size_bytes("1.5MB").unwrap(), 1_572_864);
    assert_eq!(parse_size_bytes("1KiB").unwrap(), 1_024);
    assert_eq!(parse_size_bytes("1MiB").unwrap(), 1_024 * 1_024);
    assert_eq!(parse_size_bytes("1GiB").unwrap(), 1_024 * 1_024 * 1_024);

    assert!(parse_size_bytes("").is_err());
    assert!(parse_size_bytes("abc").is_err());
    assert!(parse_size_bytes("5XX").is_err());
    assert!(parse_size_bytes("-3").is_err());
}

#[test]
fn test_run_cmd_complexity_linear_passes() {
    let sizes = [50, 100, 200, 400, 800];
    let cmd = "awk 'BEGIN { n = {n}; printf \"{\\\"steps\\\": %d}\\n\", n }'";
    let slope = run_cmd_complexity(cmd, &sizes, 1.2, Duration::from_secs(60)).unwrap();
    assert!(
        (slope - 1.0).abs() < 0.05,
        "expected slope near 1.0, got {slope}"
    );
}

#[test]
fn test_run_cmd_complexity_quadratic_fails() {
    let sizes = [50, 100, 200, 400, 800];
    let cmd = "awk 'BEGIN { n = {n}; printf \"{\\\"steps\\\": %d}\\n\", n * n }'";
    let result = run_cmd_complexity(cmd, &sizes, 1.2, Duration::from_secs(60));
    assert!(result.is_err(), "expected complexity failure, got {result:?}");
    let err = result.unwrap_err();
    assert!(err.contains("exceeds"), "unexpected error: {err}");
}

#[test]
fn test_run_cmd_complexity_no_placeholder_appends_n() {
    let sizes = [10, 20, 40];
    let cmd = "sh -c 'true'";
    let slope = run_cmd_complexity(cmd, &sizes, 1.2, Duration::from_secs(60)).unwrap();
    assert!(slope <= 1.2 + 0.25, "unexpected slope {slope}");
}

#[test]
fn test_run_cmd_memory_pass_and_fail() {
    let ok = run_cmd_memory(
        "sh -c 'true'",
        10 * 1_024 * 1_024 * 1_024,
        Duration::from_secs(60),
    );
    assert!(ok.is_ok(), "expected memory check to pass, got {ok:?}");
    let peak = ok.unwrap();
    assert!(peak > 0, "expected non-zero peak RSS, got {peak}");

    let over = run_cmd_memory("sh -c 'true'", 0, Duration::from_secs(60));
    assert!(over.is_err(), "expected memory check to fail with zero budget, got {over:?}");
    assert!(over.unwrap_err().contains("exceeds budget"));
}
