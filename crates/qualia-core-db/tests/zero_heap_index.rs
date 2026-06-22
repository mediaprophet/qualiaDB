//! Empirical zero-heap proof for the `QuinIndex` resolution hot path (task #22).
//!
//! The legacy `by_*` methods return `Vec<NQuin>` — one heap allocation per call (the
//! "allocation trap"). The new `iter_*` / `object_of` / `rows_by_subject` accessors
//! must allocate ZERO on lookup, so modal-kind resolution can run continuously on the
//! hot path without violating the zero-heap invariant.
//!
//! Measured with a counting global allocator. Single test per binary on purpose: a
//! global counter is polluted by tests running in parallel within one binary.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use qualia_core_db::indexing::QuinIndex;
use qualia_core_db::NQuin;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}

#[global_allocator]
static GA: Counting = Counting;

fn q(s: u64, p: u64, o: u64) -> NQuin {
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

#[test]
fn quin_index_resolution_is_zero_heap() {
    // ── Build phase (allocations expected; OUTSIDE the measured window). ──────────
    let mut quins = Vec::new();
    for i in 0..64u64 {
        quins.push(q(i, 100 + (i % 4), 900 + i));
    }
    // The "obligation" we resolve: subject 7, predicate 101 ("has-modality-kind"),
    // object 12345 (the kind).
    quins.push(q(7, 101, 12345));
    let idx = QuinIndex::from_slice(&quins);

    // Warm up any one-time lazy init before measuring.
    let _ = idx.object_of(7, 101);

    // ── Measured window: continuous resolution via the zero-alloc accessors. ──────
    let before = ALLOCS.load(Ordering::SeqCst);
    let mut sink = 0u64;
    for _ in 0..1000 {
        if let Some(o) = idx.object_of(7, 101) {
            sink ^= o;
        }
        sink = sink.wrapping_add(idx.iter_by_subject_and_predicate(7, 101).count() as u64);
        for &row in idx.rows_by_subject(3) {
            sink ^= idx.quin_at(row).object;
        }
    }
    let during = ALLOCS.load(Ordering::SeqCst).wrapping_sub(before);
    std::hint::black_box(sink);

    assert_eq!(
        during, 0,
        "QuinIndex zero-alloc accessors allocated {during} times across 1000 resolutions"
    );

    // ── Correctness: the resolution is real, not vacuous. ────────────────────────
    assert_eq!(
        idx.object_of(7, 101),
        Some(12345),
        "object_of must resolve the modal kind"
    );
    assert_eq!(idx.iter_by_subject_and_predicate(7, 101).count(), 1);
    assert!(idx.object_of(7, 999).is_none(), "absent predicate resolves to None");

    // ── Contrast: the legacy Vec-returning method DOES allocate (the trap we replaced). ──
    let before_vec = ALLOCS.load(Ordering::SeqCst);
    let v = idx.by_subject_and_predicate(7, 101);
    let after_vec = ALLOCS.load(Ordering::SeqCst).wrapping_sub(before_vec);
    std::hint::black_box(&v);
    assert!(
        after_vec >= 1,
        "legacy by_subject_and_predicate is expected to allocate a Vec; got {after_vec}"
    );
}
