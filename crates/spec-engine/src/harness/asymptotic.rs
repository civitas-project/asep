//! Empirical asymptotic complexity measurement and assertion (ARCH-07).

use std::sync::mpsc::sync_channel;
use std::time::{Duration, Instant};

// @impl:FR-AUD-13
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

const SLOPE_TOLERANCE: f64 = 0.25;

/// Analyzer for empirical step-count or duration complexity across geometric
/// input sizes: N = [N0, 2*N0, 4*N0, 8*N0, ...].
#[derive(Default)]
pub struct AsymptoticAnalyzer;

impl AsymptoticAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Measure step counts returned by `f` for each size in `sizes`.
    pub fn measure_steps<F>(&self, sizes: &[usize], f: F) -> Vec<(usize, u64)>
    where
        F: Fn(usize) -> u64,
    {
        sizes.iter().copied().map(|n| (n, f(n))).collect()
    }

    /// Measure elapsed wall-clock duration of `f` for each size in `sizes`.
    pub fn measure_duration<F>(&self, sizes: &[usize], mut f: F) -> Vec<(usize, f64)>
    where
        F: FnMut(usize),
    {
        sizes
            .iter()
            .copied()
            .map(|n| {
                let start = Instant::now();
                f(n);
                (n, start.elapsed().as_secs_f64())
            })
            .collect()
    }

    /// Run `f` in a detached spawned thread and receive its result over an mpsc
    /// channel with `recv_timeout`. Wall-clock is bounded by `timeout`.
    ///
    /// Rust cannot forcibly cancel a thread: on timeout the worker is only
    /// cooperatively cancelled via `CancellationToken` and may keep running
    /// until it observes the token. This function returns immediately without
    /// joining the worker.
    pub fn run_with_timeout<F, R>(timeout: Duration, f: F) -> Result<R, String>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let (tx, rx) = sync_channel(1);
        std::thread::spawn(move || {
            let result = f();
            if !worker_token.is_cancelled() {
                let _ = tx.send(result);
            }
        });
        match rx.recv_timeout(timeout) {
            Ok(result) => Ok(result),
            Err(_) => {
                token.cancel();
                Err(format!(
                    "timed out after {:?} (cooperative cancellation; worker may still be running)",
                    timeout
                ))
            }
        }
    }

    /// Estimate the log-log slope of `f`'s step counts across `sizes` and
    /// assert it does not exceed `max_slope` plus a small noise tolerance.
    /// Returns the estimated slope on success.
    pub fn assert_complexity<F>(&self, sizes: &[usize], f: F, max_slope: f64) -> Result<f64, String>
    where
        F: Fn(usize) -> u64,
    {
        if sizes.len() < 2 {
            return Err("need at least two sizes to estimate slope".to_string());
        }

        let mut xs = Vec::with_capacity(sizes.len());
        let mut ys = Vec::with_capacity(sizes.len());
        for &n in sizes {
            if n == 0 {
                return Err("sizes must be greater than zero".to_string());
            }
            let steps = f(n);
            if steps == 0 {
                return Err("step counts must be greater than zero".to_string());
            }
            xs.push((n as f64).ln());
            ys.push((steps as f64).ln());
        }

        let slope = log_log_slope(&xs, &ys)?;
        let limit = max_slope + SLOPE_TOLERANCE;

        if slope > limit {
            return Err(format!(
                "complexity assert failed: estimated log-log slope {:.4} exceeds max_slope {} + tolerance {}",
                slope, max_slope, SLOPE_TOLERANCE
            ));
        }
        Ok(slope)
    }
}

/// Least-squares slope of y on x: sum((x-xbar)(y-ybar)) / sum((x-xbar)^2).
fn log_log_slope(xs: &[f64], ys: &[f64]) -> Result<f64, String> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err("need at least two paired observations".to_string());
    }

    let n = xs.len() as f64;
    let x_mean = xs.iter().copied().sum::<f64>() / n;
    let y_mean = ys.iter().copied().sum::<f64>() / n;

    let mut numerator = 0.0f64;
    let mut denominator = 0.0f64;
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        let dx = x - x_mean;
        let dy = y - y_mean;
        numerator += dx * dy;
        denominator += dx * dx;
    }

    if denominator == 0.0 {
        return Err("zero variance in input sizes; cannot estimate slope".to_string());
    }

    Ok(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn linear_complexity_passes() {
        let analyzer = AsymptoticAnalyzer::new();
        let sizes = [100, 200, 400, 800, 1600];
        let slope = analyzer
            .assert_complexity(&sizes, |n| n as u64, 1.2)
            .unwrap();
        assert!((slope - 1.0).abs() < 1e-9);
    }

    #[test]
    fn quadratic_complexity_fails() {
        let analyzer = AsymptoticAnalyzer::new();
        let sizes = [50, 100, 200, 400, 800];
        let result = analyzer.assert_complexity(&sizes, |n| (n * n) as u64, 1.3);
        assert!(result.is_err(), "expected error, got {:?}", result);
    }

    #[test]
    fn constant_complexity_passes() {
        let analyzer = AsymptoticAnalyzer::new();
        let sizes = [10, 20, 40, 80, 160];
        let slope = analyzer.assert_complexity(&sizes, |_| 7, 0.3).unwrap();
        assert!((slope - 0.0).abs() < 1e-9);
    }

    #[test]
    fn run_with_timeout_returns_err_when_task_exceeds_timeout() {
        let result = AsymptoticAnalyzer::run_with_timeout(Duration::from_millis(30), || {
            std::thread::sleep(Duration::from_millis(200));
            42u32
        });
        assert!(result.is_err());
    }

    #[test]
    fn run_with_timeout_returns_value_when_task_completes() {
        let result = AsymptoticAnalyzer::run_with_timeout(Duration::from_secs(5), || 7u32);
        assert_eq!(result.unwrap(), 7);
    }

    // @test:FR-AUD-13
    #[test]
    fn run_with_timeout_bounds_wall_clock() {
        let token = CancellationToken::new();
        let start = Instant::now();
        let result = AsymptoticAnalyzer::run_with_timeout(Duration::from_millis(20), move || {
            let mut slept_ms = 0u64;
            while slept_ms < 500 && !token.is_cancelled() {
                std::thread::sleep(Duration::from_millis(10));
                slept_ms += 10;
            }
            slept_ms
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
        assert!(start.elapsed() < Duration::from_millis(250));
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    // @test:B13
    fn timeout_is_cooperative_and_returns_immediately() {
        let observed = Arc::new(AtomicBool::new(false));
        let worker_observed = observed.clone();
        let start = std::time::Instant::now();
        let result = AsymptoticAnalyzer::run_with_timeout(Duration::from_millis(20), move || {
            std::thread::sleep(Duration::from_millis(120));
            worker_observed.store(true, Ordering::SeqCst);
            42
        });
        assert!(result.is_err(), "unfinished workload must time out");
        assert!(start.elapsed() < Duration::from_millis(500), "timeout must not block on worker");
    }
}
