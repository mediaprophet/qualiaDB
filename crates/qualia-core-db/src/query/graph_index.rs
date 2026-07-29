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
use std::sync::RwLock;

/// Memoizes a value `T` by a monotonically-advancing `revision`, rebuilding via `build`
/// only when the supplied revision differs from the cached one.
pub struct RevisionCache<T> {
    inner: RwLock<Option<(u64, T)>>,
}

impl<T> RevisionCache<T> {
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// Run `f` against the value cached at `revision`, building it via `build` first if
    /// the cache is empty or stale. `build` runs at most once per distinct revision, and
    /// only when a caller actually needs the value (lazy).
    pub fn with<R>(&self, revision: u64, build: impl FnOnce() -> T, f: impl FnOnce(&T) -> R) -> R {
        // Fast path: cache present and fresh.
        {
            let guard = self.inner.read().unwrap();
            if let Some((rev, value)) = guard.as_ref() {
                if *rev == revision {
                    return f(value);
                }
            }
        }
        // Slow path: (re)build and store, then serve.
        let value = build();
        {
            let mut w = self.inner.write().unwrap();
            *w = Some((revision, value));
        }
        let guard = self.inner.read().unwrap();
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

    #[test]
    fn rebuilds_only_on_revision_change() {
        let cache: RevisionCache<usize> = RevisionCache::new();
        let builds = AtomicUsize::new(0);

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
}
