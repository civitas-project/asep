//! Loom template for the SPSC lock-free ring buffer (P5, consumed by Task 4).
//!
//! Systematic interleaving exploration of one producer and one consumer
//! thread under `loom::model`. Atomics are feature-gated:
//!   - `loom-test` feature: `loom::sync::atomic` (systematic schedules)
//!   - otherwise:           `std::sync::atomic` (plain sanity run)
//!
//! Contract with the dummy project: its crate exposes `SpscRing` at its
//! root (expected crate name `spsc_ring`) with the exact public API:
//!   `SpscRing::new(capacity)`, `push(u64) -> Result<(), PushError>`,
//!   `pop() -> Option<u64>`; capacity is a power of two; payload `u64`;
//!   safe Rust only.
//!
//! Run with: LOOM_MAX_BRANCHES=4096 cargo test --features loom-test --test loom_test

#[cfg(feature = "loom-test")]
use loom::sync::atomic::*;
#[cfg(not(feature = "loom-test"))]
#[allow(unused_imports)]
use std::sync::atomic::*; // sanity build without loom

use std::sync::Arc;
use spsc_ring::SpscRing;

const CAPACITY: usize = 4; // power of two
const VALUES: &[u64] = &[1, 2, 3];

#[cfg(feature = "loom-test")]
#[test]
fn loom_sp_sc_fifo_no_loss_no_dup() {
    loom::model(|| {
        let ring: Arc<SpscRing> = Arc::new(SpscRing::new(CAPACITY));
        let log: Arc<loom::sync::Mutex<Vec<u64>>> =
            Arc::new(loom::sync::Mutex::new(Vec::new()));

        let producer = loom::thread::spawn({
            let ring = Arc::clone(&ring);
            move || {
                for &v in VALUES {
                    while ring.push(v).is_err() {
                        // Full ring: spin until a slot frees.
                        std::thread::yield_now();
                    }
                }
            }
        });

        let consumer = loom::thread::spawn({
            let ring = Arc::clone(&ring);
            let log = Arc::clone(&log);
            move || {
                while log.lock().len() < VALUES.len() {
                    if let Some(v) = ring.pop() {
                        log.lock().push(v);
                    } else {
                        std::thread::yield_now();
                    }
                }
            }
        });

        producer.join().expect("producer panicked under loom");
        consumer.join().expect("consumer panicked under loom");

        // Every explored schedule must reproduce the push sequence:
        // no loss, no duplication, no reordering.
        assert_eq!(*log.lock(), VALUES);
    });
}
