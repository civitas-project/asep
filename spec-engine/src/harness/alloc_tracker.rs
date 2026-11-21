//! Zero-allocation memory allocation and leak tracker (ARCH-08).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Wraps allocator `A` (default `std::alloc::System`) and atomically counts
/// every allocation it serves so callers can enforce byte budgets and detect
/// leaks without installing a `#[global_allocator]`.
pub struct TrackingAllocator<A = System> {
    inner: A,
    current_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
    total_allocated: AtomicUsize,
    alloc_count: AtomicUsize,
    dealloc_count: AtomicUsize,
}

impl<A> TrackingAllocator<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            current_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
            alloc_count: AtomicUsize::new(0),
            dealloc_count: AtomicUsize::new(0),
        }
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes.load(Ordering::Relaxed)
    }

    pub fn peak_bytes(&self) -> usize {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    pub fn total_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }

    pub fn alloc_count(&self) -> usize {
        self.alloc_count.load(Ordering::Relaxed)
    }

    pub fn dealloc_count(&self) -> usize {
        self.dealloc_count.load(Ordering::Relaxed)
    }

    /// Resets the peak high-water mark to the current byte count.
    pub fn reset_peak(&self) {
        self.peak_bytes
            .store(self.current_bytes(), Ordering::Relaxed);
    }

    pub fn assert_peak_budget(&self, max_bytes: usize) -> Result<(), String> {
        let peak = self.peak_bytes();
        if peak > max_bytes {
            return Err(format!(
                "peak allocation budget exceeded: peak {} bytes, max {} bytes",
                peak, max_bytes
            ));
        }
        Ok(())
    }

    pub fn assert_zero_leak(&self, baseline_bytes: usize) -> Result<(), String> {
        let current = self.current_bytes();
        if current == baseline_bytes {
            return Ok(());
        }
        let (delta, direction) = match current > baseline_bytes {
            true => (current - baseline_bytes, "above"),
            false => (baseline_bytes - current, "below"),
        };
        Err(format!(
            "memory leak detected: current {} bytes differs from baseline {} bytes by {} bytes {}",
            current, baseline_bytes, delta, direction
        ))
    }

    pub fn measure_scope<F, R>(&self, f: F) -> (R, ScopedAllocationReport)
    where
        F: FnOnce() -> R,
    {
        let start_bytes = self.current_bytes();
        let start_count = self.alloc_count();
        self.reset_peak();
        let result = f();
        let end_bytes = self.current_bytes();
        let report = ScopedAllocationReport {
            start_bytes,
            end_bytes,
            net_bytes: net_diff(start_bytes, end_bytes),
            alloc_count: self.alloc_count() - start_count,
            peak_bytes: self.peak_bytes(),
        };
        (result, report)
    }
}

impl TrackingAllocator {
    pub const fn system() -> TrackingAllocator<System> {
        TrackingAllocator::new(System)
    }
}

#[allow(unsafe_code)]
unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if ptr.is_null() {
            return ptr;
        }
        let size = layout.size();
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
        let current = self.current_bytes.fetch_add(size, Ordering::Relaxed) + size;
        self.peak_bytes.fetch_max(current, Ordering::Relaxed);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.dealloc_count.fetch_add(1, Ordering::Relaxed);
        self.current_bytes.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if new_ptr.is_null() {
            return new_ptr;
        }
        let old_size = layout.size();
        if new_size < old_size {
            let diff = old_size - new_size;
            self.current_bytes.fetch_sub(diff, Ordering::Relaxed);
            return new_ptr;
        }
        let diff = new_size - old_size;
        let prev = self.current_bytes.fetch_add(diff, Ordering::Relaxed);
        self.total_allocated.fetch_add(diff, Ordering::Relaxed);
        self.peak_bytes.fetch_max(prev + diff, Ordering::Relaxed);
        new_ptr
    }
}

/// Allocation snapshot for a scoped measurement: peak is the high-water mark
/// observed while `f` ran (the peak counter is reset at scope start).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScopedAllocationReport {
    pub start_bytes: usize,
    pub end_bytes: usize,
    pub net_bytes: isize,
    pub alloc_count: usize,
    pub peak_bytes: usize,
}

fn net_diff(start: usize, end: usize) -> isize {
    if end < start {
        return -((start - end) as isize);
    }
    (end - start) as isize
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    fn test_layout(size: usize) -> Layout {
        Layout::from_size_align(size, 8).unwrap()
    }

    #[test]
    fn alloc_and_dealloc_update_counters() {
        let tracker = TrackingAllocator::system();
        let layout = test_layout(64);
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!(tracker.alloc_count(), 1);
        assert_eq!(tracker.current_bytes(), 64);
        assert_eq!(tracker.total_allocated(), 64);
        assert_eq!(tracker.peak_bytes(), 64);
        assert_eq!(tracker.dealloc_count(), 0);

        unsafe { tracker.dealloc(ptr, layout) };
        assert_eq!(tracker.dealloc_count(), 1);
        assert_eq!(tracker.current_bytes(), 0);
        assert_eq!(tracker.total_allocated(), 64);
    }

    #[test]
    fn peak_is_high_water_mark_survives_dealloc() {
        let tracker = TrackingAllocator::system();
        let p1 = unsafe { tracker.alloc(test_layout(100)) };
        let p2 = unsafe { tracker.alloc(test_layout(50)) };
        assert!(!p1.is_null() && !p2.is_null());
        assert_eq!(tracker.current_bytes(), 150);
        assert_eq!(tracker.peak_bytes(), 150);

        unsafe { tracker.dealloc(p1, test_layout(100)) };
        assert_eq!(tracker.current_bytes(), 50);
        assert_eq!(tracker.peak_bytes(), 150);

        tracker.reset_peak();
        assert_eq!(tracker.peak_bytes(), 50);

        let p3 = unsafe { tracker.alloc(test_layout(30)) };
        assert!(!p3.is_null());
        assert_eq!(tracker.current_bytes(), 80);
        assert_eq!(tracker.peak_bytes(), 80);

        unsafe {
            tracker.dealloc(p2, test_layout(50));
            tracker.dealloc(p3, test_layout(30));
        }
        assert_eq!(tracker.current_bytes(), 0);
    }

    #[test]
    fn assert_peak_budget_pass_and_fail() {
        let tracker = TrackingAllocator::system();
        let layout = test_layout(10);
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        assert!(tracker.assert_peak_budget(100).is_ok());
        assert!(tracker.assert_peak_budget(9).is_err());
        unsafe { tracker.dealloc(ptr, layout) };
    }

    #[test]
    fn assert_zero_leak_pass_and_fail() {
        let tracker = TrackingAllocator::system();
        assert!(tracker.assert_zero_leak(0).is_ok());

        let layout = test_layout(16);
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        assert!(tracker.assert_zero_leak(0).is_err());
        unsafe { tracker.dealloc(ptr, layout) };
        assert!(tracker.assert_zero_leak(0).is_ok());
    }

    #[test]
    fn measure_scope_reports_allocation_delta() {
        let tracker = TrackingAllocator::system();
        let layout = test_layout(32);
        let (value, report) = tracker.measure_scope(|| {
            let ptr = unsafe { tracker.alloc(layout) };
            assert!(!ptr.is_null());
            unsafe { tracker.dealloc(ptr, layout) };
            7usize
        });
        assert_eq!(value, 7);
        assert_eq!(report.start_bytes, 0);
        assert_eq!(report.end_bytes, 0);
        assert_eq!(report.net_bytes, 0);
        assert_eq!(report.alloc_count, 1);
        assert_eq!(report.peak_bytes, 32);
    }

    #[test]
    fn measure_scope_reports_net_leak() {
        let tracker = TrackingAllocator::system();
        let layout = test_layout(64);
        let (ptr, report) = tracker.measure_scope(|| {
            let ptr = unsafe { tracker.alloc(layout) };
            assert!(!ptr.is_null());
            ptr
        });
        let _ = ptr;
        assert_eq!(report.start_bytes, 0);
        assert_eq!(report.end_bytes, 64);
        assert_eq!(report.net_bytes, 64);
        assert_eq!(report.alloc_count, 1);
        assert_eq!(report.peak_bytes, 64);
    }

    #[test]
    fn realloc_atomic_accounting() {
        let tracker = TrackingAllocator::system();
        let layout = test_layout(32);
        let ptr = unsafe { tracker.alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!(tracker.current_bytes(), 32);
        assert_eq!(tracker.total_allocated(), 32);

        let grown = unsafe { tracker.realloc(ptr, layout, 96) };
        assert!(!grown.is_null());
        assert_eq!(tracker.current_bytes(), 96);
        assert_eq!(tracker.total_allocated(), 96);
        assert_eq!(tracker.peak_bytes(), 96);

        let shrunk = unsafe { tracker.realloc(grown, test_layout(96), 48) };
        assert!(!shrunk.is_null());
        assert_eq!(tracker.current_bytes(), 48);
        assert_eq!(tracker.total_allocated(), 96);
        assert_eq!(tracker.peak_bytes(), 96);

        unsafe { tracker.dealloc(shrunk, test_layout(48)) };
        assert_eq!(tracker.current_bytes(), 0);
        assert_eq!(tracker.total_allocated(), 96);
    }
}
