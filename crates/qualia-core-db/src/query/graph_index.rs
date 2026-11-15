//! Revision-cached graph index — the per-cell index lifecycle (#22 step 3).
//!
//! Building a `QuinIndex` over the daemon-graph snapshot on every resolve would be O(n)
//! per call. Instead we memoize it by `daemon_graph::graph_revision()`: the index is
//! rebuilt LAZILY only when the graph has actually changed, so resolution is O(1)
//! amortized. The cache is host-side (separate from the 42 MB SlgArena).
//!
//! "Per-cell" in the Fractal-Shard model means one cache per 512 MB cell; for the single
//! daemon graph today this is that one cache. Streaming a huge graph via BIDX/demand-
//! paging — so the index need not copy the whole snapshot — remains future work.

#[cfg(not(target_arch = "wasm32"))]
use crate::indexing::QuinIndex;
use std::sync::{Mutex, RwLock};

/// Memoizes a value `T` by a monotonically-advancing `revision`, rebuilding via `build`
/// only when the supplied revision exceeds the cached one.
///
/// Uses a dedicated single-flight `build_lock` alongside the `RwLock` storage so:
/// 1. Concurrent callers requesting a newer revision coalesce into a single `build()`
///    invocation instead of triggering an $N$-thread rebuild stampede.
/// 2. Existing readers can continue reading the previous snapshot while the new revision
///    is being built, blocking only for the $O(1)$ pointer swap.
/// 3. Monotonic revision checks prevent a stale revision from overwriting a newer one.
pub struct RevisionCache<T> {
    inner: RwLock<Option<(u64, T)>>,
    build_lock: Mutex<()>,
}

impl<T> RevisionCache<T> {
    /// Creates a default `RevisionCache`.
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(None),
            build_lock: Mutex::new(()),
        }
    }

    /// Returns the currently cached revision, or `None` if the cache is empty.
    pub fn current_revision(&self) -> Option<u64> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().map(|(rev, _)| *rev)
    }

    /// Clears the cached value so the next [`Self::with`] call rebuilds it.
    pub fn invalidate(&self) {
        let _build_guard = self.build_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut w = self.inner.write().unwrap_or_else(|e| e.into_inner());
        *w = None;
    }

    /// Run `f` against the value cached at `revision` (or newer), building it via `build`
    /// first if the cache is empty or at a lower revision. `build` runs at most once per
    /// distinct monotonic revision advance, and only when a caller actually needs the value.
    pub fn with<R>(&self, revision: u64, build: impl FnOnce() -> T, f: impl FnOnce(&T) -> R) -> R {
        // Fast path: cache present and at or ahead of `revision`.
        {
            let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
            if let Some((rev, value)) = guard.as_ref() {
                if *rev >= revision {
                    return f(value);
                }
            }
        }
        // Slow path: serialize rebuilds with `build_lock` and double-check `inner`.
        {
            let _build_guard = self.build_lock.lock().unwrap_or_else(|e| e.into_inner());
            let needs_build = {
                let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
                guard.as_ref().map_or(true, |(rev, _)| *rev < revision)
            };
            if needs_build {
                let value = build();
                let mut w = self.inner.write().unwrap_or_else(|e| e.into_inner());
                if w.as_ref().map_or(true, |(cur_rev, _)| revision >= *cur_rev) {
                    *w = Some((revision, value));
                }
            }
        }
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let (_, value) = guard.as_ref().expect("cache just populated");
        f(value)
    }
}

impl<T> Default for RevisionCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
static GRAPH_INDEX: RevisionCache<QuinIndex> = RevisionCache::new();

/// Run `f` against a `QuinIndex` over the current daemon graph, rebuilt only when the
/// graph revision has changed since the last build. This is what `graph_resolve` (and
/// other index consumers) route through, so a burst of resolves between graph changes
/// shares a single O(n) build.
// Routes through `daemon_graph` (the native daemon's in-memory graph), which does not exist on
// wasm32; the only caller (mcp_tool_impls) is native-only too.
#[cfg(not(target_arch = "wasm32"))]
/// Builder: `graph-index`.
pub fn with_graph_index<R>(f: impl FnOnce(&QuinIndex) -> R) -> R {
    let revision = crate::daemon_graph::graph_revision();
    GRAPH_INDEX.with(
        revision,
        || {
            let guard = crate::daemon_graph::graph_read_guard();
            QuinIndex::from_slice(guard.as_slice())
        },
        f,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    #[test]
    fn rebuilds_only_on_revision_change() {
        let cache: RevisionCache<usize> = RevisionCache::new();
        let builds = AtomicUsize::new(0);
        assert_eq!(cache.current_revision(), None);

        // First touch at rev 1 builds.
        let v = cache.with(
            1,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                42
            },
            |v| *v,
        );
        assert_eq!(v, 42);
        assert_eq!(cache.current_revision(), Some(1));
        // Same rev: served from cache, no rebuild.
        cache.with(
            1,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                0
            },
            |_| (),
        );
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "same revision must not rebuild"
        );

        // New rev: rebuild.
        cache.with(
            2,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                99
            },
            |_| (),
        );
        assert_eq!(
            builds.load(Ordering::SeqCst),
            2,
            "changed revision must rebuild"
        );
        assert_eq!(cache.current_revision(), Some(2));
        // And stays cached at the new rev.
        cache.with(
            2,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                0
            },
            |_| (),
        );
        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn monotonic_revision_prevents_stale_overwrite() {
        let cache: RevisionCache<usize> = RevisionCache::new();
        let builds = AtomicUsize::new(0);

        let v2 = cache.with(
            2,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                200
            },
            |v| *v,
        );
        assert_eq!(v2, 200);
        assert_eq!(cache.current_revision(), Some(2));

        // Requesting a stale revision (1 < 2) must neither rebuild nor overwrite revision 2.
        let v_stale = cache.with(
            1,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                100
            },
            |v| *v,
        );
        assert_eq!(v_stale, 200);
        assert_eq!(builds.load(Ordering::SeqCst), 1);
        assert_eq!(cache.current_revision(), Some(2));

        // Explicit invalidation resets the cache so revision 1 can be built fresh if needed.
        cache.invalidate();
        assert_eq!(cache.current_revision(), None);
        let v1 = cache.with(
            1,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                100
            },
            |v| *v,
        );
        assert_eq!(v1, 100);
        assert_eq!(cache.current_revision(), Some(1));
    }

    #[test]
    fn single_flight_deduplicates_concurrent_rebuilds() {
        let cache = Arc::new(RevisionCache::<usize>::new());
        let builds = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(8));

        let mut handles = Vec::with_capacity(8);
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let builds = Arc::clone(&builds);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                cache.with(
                    5,
                    || {
                        builds.fetch_add(1, Ordering::SeqCst);
                        555
                    },
                    |v| *v,
                )
            }));
        }

        for h in handles {
            assert_eq!(h.join().unwrap(), 555);
        }
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "8 concurrent threads requesting revision 5 must invoke build() exactly once"
        );
        assert_eq!(cache.current_revision(), Some(5));
    }

    #[test]
    fn reentrant_with_inside_callback_does_not_deadlock() {
        let cache: RevisionCache<usize> = RevisionCache::new();
        let outer = cache.with(
            3,
            || 30,
            |v| {
                let inner = cache.with(3, || 999, |inner_v| *inner_v + 7);
                *v + inner
            },
        );
        assert_eq!(outer, 67);
    }
}
