// ─── Sticky 1-thread infer pool (native) ────────────────────────────────────
// Every `infer` previously did `thread::spawn` + `QTensorEngine::new()` which
// rebuilds wgpu pipelines (~seconds) even when the mmap is already resident.
// A size-1 rayon pool keeps a dedicated OS thread whose `thread_local` engine
// survives across jobs; same-path multi-turn / multi-prompt reuses the engine.
use crate::gguf_bridge::QTensorEngine;
use crate::inference::runtime::prepared::generation::ResidencyGenerationTracker;
use crate::inference::runtime::scheduler::queue::{BoundedIntakeQueue, IntakeError, IntakeRequest, IntakeState};
use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

/// Global residency generation tracker for the sticky infer engine.
pub static RESIDENCY_TRACKER: ResidencyGenerationTracker = ResidencyGenerationTracker::new(1);

pub struct StickyEngine {
    pub path: String,
    pub generation: u64,
    pub engine: QTensorEngine,
}

thread_local! {
    static ENGINE: RefCell<Option<StickyEngine>> = const { RefCell::new(None) };
}

pub fn pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            // The decode path is deliberately zero-heap (§6): it holds large
            // buffers on the stack — e.g. `[0f32; PREFILL_CHUNK_STACK_FLOATS]`
            // (2560·64 floats ≈ 640 KB) — and runs a full transformer forward
            // pass whose (debug-unoptimised) call chain adds several MB more.
            // rayon's default worker stack (~2 MB) overflows on the real decode
            // path (the `qualia-infer-0` STACK_OVERFLOW), on device as much as in
            // tests. Reserve a generous stack for this single dedicated worker;
            // on 64-bit the reservation is address space only (committed on
            // demand), so it costs nothing until touched.
            .stack_size(64 * 1024 * 1024)
            .thread_name(|i| format!("qualia-infer-{i}"))
            .build()
            .expect("qualia sticky infer pool")
    })
}

/// Advance the residency generation, invalidating cached sticky engine handles
/// across all worker threads.
pub fn invalidate_sticky_residency() -> u64 {
    RESIDENCY_TRACKER.advance_generation()
}

/// Read the current residency generation number.
pub fn current_residency_generation() -> u64 {
    RESIDENCY_TRACKER.generation()
}

/// Borrow-or-reload the sticky engine for `path`, then run `f`.
/// Validates both pathname equality and residency generation freshness.
pub fn with_engine<R>(
    path: &str,
    mut load: impl FnMut(&mut QTensorEngine),
    f: impl FnOnce(&mut QTensorEngine) -> R,
) -> R {
    let current_gen = RESIDENCY_TRACKER.generation();
    ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let reload = match slot.as_ref() {
            Some(s) => s.path != path || s.generation != current_gen,
            None => true,
        };
        if reload {
            let mut engine = QTensorEngine::new();
            load(&mut engine);
            *slot = Some(StickyEngine {
                path: path.to_string(),
                generation: current_gen,
                engine,
            });
        }
        f(&mut slot.as_mut().expect("sticky engine just loaded").engine)
    })
}

/// Shared bounded intake manager for sticky infer execution (Work Package F4).
pub struct StickyAdmissionDriver {
    queue: Mutex<BoundedIntakeQueue<128>>,
}

impl Default for StickyAdmissionDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl StickyAdmissionDriver {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(BoundedIntakeQueue::new()),
        }
    }

    /// Global singleton instance of the sticky intake driver.
    pub fn global() -> &'static Self {
        static DRIVER: OnceLock<StickyAdmissionDriver> = OnceLock::new();
        DRIVER.get_or_init(Self::new)
    }

    /// Enqueue a request descriptor into the bounded intake queue.
    pub fn enqueue(&self, request: IntakeRequest) -> Result<(), IntakeError> {
        let mut q = self.queue.lock().unwrap();
        q.enqueue(request)
    }

    /// Pop the next runnable request descriptor.
    pub fn pop_next(&self) -> Option<IntakeRequest> {
        let mut q = self.queue.lock().unwrap();
        q.pop_next_runnable()
    }

    /// Cancel a request in the queue.
    pub fn cancel(&self, request_id: u64) -> Result<IntakeState, IntakeError> {
        let mut q = self.queue.lock().unwrap();
        q.cancel(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_residency_invalidation_lifecycle() {
        let gen1 = current_residency_generation();
        let gen2 = invalidate_sticky_residency();
        assert_eq!(gen2, gen1 + 1);
        assert_eq!(current_residency_generation(), gen2);
    }

    #[test]
    fn test_sticky_admission_driver_intake_and_cancel() {
        let driver = StickyAdmissionDriver::new();
        let req1 = IntakeRequest::new(101, 50, 20);
        let req2 = IntakeRequest::new(102, 100, 40);

        driver.enqueue(req1).unwrap();
        driver.enqueue(req2).unwrap();

        driver.cancel(101).unwrap();

        // req1 was cancelled, so pop_next returns req2
        let next = driver.pop_next().unwrap();
        assert_eq!(next.request_id, 102);

        // No more runnable requests
        assert!(driver.pop_next().is_none());

        // Global singleton is accessible
        assert!(StickyAdmissionDriver::global().pop_next().is_none());
    }
}
