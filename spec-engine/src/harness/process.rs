//! Multi-language CLI harness for empirical complexity and process memory
//! budget enforcement (ARCH-10).

use serde_json::Value;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::harness::asymptotic::AsymptoticAnalyzer;

#[cfg(target_os = "linux")]
use std::collections::HashSet;

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Parse a byte size string into bytes.
///
/// Supports raw numbers and units `KB`, `MB`, `GB` (case-insensitive).
/// Units are binary multiples (1 KB = 1024 bytes); `KiB`/`MiB`/`GiB` are
/// accepted as aliases. A leading decimal fraction is allowed.
pub fn parse_size_bytes(s: &str) -> Result<usize, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty size string".to_string());
    }

    let split = trimmed
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
        .map_or(trimmed.len(), |(i, _)| i);

    let (num_str, unit) = trimmed.split_at(split);
    let value: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid size '{s}': {e}"))?;
    let multiplier = match unit {
        "" | "B" | "b" => 1,
        "KB" | "Kb" | "kB" | "kb" | "KIB" | "Kib" | "KiB" | "kib" => 1_024,
        "MB" | "Mb" | "mB" | "mb" | "MIB" | "Mib" | "MiB" | "mib" => 1_024 * 1_024,
        "GB" | "Gb" | "gB" | "gb" | "GIB" | "Gib" | "GiB" | "gib" => 1_024 * 1_024 * 1_024,
        _ => return Err(format!("unknown unit in size '{s}'")),
    };

    Ok((value * multiplier as f64) as usize)
}

/// Replace `{n}` with `n`; append ` n` when the template has no placeholder.
fn substitute_n(cmd_template: &str, n: usize) -> String {
    if !cmd_template.contains("{n}") {
        return format!("{cmd_template} {n}");
    }
    cmd_template.replace("{n}", &n.to_string())
}

/// Spawn `cmd` through the platform shell (`sh -c` on Unix, `cmd /c` on
/// Windows).
fn spawn_shell(cmd: &str) -> Result<Child, String> {
    #[cfg(target_os = "windows")]
    let mut command = Command::new("cmd");
    #[cfg(not(target_os = "windows"))]
    let mut command = Command::new("sh");

    #[cfg(target_os = "windows")]
    command.arg("/c");
    #[cfg(not(target_os = "windows"))]
    command.arg("-c");

    command
        .arg(cmd)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn command '{cmd}': {e}"))
}

/// @impl:FR-AUD-11
/// Spawn `cmd` through the platform shell with stdout/stderr discarded, so
/// a child writing heavily to stdout cannot block on pipe capacity.
fn spawn_shell_quiet(cmd: &str) -> Result<Child, String> {
    #[cfg(target_os = "windows")]
    let mut command = Command::new("cmd");
    #[cfg(not(target_os = "windows"))]
    let mut command = Command::new("sh");

    #[cfg(target_os = "windows")]
    command.arg("/c");
    #[cfg(not(target_os = "windows"))]
    command.arg("-c");

    command
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn command '{cmd}': {e}"))
}

/// Wait for `child` to exit, enforcing `timeout` by polling. On timeout the
/// child is killed and reaped.
fn wait_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("command timed out after {:?}", timeout));
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(e) => return Err(format!("failed to wait for child process: {e}")),
        }
    }
}

/// Capture child stdout and return elapsed microseconds.
fn capture_output(child: &mut Child, start: Instant) -> Result<(String, u64), String> {
    let mut output = String::new();
    child
        .stdout
        .take()
        .ok_or("child has no captured stdout")?
        .read_to_string(&mut output)
        .map_err(|e| format!("failed to read child output: {e}"))?;
    Ok((output, start.elapsed().as_micros() as u64))
}

/// @impl:FR-AUD-14
/// Parse `{"steps": N}` from stdout; any missing/invalid shape is an error.
fn steps_from_output(output: &str) -> Result<u64, String> {
    let trimmed = output.trim();
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|_| "complexity workload did not report a valid steps count".to_string())?;
    let object = value
        .as_object()
        .ok_or("complexity workload did not report a valid steps count")?;
    let steps = object
        .get("steps")
        .and_then(Value::as_u64)
        .ok_or("complexity workload did not report a valid steps count")?;
    Ok(steps)
}

/// Run one shell command and return its step count.
fn run_once(cmd: &str, timeout: Duration) -> Result<u64, String> {
    let start = Instant::now();
    let mut child = spawn_shell(cmd)?;
    let status = wait_with_timeout(&mut child, timeout)?;
    let (output, _elapsed_micros) = capture_output(&mut child, start)?;
    if !status.success() {
        return Err(format!(
            "command '{cmd}' exited with status {status}; output: {output}"
        ));
    }
    steps_from_output(&output)
}

/// Measure empirical log-log complexity slope of an external command and
/// assert it stays within `max_slope` (plus a 0.25 noise tolerance).
///
/// For each `N` in `sizes`, `{n}` in `cmd_template` is replaced with `N`
/// (or `N` is appended when no placeholder exists). Each command is run
/// through the platform shell with `timeout` enforced. Steps come from a
/// `{"steps": N}` JSON payload on stdout.
pub fn run_cmd_complexity(
    cmd_template: &str,
    sizes: &[usize],
    max_slope: f64,
    timeout: Duration,
) -> Result<f64, String> {
    if !max_slope.is_finite() || max_slope < 0.0 {
        return Err("max_slope must be finite and nonnegative".to_string());
    }
    if sizes.is_empty() {
        return Err("sizes must not be empty".to_string());
    }

    let mut measured = Vec::with_capacity(sizes.len());
    for &n in sizes {
        let cmd = substitute_n(cmd_template, n);
        let steps = run_once(&cmd, timeout)?;
        measured.push((n, steps));
    }

    let analyzer = AsymptoticAnalyzer::new();
    analyzer.assert_complexity(
        sizes,
        |n| {
            measured
                .iter()
                .find(|(m, _)| *m == n)
                .map(|(_, steps)| *steps)
                .unwrap_or(0)
        },
        max_slope,
    )
}

/// Sample RSS (kB -> bytes) of `pid` from `/proc/{pid}/status`, preferring
/// `VmHWM` (high-water mark) and falling back to `VmRSS`. Returns 0 when
/// `/proc` is unavailable or the process has already exited.
/// Kept for compatibility; `run_cmd_memory_detailed` does not use it.
#[allow(dead_code)]
fn sample_rss_bytes(pid: u32) -> usize {
    #[cfg(target_os = "linux")]
    {
        let content = match std::fs::read_to_string(format!("/proc/{pid}/status")) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        for line in content.lines() {
            let key = line.split(':').next().unwrap_or("");
            let value = line.split(':').nth(1).unwrap_or("").trim();
            let kb: usize = match value.split_whitespace().next() {
                Some(v) => v.parse().unwrap_or(0),
                None => 0,
            };
            if kb > 0 {
                return match key {
                    "VmHWM" | "VmRSS" => kb * 1_024,
                    _ => continue,
                };
            }
        }
        0
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        0
    }
}

/// @impl:FR-AUD-12
/// Peak RSS of `root_pid` and every descendant discovered through
/// `/proc/<pid>/task/<tid>/children`, in bytes. Descendants are collected in
/// a `HashSet<u32>` (cycle avoidance), recursion stops at depth 16, and each
/// pid's resident pages come from `/proc/<pid>/statm` times a documented
/// page size of 4096 (the Linux baseline page size). Any `/proc` read
/// failure returns `Err` with path context; an exited process is never
/// reported as a zero-byte measurement.
#[cfg(target_os = "linux")]
fn process_tree_rss_bytes(root_pid: u32) -> Result<usize, String> {
    const PAGE_SIZE: usize = 4096; // Linux baseline page size; sysconf(_SC_PAGESIZE) may differ on exotic kernels.
    std::fs::read_to_string(format!("/proc/{root_pid}/status"))
        .map_err(|e| format!("failed to read /proc/{root_pid}/status: {e}"))?;
    let mut pids = HashSet::new();
    discover_tree(root_pid, 0, &mut pids)?;
    pids.insert(root_pid);
    let mut total = 0usize;
    for pid in pids {
        let pages = statm_resident_pages(pid)?;
        total = total
            .checked_add(pages * PAGE_SIZE)
            .ok_or("rss sum overflow".to_string())?;
    }
    Ok(total)
}

#[cfg(target_os = "linux")]
fn discover_tree(pid: u32, depth: u32, seen: &mut HashSet<u32>) -> Result<(), String> {
    if depth >= 16 {
        return Ok(());
    }
    for tid in task_ids(pid)? {
        let content = std::fs::read_to_string(format!("/proc/{pid}/task/{tid}/children"))
            .map_err(|e| format!("failed to read /proc/{pid}/task/{tid}/children: {e}"))?;
        for token in content.split_whitespace() {
            let child: u32 = token
                .parse()
                .map_err(|e| format!("invalid child pid '{token}' in /proc/{pid}/task/{tid}/children: {e}"))?;
            if seen.insert(child) {
                discover_tree(child, depth + 1, seen)?;
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn task_ids(pid: u32) -> Result<Vec<u32>, String> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(format!("/proc/{pid}/task"))
        .map_err(|e| format!("failed to read /proc/{pid}/task: {e}"))?
    {
        let entry = entry
            .map_err(|e| format!("failed to read task dir entry for /proc/{pid}/task: {e}"))?;
        let id: u32 = entry
            .file_name()
            .to_string_lossy()
            .parse()
            .map_err(|e| format!("non-numeric task dir under /proc/{pid}/task: {e}"))?;
        ids.push(id);
    }
    Ok(ids)
}

#[cfg(target_os = "linux")]
fn statm_resident_pages(pid: u32) -> Result<usize, String> {
    let content = std::fs::read_to_string(format!("/proc/{pid}/statm"))
        .map_err(|e| format!("failed to read /proc/{pid}/statm: {e}"))?;
    let pages = content
        .split_whitespace()
        .nth(1)
        .ok_or(format!("missing resident field in /proc/{pid}/statm"))?
        .parse()
        .map_err(|e| format!("invalid resident field in /proc/{pid}/statm: {e}"))?;
    Ok(pages)
}

#[cfg(not(target_os = "linux"))]
fn process_tree_rss_bytes(_root_pid: u32) -> Result<usize, String> {
    Err("memory measurement unavailable on this platform".to_string())
}

#[cfg(target_os = "linux")]
const MEMORY_METHOD: &str = "linux-process-tree-rss";
#[cfg(not(target_os = "linux"))]
const MEMORY_METHOD: &str = "unavailable";

#[cfg(target_os = "linux")]
const MEMORY_COMPLETE: bool = true;
#[cfg(not(target_os = "linux"))]
const MEMORY_COMPLETE: bool = false;

/// Peak RSS measurement result.
pub struct MemoryMeasurement {
    pub peak_bytes: usize,
    pub method: &'static str,
    pub complete: bool,
}

/// Run `cmd` through the platform shell while sampling peak RSS of the whole
/// process tree, enforcing `timeout` and `peak_budget_bytes`. A sampling
/// failure returns `Err`; zero is never accepted as a valid measurement.
pub fn run_cmd_memory_detailed(
    cmd: &str,
    peak_budget_bytes: usize,
    timeout: Duration,
) -> Result<MemoryMeasurement, String> {
    let mut child = spawn_shell_quiet(cmd)?;
    let pid = child.id();
    let start = Instant::now();
    let mut peak = process_tree_rss_bytes(pid)
        .map_err(|e| format!("memory measurement unavailable: {e}"))?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(format!("command exited with {status}"));
                }
                break;
            }
            Ok(None) => {
                let sample = process_tree_rss_bytes(pid)
                    .map_err(|e| format!("memory measurement unavailable: {e}"))?;
                peak = peak.max(sample);
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("command timed out after {:?}", timeout));
                }
                thread::sleep(POLL_INTERVAL);
            }
            Err(e) => return Err(format!("failed to wait for child process: {e}")),
        }
    }

    if peak > peak_budget_bytes {
        return Err(format!(
            "peak RSS {peak} bytes exceeds budget {peak_budget_bytes} bytes"
        ));
    }
    Ok(MemoryMeasurement {
        peak_bytes: peak,
        method: MEMORY_METHOD,
        complete: MEMORY_COMPLETE,
    })
}

/// @impl:FR-AUD-10
/// Run `cmd` through the platform shell while sampling peak RSS, enforcing
/// `timeout` and `peak_budget_bytes`. Returns the observed peak RSS in bytes.
pub fn run_cmd_memory(
    cmd: &str,
    peak_budget_bytes: usize,
    timeout: Duration,
) -> Result<usize, String> {
    run_cmd_memory_detailed(cmd, peak_budget_bytes, timeout).map(|m| m.peak_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_bytes_raw_and_units() {
        assert_eq!(parse_size_bytes("0").unwrap(), 0);
        assert_eq!(parse_size_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_size_bytes("64KB").unwrap(), 64 * 1_024);
        assert_eq!(parse_size_bytes("4mb").unwrap(), 4 * 1_024 * 1_024);
        assert_eq!(parse_size_bytes("2GB").unwrap(), 2 * 1_024 * 1_024 * 1_024);
        assert_eq!(parse_size_bytes("1.5MB").unwrap(), 1_572_864);
        assert_eq!(parse_size_bytes("1KiB").unwrap(), 1_024);
        assert_eq!(parse_size_bytes("1MiB").unwrap(), 1_024 * 1_024);
    }

    #[test]
    fn parse_size_bytes_rejects_invalid() {
        assert!(parse_size_bytes("").is_err());
        assert!(parse_size_bytes("abc").is_err());
        assert!(parse_size_bytes("5XX").is_err());
    }

    #[test]
    fn run_cmd_complexity_linear_passes() {
        let sizes = [50, 100, 200, 400, 800];
        let cmd = "awk 'BEGIN { n = {n}; printf \"{\\\"steps\\\": %d}\\n\", n }'";
        let slope = run_cmd_complexity(cmd, &sizes, 1.2, Duration::from_secs(30)).unwrap();
        assert!((slope - 1.0).abs() < 0.01, "slope was {slope}");
    }

    #[test]
    fn run_cmd_complexity_quadratic_fails() {
        let sizes = [50, 100, 200, 400, 800];
        let cmd = "awk 'BEGIN { n = {n}; printf \"{\\\"steps\\\": %d}\\n\", n * n }'";
        let result = run_cmd_complexity(cmd, &sizes, 1.2, Duration::from_secs(30));
        assert!(result.is_err(), "expected failure, got {result:?}");
    }

    // @test:FR-AUD-14
    #[test]
    fn complexity_rejects_nan_max_slope() {
        let err = run_cmd_complexity("sh -c 'true'", &[1, 2], f64::NAN, Duration::from_secs(30))
            .unwrap_err();
        assert!(err.contains("finite"), "unexpected error: {err}");
    }

    #[test]
    fn complexity_rejects_invalid_steps() {
        let err = run_cmd_complexity(
            "sh -c 'echo not-json'",
            &[1, 2],
            1.2,
            Duration::from_secs(30),
        )
        .unwrap_err();
        assert!(err.contains("steps"), "unexpected error: {err}");

        let err = run_cmd_complexity(
            "sh -c 'echo {\"steps\": -1}'",
            &[1, 2],
            1.2,
            Duration::from_secs(30),
        )
        .unwrap_err();
        assert!(err.contains("steps"), "unexpected error: {err}");
    }

    #[test]
    fn run_cmd_memory_pass_and_fail() {
        let ok = run_cmd_memory(
            "sh -c 'true'",
            10 * 1_024 * 1_024 * 1_024,
            Duration::from_secs(30),
        );
        assert!(ok.is_ok(), "expected pass, got {ok:?}");
        let peak = ok.unwrap();
        assert!(peak > 0);

        let over = run_cmd_memory("sh -c 'true'", 0, Duration::from_secs(30));
        assert!(
            over.is_err(),
            "expected fail with zero budget, got {over:?}"
        );
    }

    // @test:FR-AUD-10
    #[test]
    fn run_cmd_memory_nonzero_exit_is_error() {
        let err = run_cmd_memory(
            "sh -c 'exit 7'",
            10 * 1_024 * 1_024 * 1_024,
            Duration::from_secs(30),
        )
        .unwrap_err();
        assert!(err.contains("exited with"), "unexpected error: {err}");

        let ok = run_cmd_memory(
            "sh -c 'true'",
            10 * 1_024 * 1_024 * 1_024,
            Duration::from_secs(30),
        );
        assert!(ok.is_ok(), "expected ok, got {ok:?}");
    }

    // @test:FR-AUD-11
    #[test]
    fn run_cmd_memory_large_stdout_completes() {
        let ok = run_cmd_memory(
            "head -c 1048576 /dev/zero",
            10 * 1_024 * 1_024 * 1_024,
            Duration::from_secs(30),
        );
        assert!(ok.is_ok(), "expected ok, got {ok:?}");
    }

    // @test:FR-AUD-12
    #[test]
    fn memory_measurement_records_method() {
        let result = run_cmd_memory_detailed(
            "sh -c 'sleep 0.2'",
            10 * 1_024 * 1_024 * 1_024,
            Duration::from_secs(30),
        );
        #[cfg(target_os = "linux")]
        {
            let m = result.unwrap();
            assert_eq!(m.method, "linux-process-tree-rss");
            assert!(m.complete);
            assert!(m.peak_bytes > 0, "peak was {}", m.peak_bytes);
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert!(result.is_err(), "expected err on non-linux, got {result:?}");
        }
    }

    #[test]
    fn child_process_memory_is_counted() {
        let cmd = match memory_intensive_command() {
            Some(c) => c,
            None => {
                eprintln!("skip child_process_memory_is_counted: no python3 or perl available");
                return;
            }
        };
        let m = run_cmd_memory_detailed(&cmd, 10 * 1_024 * 1_024 * 1_024, Duration::from_secs(30))
            .unwrap_or_else(|e| panic!("memory measurement failed: {e}"));
        assert!(
            m.peak_bytes > 8 * 1_024 * 1_024,
            "peak {} bytes is not above 8 MiB",
            m.peak_bytes
        );
    }

    fn memory_intensive_command() -> Option<String> {
        let probes = [
            ("python3", "a=bytearray(8*1024*1024)"),
            ("perl", "my $a=\"x\" x (8*1024*1024);"),
        ];
        for (interpreter, code) in probes {
            let ok = std::process::Command::new(interpreter)
                .arg("-c")
                .arg(code)
                .status()
                .map_or(false, |status| status.success());
            if ok {
                return Some(format!("{interpreter} -c '{code}'"));
            }
        }
        None
    }
}
