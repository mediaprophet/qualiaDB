//! DXC SPIR-V binary cache — avoids re-running DXC on identical HLSL source.
//!
//! The cache is keyed by `blake3(hlsl_source + entry_point)` and stores the
//! binary SPIR-V blob in a thread-local `HashMap`. On a cache hit, the DXC
//! subprocess is skipped entirely (saves ~50-200ms per kernel).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::wgsl_forge::ForgeError;

/// Cache key: blake3 hash of (hlsl_source, entry_point).
fn cache_key(hlsl_source: &str, entry_point: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(hlsl_source.as_bytes());
    hasher.update(entry_point.as_bytes());
    hasher.finalize().to_hex().to_string()
}

struct DxcCache {
    entries: HashMap<String, Vec<u8>>,
}

impl DxcCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

fn dxc_cache() -> &'static Mutex<DxcCache> {
    static C: OnceLock<Mutex<DxcCache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(DxcCache::new()))
}

/// Compile HLSL to SPIR-V with caching. On a cache hit, returns the stored
/// binary without invoking DXC. On a miss, calls the inner compiler and stores
/// the result.
pub fn compile_hlsl_to_spirv_cached(
    hlsl_source: &str,
    entry_point: &str,
    inner: impl Fn(&str, &str) -> Result<Vec<u8>, ForgeError>,
) -> Result<Vec<u8>, ForgeError> {
    let key = cache_key(hlsl_source, entry_point);

    // Check cache.
    if let Ok(guard) = dxc_cache().lock() {
        if let Some(blob) = guard.entries.get(&key) {
            return Ok(blob.clone());
        }
    }

    // Cache miss — compile.
    let spirv = inner(hlsl_source, entry_point)?;

    // Store in cache.
    if let Ok(mut guard) = dxc_cache().lock() {
        guard.entries.insert(key, spirv.clone());
    }

    Ok(spirv)
}

/// Clear the DXC SPIR-V cache (useful for testing or forced recompilation).
pub fn clear_dxc_cache() {
    if let Ok(mut guard) = dxc_cache().lock() {
        guard.entries.clear();
    }
}

/// Number of cached SPIR-V binaries.
pub fn dxc_cache_len() -> usize {
    dxc_cache().lock().map(|g| g.entries.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn cache_key_is_deterministic() {
        let a = cache_key("shader1", "main");
        let b = cache_key("shader1", "main");
        assert_eq!(a, b);
        let c = cache_key("shader2", "main");
        assert_ne!(a, c);
        let d = cache_key("shader1", "other");
        assert_ne!(a, d);
    }

    #[test]
    fn cache_hit_skips_inner() {
        clear_dxc_cache();
        let calls = Cell::new(0u32);
        let inner = |_: &str, _: &str| -> Result<Vec<u8>, ForgeError> {
            calls.set(calls.get() + 1);
            Ok(vec![0x07, 0x23, 0x02, 0x03])
        };
        let r1 = compile_hlsl_to_spirv_cached("src", "entry", inner).unwrap();
        let r2 = compile_hlsl_to_spirv_cached("src", "entry", inner).unwrap();
        assert_eq!(r1, r2);
        assert_eq!(
            calls.get(),
            1,
            "inner should only be called once on cache hit"
        );
    }

    #[test]
    fn cache_miss_on_different_source() {
        clear_dxc_cache();
        let calls = Cell::new(0u32);
        let inner = |_: &str, _: &str| -> Result<Vec<u8>, ForgeError> {
            calls.set(calls.get() + 1);
            Ok(vec![1, 2, 3])
        };
        let _ = compile_hlsl_to_spirv_cached("src_a", "entry", inner).unwrap();
        let _ = compile_hlsl_to_spirv_cached("src_b", "entry", inner).unwrap();
        assert_eq!(calls.get(), 2, "different source should be a cache miss");
    }
}
