// ─── Sticky 1-thread infer pool (native) ────────────────────────────────────
// Every `infer` previously did `thread::spawn` + `QTensorEngine::new()` which
// rebuilds wgpu pipelines (~seconds) even when the mmap is already resident.
// A size-1 rayon pool keeps a dedicated OS thread whose `thread_local` engine
// survives across jobs; same-path multi-turn / multi-prompt reuses the engine.
use crate::gguf_bridge::QTensorEngine;
use std::cell::RefCell;
use std::sync::OnceLock;

pub struct StickyEngine {
    pub path: String,
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

/// Borrow-or-reload the sticky engine for `path`, then run `f`.
pub fn with_engine<R>(
    path: &str,
    mut load: impl FnMut(&mut QTensorEngine),
    f: impl FnOnce(&mut QTensorEngine) -> R,
) -> R {
    ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let reload = match slot.as_ref() {
            Some(s) => s.path != path,
            None => true,
        };
        if reload {
            let mut engine = QTensorEngine::new();
            load(&mut engine);
            *slot = Some(StickyEngine {
                path: path.to_string(),
                engine,
            });
        }
        f(&mut slot.as_mut().expect("sticky engine just loaded").engine)
    })
}
