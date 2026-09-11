//! Connect the real computational-geometry allocator hook to QDNF tests.
//!
//! The QDNF harness `record_alloc` counter is explicit and cannot prove
//! dependency heap activity. Qualification uses `CountingAllocator`.

#![cfg(test)]

use crate::specialized_libs::computational_geometry::allocation_counter::{
    AllocGuard, AllocSnapshot,
};

/// Issued alloc calls, dealloc calls and bytes while `f` ran on this thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterceptReport {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub bytes_allocated: u64,
}

impl InterceptReport {
    #[inline]
    pub const fn had_heap_touch(self) -> bool {
        self.alloc_calls > 0
    }
}

/// Measure `f` using the installed global counting allocator.
pub fn measure<F: FnOnce()>(f: F) -> InterceptReport {
    let start = AllocSnapshot::now();
    let guard = AllocGuard::begin("qdnf-intercept", true);
    f();
    // check() clears measuring; ignore zero-heap assertion — we want the delta.
    let _ = guard.check();
    let end = AllocSnapshot::now();
    InterceptReport {
        alloc_calls: end.alloc_calls.saturating_sub(start.alloc_calls),
        dealloc_calls: end.dealloc_calls.saturating_sub(start.dealloc_calls),
        bytes_allocated: end.bytes_allocated.saturating_sub(start.bytes_allocated),
    }
}

pub fn measure_on_worker<F>(f: F) -> InterceptReport
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(move || measure(f))
        .join()
        .expect("qdnf intercept worker")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intercept_detects_intentional_allocation() {
        let report = measure(|| {
            let _leak_detector = vec![0u8; 64];
        });
        assert!(report.had_heap_touch());
        assert!(report.alloc_calls >= 1);
        assert!(report.bytes_allocated >= 64);
    }

    #[test]
    fn intercept_counts_alloc_dealloc_pairs() {
        let report = measure(|| {
            let v = vec![1u8, 2, 3, 4];
            drop(v);
        });
        assert!(report.alloc_calls >= 1);
        assert!(report.dealloc_calls >= 1);
    }

    #[test]
    fn intercept_observes_worker_thread() {
        let report = measure_on_worker(|| {
            let _v = vec![0u8; 32];
        });
        assert!(report.had_heap_touch());
    }

    #[test]
    fn empty_stack_work_need_not_allocate() {
        let report = measure(|| {
            let mut buf = [0u8; 32];
            buf[0] = 1;
            let _ = buf[0];
        });
        assert_eq!(report.alloc_calls, 0);
        assert_eq!(report.bytes_allocated, 0);
    }
}
