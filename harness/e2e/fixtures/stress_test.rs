//! Stress fixture for the SPSC lock-free ring buffer (P5 bug hunt).
//!
//! This file is copied unchanged into the dummy project's `tests/`
//! directory. Contract with the dummy project:
//!   - its crate exposes `SpscRing` at its root (expected crate name
//!     `spsc_ring`; adjust the import below to `[package].name`)
//!   - `SpscRing::new(capacity: usize)` where capacity is a power of two
//!   - `push(&self, u64) -> Result<(), PushError>` (ring full -> Err)
//!   - `pop(&self) -> Option<u64>` (empty -> None)
//!   - payload `u64`; safe Rust only
//!
//! Reads the run seed from `manifest.json` (P0 bootstrap), runs
//! `100_000` push/pop operations, hooks `thread::yield_now()` at
//! seed-given points (complements `loom`), and runs a watchdog that
//! aborts after `TIMEOUT_SECS` seconds so a hang/livelock is a hard
//! FAIL, not an infinite wait. Prints one line per operation to stdout
//! for the artifact log. Runs unchanged under `cargo test` (debug) and
//! `cargo test --release`.

use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use spsc_ring::SpscRing;

const ITERATIONS: u64 = 100_000;
const CAPACITY: usize = 1024; // power of two
const TIMEOUT_SECS: u64 = 30; // watchdog: abort the run after this many seconds
const YIELD_MOD: u64 = 3; // yield hook fires every N-th seed-marked step
const MANIFEST: &str = "manifest.json";

fn seed_from_manifest() -> u64 {
    const DEFAULT_SEED: u64 = 0x5EED_5EED;
    // Manual override for ad-hoc runs.
    if let Ok(value) = env::var("STRESS_SEED") {
        if let Ok(seed) = value.trim().parse::<u64>() {
            return seed;
        }
    }
    // P0 manifest.json carries the run seed written at bootstrap.
    if let Ok(text) = fs::read_to_string(MANIFEST) {
        if let Some(pos) = text.find("\"seed\"") {
            let digits: String = text[pos + "\"seed\"".len()..]
                .chars()
                .skip_while(|c| *c != '-' && !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(seed) = digits.parse::<u64>() {
                return seed;
            }
        }
    }
    DEFAULT_SEED
}

fn yield_hook(seed: u64, counter: u64) {
    // Seed-given deterministic interleave hook.
    if seed.wrapping_add(counter) % YIELD_MOD == 0 {
        thread::yield_now();
    }
}

#[test]
fn stress_fifo_no_loss_no_dup() {
    let seed = seed_from_manifest();
    let start = Instant::now();
    let ring = SpscRing::new(CAPACITY);

    let produced = AtomicU64::new(0);
    let consumed = AtomicU64::new(0);
    let done = AtomicBool::new(false);

    // Watchdog: a hang/livelock aborts the run instead of waiting forever.
    let watchdog = thread::spawn(move || {
        thread::sleep(Duration::from_secs(TIMEOUT_SECS));
        eprintln!("stress watchdog: timeout after {TIMEOUT_SECS}s");
        std::process::exit(124);
    });

    let producer = thread::spawn(move || {
        let mut next: u64 = 0;
        while next < ITERATIONS {
            match ring.push(next) {
                Ok(()) => {
                    println!("push {next} ok");
                    produced.fetch_add(1, Ordering::Relaxed);
                    next += 1;
                    yield_hook(seed, next);
                }
                Err(_) => {
                    // Full ring: back off and retry, never drop a payload.
                    println!("push {next} full");
                    yield_hook(seed, next);
                    thread::yield_now();
                }
            }
        }
        done.store(true, Ordering::Release);
        println!("producer done");
    });

    let consumer = thread::spawn(move || {
        let mut values = Vec::with_capacity(ITERATIONS as usize);
        let mut empty_retries = 0u64;
        loop {
            match ring.pop() {
                Some(value) => {
                    println!("pop {value} ok");
                    values.push(value);
                    consumed.fetch_add(1, Ordering::Relaxed);
                    empty_retries = 0;
                    yield_hook(seed, value);
                }
                None => {
                    if done.load(Ordering::Acquire) {
                        empty_retries += 1;
                        if empty_retries > 16 {
                            break; // producer finished and ring is drained
                        }
                    } else {
                        empty_retries = 0;
                    }
                    println!("pop empty");
                    yield_hook(seed, empty_retries);
                    thread::yield_now();
                }
            }
        }
        values
    });

    producer.join().expect("producer thread panicked");
    let values = consumer.join().expect("consumer thread panicked");

    let elapsed = start.elapsed();

    // FIFO contract: no payload lost, none duplicated, no reordering.
    let expected: Vec<u64> = (0..ITERATIONS).collect();
    assert_eq!(
        values.len(),
        ITERATIONS as usize,
        "stress: payload loss (consumer count != iterations)"
    );
    assert_eq!(
        values, expected,
        "stress: FIFO contract broken (loss, duplication, or reordering)"
    );
    assert_eq!(
        consumed.load(Ordering::Relaxed),
        ITERATIONS,
        "stress: consumed counter mismatch"
    );

    println!(
        "stress done: seed={seed} iterations={ITERATIONS} capacity={CAPACITY} \
         elapsed={elapsed:?}"
    );
}
