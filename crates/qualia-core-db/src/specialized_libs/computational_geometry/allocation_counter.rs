//! P10.3 — Allocation counter for zero-heap hot-path verification.
//!
//! Provides a `#[cfg(test)]`-only custom global allocator that counts
//! `alloc` / `dealloc` / `realloc` calls **on the measuring thread only**,
//! plus a guard that asserts a closure performs zero heap allocations. This
//! is the enforcement side of the `AllocationClass::HotZeroHeap` claim in
//! `capability_manifests.rs`: a hot path op that claims zero-heap MUST verify
//! it here.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::specialized_libs::computational_geometry::allocation_counter::*;
//!
//! #[test]
//! fn orientation_2_is_zero_heap() {
//!     assert_zero_alloc("orientation_2", || {
//!         let _ = orientation_2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(0.5, 0.5));
//!     });
//! }
//! ```
//!
//! ## How it works (thread-local, parallel-safe)
//!
//! A `std::alloc::System` wrapper (`CountingAllocator`) is installed as the
//! global allocator when this module is compiled into a test build. The
//! allocator increments a **thread-local** counter only when the current
//! thread has set its **thread-local `MEASURING` flag**. The RAII guard
//! (`AllocGuard`) sets the flag on the current thread on creation and clears
//! it on drop, snapshotting the thread-local counter on entry and checking
//! the delta on exit.
//!
//! Because both the counter and the flag are thread-local, allocations from
//! OTHER test threads running in parallel are invisible — each thread only
//! counts its own allocations while it is actively measuring. This makes the
//! zero-heap tests reliable under parallel test execution
//! (`--test-threads=N`, the default); no `--test-threads=1` requirement.
//!
//! ## Caveats
//!
//! - `assert_zero_alloc` counts net allocations (alloc − dealloc). A closure
//!   that allocates and then frees within its body will show zero net — but
//!   that is still a heap touch (the hot path must not allocate at all, even
//!   transiently). Use `assert_no_alloc_calls` for the stricter check that
//!   counts raw `alloc` calls.
//! - The first call to an op may trigger lazy initialization (e.g. thread-local
//!   state) that allocates. Tests should warm up the op once before measuring.
//! - The thread-local counters are `Cell<u64>` (per-thread, no atomics needed
//!   since only the owning thread reads/writes them). The allocator's
//!   `thread_local!` accessors are accessed on every alloc call, but the
//!   fast path (flag not set) is a single `Cell::get` + branch — negligible
//!   overhead outside test builds.

#![cfg(test)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// The counting allocator. Wraps `std::alloc::System` and increments
/// thread-local counters on every `alloc` / `dealloc` / `realloc` call — but
/// ONLY when the current thread has set its `MEASURING` flag.
pub struct CountingAllocator;

// Per-thread counters. Only the owning thread accesses these (via the
// thread_local! accessor), so `Cell<u64>` is sufficient — no atomics needed.
thread_local! {
    static ALLOC_CALLS: Cell<u64> = const { Cell::new(0) };
    static DEALLOC_CALLS: Cell<u64> = const { Cell::new(0) };
    static BYTES_ALLOCATED: Cell<u64> = const { Cell::new(0) };
    /// When true, the current thread's allocations are counted. Set by
    /// `AllocGuard` on creation and cleared on drop.
    static MEASURING: Cell<bool> = const { Cell::new(false) };
}

/// Increment the current thread's alloc counter iff it is measuring.
#[inline]
fn count_alloc(layout: &Layout) {
    MEASURING.with(|m| {
        if m.get() {
            ALLOC_CALLS.with(|c| c.set(c.get().wrapping_add(1)));
            BYTES_ALLOCATED.with(|b| b.set(b.get().wrapping_add(layout.size() as u64)));
        }
    });
}

/// Increment the current thread's dealloc counter iff it is measuring.
#[inline]
fn count_dealloc() {
    MEASURING.with(|m| {
        if m.get() {
            DEALLOC_CALLS.with(|c| c.set(c.get().wrapping_add(1)));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc(&layout);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count_dealloc();
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc(&layout);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        // realloc is an alloc + dealloc from the counter's perspective.
        count_alloc(&Layout::from_size_align(new_size, old_layout.align()).unwrap_or(old_layout));
        count_dealloc();
        System.realloc(ptr, old_layout, new_size)
    }
}

/// Install `CountingAllocator` as the global allocator.
#[cfg(test)]
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Snapshot of the current thread's allocation counters at a point in time.
#[derive(Debug, Clone, Copy)]
pub struct AllocSnapshot {
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub bytes_allocated: u64,
}

impl AllocSnapshot {
    /// Capture the current thread's counter values. Reads the thread-local
    /// counters on the calling thread.
    pub fn now() -> Self {
        Self {
            alloc_calls: ALLOC_CALLS.with(|c| c.get()),
            dealloc_calls: DEALLOC_CALLS.with(|c| c.get()),
            bytes_allocated: BYTES_ALLOCATED.with(|b| b.get()),
        }
    }

    /// The delta in raw `alloc` calls between two snapshots.
    pub fn alloc_delta(&self, earlier: AllocSnapshot) -> u64 {
        self.alloc_calls.saturating_sub(earlier.alloc_calls)
    }
}

/// RAII guard that snapshots the current thread's counters on creation,
/// sets the thread-local `MEASURING` flag (so the allocator counts this
/// thread's allocations), and asserts on drop (clearing the flag).
///
/// Created by `assert_zero_alloc` / `assert_no_alloc_calls`. The assertion
/// runs on `Drop` so it fires even if the closure under test panics (the
/// panic is caught by the test harness before `Drop` runs, but if the guard
/// is dropped normally the assertion is checked).
pub struct AllocGuard {
    start: AllocSnapshot,
    strict: bool, // true = count raw alloc calls; false = count net (alloc - dealloc)
    label: &'static str,
    checked: std::cell::Cell<bool>,
}

impl AllocGuard {
    /// Begin measuring on the current thread. `strict = true` counts raw
    /// alloc calls (the zero-heap rule: no alloc at all, even transiently).
    /// `strict = false` counts net allocations (alloc − dealloc).
    pub fn begin(label: &'static str, strict: bool) -> Self {
        let start = AllocSnapshot::now();
        // Set the measuring flag AFTER snapshotting, so the snapshot itself
        // isn't counted (it shouldn't allocate, but be defensive).
        MEASURING.with(|m| m.set(true));
        Self {
            start,
            strict,
            label,
            checked: std::cell::Cell::new(false),
        }
    }

    /// Check the assertion now (also called on Drop). Returns Ok if the
    /// assertion holds, Err with a diagnostic if it fails. Marks the guard
    /// as checked so `Drop` does not double-assert. Clears the measuring flag.
    pub fn check(&self) -> Result<(), String> {
        self.checked.set(true);
        // Clear the measuring flag before reading the final count, so any
        // allocation triggered by the snapshot read itself isn't counted.
        MEASURING.with(|m| m.set(false));
        let end = AllocSnapshot::now();
        if self.strict {
            let delta = end.alloc_delta(self.start);
            if delta != 0 {
                return Err(format!(
                    "{}: {} heap alloc call(s) detected — hot path must be zero-heap",
                    self.label, delta
                ));
            }
        } else {
            let net = (end.alloc_calls - end.dealloc_calls)
                .saturating_sub(self.start.alloc_calls - self.start.dealloc_calls);
            if net != 0 {
                return Err(format!(
                    "{}: {} net heap allocation(s) detected — expected zero net",
                    self.label, net
                ));
            }
        }
        Ok(())
    }
}

impl Drop for AllocGuard {
    fn drop(&mut self) {
        // Only assert on drop if check() was NOT already called (to avoid
        // double-panic when the caller already panicked on check()).
        // Also skip if we're already panicking (double-panic would abort).
        if !self.checked.get() {
            // Ensure the measuring flag is cleared even on panic.
            MEASURING.with(|m| m.set(false));
            if !std::thread::panicking() {
                if let Err(msg) = self.check() {
                    panic!("{}", msg);
                }
            }
        }
    }
}

/// Assert that a closure performs ZERO raw `alloc` calls (the strict
/// zero-heap rule — no allocation at all, even transiently).
///
/// This is the function P10.3's hot-path tests use. The closure should
/// warm up any lazy state BEFORE calling this function, so the measurement
/// captures only the hot-path op itself.
///
/// Thread-local: counts only the current thread's allocations while the
/// guard is active. Safe under parallel test execution.
pub fn assert_zero_alloc<F: FnOnce()>(label: &'static str, f: F) {
    let guard = AllocGuard::begin(label, true);
    f();
    guard.check().expect("zero-alloc assertion failed");
}

/// Assert that a closure performs zero NET allocations (alloc − dealloc).
/// Stricter than `assert_zero_alloc` is the zero-heap rule; this is the
/// weaker "no leak" check useful for cold builders that may allocate
/// transiently but must free everything.
pub fn assert_zero_net_alloc<F: FnOnce()>(label: &'static str, f: F) {
    let guard = AllocGuard::begin(label, false);
    f();
    guard.check().expect("zero-net-alloc assertion failed");
}

/// Get the current thread's raw alloc-call count (for custom assertions).
pub fn current_alloc_calls() -> u64 {
    ALLOC_CALLS.with(|c| c.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter itself must not allocate to read.
    #[test]
    fn snapshot_does_not_count_when_not_measuring() {
        // No guard active ⇒ measuring flag is false ⇒ allocations not counted.
        let before = AllocSnapshot::now();
        let _ = vec![0u8; 64]; // allocates
        let after = AllocSnapshot::now();
        assert_eq!(
            after.alloc_delta(before),
            0,
            "allocs outside a guard must not be counted"
        );
    }

    /// The counter MUST count allocations on the measuring thread inside a guard.
    #[test]
    fn guard_counts_allocs_on_measuring_thread() {
        let guard = AllocGuard::begin("test_guard", true);
        let _ = vec![0u8; 64]; // allocates
        let result = guard.check();
        assert!(
            result.is_err(),
            "expected the guard to detect the allocation"
        );
        let msg = result.unwrap_err();
        assert!(
            msg.contains("test_guard"),
            "error should name the label: {}",
            msg
        );
        assert!(
            msg.contains("zero-heap"),
            "error should mention zero-heap: {}",
            msg
        );
    }

    /// A closure that does not allocate must pass.
    #[test]
    fn zero_alloc_closure_passes() {
        let guard = AllocGuard::begin("no_alloc", true);
        let _x: u64 = 42; // stack, no heap
        let _y: [u8; 16] = [0; 16]; // stack, no heap
        guard.check().expect("no allocations ⇒ should pass");
    }

    /// The measuring flag is cleared after check(), so subsequent allocations
    /// outside a guard are not counted.
    #[test]
    fn flag_cleared_after_check() {
        {
            let guard = AllocGuard::begin("transient", true);
            let _ = vec![0u8; 16];
            let _ = guard.check(); // err, but we ignore it
        }
        // Now no guard is active; allocations should not be counted.
        let before = AllocSnapshot::now();
        let _ = vec![0u8; 64];
        let after = AllocSnapshot::now();
        assert_eq!(
            after.alloc_delta(before),
            0,
            "flag must be cleared after check"
        );
    }

    /// The measuring flag is cleared even if the closure panics (Drop clears it).
    #[test]
    fn flag_cleared_on_panic_via_drop() {
        let result = std::panic::catch_unwind(|| {
            let _guard = AllocGuard::begin("panic_test", true);
            panic!("intentional");
        });
        assert!(result.is_err());
        // After the panic, the guard's Drop should have cleared the flag.
        // Verify by checking that allocations are not counted.
        let before = AllocSnapshot::now();
        let _ = vec![0u8; 64];
        let after = AllocSnapshot::now();
        assert_eq!(
            after.alloc_delta(before),
            0,
            "flag must be cleared after panic-drop"
        );
    }
}
