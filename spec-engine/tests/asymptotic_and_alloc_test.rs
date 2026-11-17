#![allow(unsafe_code)]

//! Integration tests for ARCH-07 (asymptotic complexity) and ARCH-08 (allocation tracking).

use asep::harness::{AsymptoticAnalyzer, ScopedAllocationReport, TrackingAllocator};
use std::alloc::{GlobalAlloc, Layout};

fn binary_search_iters(arr: &[i32], target: i32) -> u64 {
    let mut lo = 0usize;
    let mut hi = arr.len();
    let mut iters = 0u64;
    while lo < hi {
        iters += 1;
        let mid = lo + (hi - lo) / 2;
        if arr[mid] <= target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    iters
}

#[test]
fn test_asymptotic_analyzer_binary_search_log_n() {
    let analyzer = AsymptoticAnalyzer::new();
    let sizes = [100usize, 200, 400, 800, 1600];

    let slope = analyzer
        .assert_complexity(&sizes, |n| {
            let arr: Vec<i32> = (0..n as i32).collect();
            binary_search_iters(&arr, -1)
        }, 0.35)
        .unwrap();

    assert!(slope <= 0.35, "binary search slope {slope} exceeds 0.35");
}

#[test]
fn test_asymptotic_analyzer_linear_scan_o_n() {
    let analyzer = AsymptoticAnalyzer::new();
    let sizes = [50usize, 100, 200, 400, 800];

    let slope = analyzer
        .assert_complexity(&sizes, |n| {
            let arr = vec![0i32; n];
            let mut steps = 0u64;
            for _ in &arr {
                steps += 1;
            }
            steps
        }, 1.2)
        .unwrap();

    assert!(slope <= 1.2, "linear scan slope {slope} exceeds 1.2");
}

#[test]
fn test_asymptotic_analyzer_quadratic_bubble_sort_detect_fail() {
    let analyzer = AsymptoticAnalyzer::new();
    let sizes = [50usize, 100, 200, 400, 800];

    let result = analyzer.assert_complexity(&sizes, |n| {
        let mut arr: Vec<i32> = (0..n as i32).rev().collect();
        let mut steps = 0u64;
        for i in 0..n {
            for j in 0..n - 1 - i {
                steps += 1;
                if arr[j] > arr[j + 1] {
                    arr.swap(j, j + 1);
                }
            }
        }
        steps
    }, 1.3);

    assert!(result.is_err(), "quadratic bubble sort should fail complexity assert, got {result:?}");
}

#[test]
fn test_alloc_tracker_scoped_accounting_and_leak_detection() {
    let tracker = TrackingAllocator::system();
    let layout = Layout::from_size_align(64, 8).unwrap();

    let (value, report): (usize, ScopedAllocationReport) = tracker.measure_scope(|| {
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        unsafe { tracker.dealloc(ptr, layout) };
        42usize
    });
    assert_eq!(value, 42);
    assert_eq!(report.start_bytes, 0);
    assert_eq!(report.end_bytes, 0);
    assert_eq!(report.net_bytes, 0);
    assert_eq!(report.alloc_count, 1);
    assert_eq!(report.peak_bytes, 64);

    assert!(tracker.assert_zero_leak(0).is_ok());

    let leaked = tracker.measure_scope(|| {
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        ptr
    });
    assert_eq!(leaked.1.start_bytes, 0);
    assert_eq!(leaked.1.end_bytes, 64);
    assert_eq!(leaked.1.net_bytes, 64);
    assert_eq!(leaked.1.alloc_count, 1);

    assert!(tracker.assert_zero_leak(0).is_err());

    unsafe { tracker.dealloc(leaked.0, layout) };
    assert!(tracker.assert_zero_leak(0).is_ok());
}
