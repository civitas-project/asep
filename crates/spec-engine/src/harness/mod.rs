//! Empirical asymptotic complexity harness (ARCH-07), zero-allocation tracking (ARCH-08),
//! and multi-language CLI process harness (ARCH-10).

pub mod alloc_tracker;
pub mod asymptotic;
pub mod e2e;
pub mod process;

pub use alloc_tracker::{ScopedAllocationReport, TrackingAllocator};
pub use asymptotic::AsymptoticAnalyzer;
