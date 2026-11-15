//! All focused checks use the same actual thread-local allocator as core tests.
use crate::core_allocation_counter::{AllocGuard, AllocSnapshot};

pub fn measured(f: impl FnOnce()) -> usize {
    let start = AllocSnapshot::now();
    let guard = AllocGuard::begin("focused measured scope", true);
    f();
    let calls = AllocSnapshot::now().alloc_delta(start);
    // Mark checked and stop counting; callers assert the returned raw count.
    // Positive controls deliberately allocate, so this diagnostic is not a panic.
    let _ = guard.check();
    usize::try_from(calls).unwrap()
}
#[test]
fn counter_positive_control() {
    assert!(measured(|| { std::hint::black_box(vec![7u8; 257]); }) > 0);
}
