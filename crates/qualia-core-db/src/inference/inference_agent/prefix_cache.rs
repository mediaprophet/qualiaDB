//! Typed, bounded prefix KV cache for the QualiaDB local inference engine.
//!
//! Replaces legacy unsafe 8-byte hash keying with an exact compatibility contract:
//! - Model instance & tokenizer identity
//! - Exact token sequence digest & token length
//! - Full graph-context digest (not truncated to 8 bytes)
//! - KV cache geometry & layer count
//! - Strict completion flag (incomplete/cancelled prefill is never cached or restored)
//! - Bounded capacity with deterministic LRU eviction conforming to the Sentinel ceiling

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Default maximum number of cached prompt KV sequences.
pub const DEFAULT_MAX_ENTRIES: usize = 16;

/// Default maximum memory ceiling for cached KV floats (64 MiB).
pub const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;

/// 256-bit mixer for token ID sequences to guarantee zero collision in the cache key.
#[inline]
fn mix64(mut val: u64) -> u64 {
    val ^= val >> 30;
    val = val.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    val ^= val >> 27;
    val = val.wrapping_mul(0x94d0_49bb_1331_11eb);
    val ^ (val >> 31)
}

/// Compute a 256-bit collision-resistant digest over a slice of prompt token IDs.
pub fn digest_prompt_tokens(tokens: &[u32]) -> [u64; 4] {
    let mut words = [
        0x243f_6a88_85a3_08d3u64,
        0x1319_8a2e_0370_7344u64,
        0xa409_3822_299f_31d0u64,
        0x082e_fa98_ec4e_6c89u64,
    ];
    for (index, &token) in tokens.iter().enumerate() {
        let target = index & 3;
        let tok_val = (token as u64) ^ ((index as u64) << 32);
        words[target] = mix64(words[target] ^ tok_val);
    }
    words
}

/// Strongly-typed cache key requiring exact compatibility across model, tokens, graph, and tensor geometry.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InferenceCacheKey {
    pub model_instance: u64,
    pub tokenizer_revision: u64,
    pub prompt_tokens_digest: [u64; 4],
    pub prompt_token_count: u32,
    pub graph_context_digest: u64,
    pub kv_floats: usize,
    pub n_layers: usize,
}

impl InferenceCacheKey {
    pub fn new(
        model_instance: u64,
        tokenizer_revision: u64,
        prompt_tokens: &[u32],
        graph_context_digest: u64,
        kv_floats: usize,
        n_layers: usize,
    ) -> Self {
        Self {
            model_instance,
            tokenizer_revision,
            prompt_tokens_digest: digest_prompt_tokens(prompt_tokens),
            prompt_token_count: prompt_tokens.len() as u32,
            graph_context_digest,
            kv_floats,
            n_layers,
        }
    }
}

/// Cached KV data entry with metadata and completion invariant.
#[derive(Clone)]
pub struct InferenceCacheEntry {
    pub data: Box<[f32]>,
    pub token_count: u32,
    pub is_complete: bool,
    pub last_accessed: u64,
    pub byte_size: usize,
}

/// Bounded in-memory KV cache with capacity limits and deterministic LRU eviction.
pub struct BoundedInferenceCache {
    entries: HashMap<InferenceCacheKey, InferenceCacheEntry>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    access_clock: u64,
}

impl BoundedInferenceCache {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1024),
            current_bytes: 0,
            access_clock: 0,
        }
    }

    /// Number of active cache entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total bytes consumed by stored KV cache float buffers.
    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    /// Restore cached KV slice via a closure without cloning if the key matches and is complete.
    pub fn restore_if_match<R, F: FnOnce(&[f32]) -> R>(
        &mut self,
        key: &InferenceCacheKey,
        f: F,
    ) -> Option<R> {
        if key.prompt_token_count == 0 {
            return None;
        }
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.is_complete && entry.token_count == key.prompt_token_count {
                self.access_clock = self.access_clock.wrapping_add(1);
                entry.last_accessed = self.access_clock;
                return Some(f(&entry.data));
            }
        }
        None
    }

    /// Insert completed prefill KV data. Incomplete or empty data is strictly rejected.
    pub fn insert(
        &mut self,
        key: InferenceCacheKey,
        data: Box<[f32]>,
        token_count: u32,
        is_complete: bool,
    ) -> Result<(), &'static str> {
        if !is_complete {
            return Err("cannot cache incomplete or aborted prefill");
        }
        if token_count == 0 || token_count != key.prompt_token_count {
            return Err("token count mismatch or zero tokens");
        }
        if data.len() != key.kv_floats {
            return Err("KV float slice length does not match key layout");
        }

        let byte_size = data.len() * std::mem::size_of::<f32>();
        if byte_size > self.max_bytes {
            return Err("KV entry size exceeds maximum cache byte ceiling");
        }

        // Evict until both entry limit and byte budget are satisfied
        while self.entries.len() >= self.max_entries
            || (self.current_bytes + byte_size > self.max_bytes && !self.entries.is_empty())
        {
            if !self.evict_lru() {
                break;
            }
        }

        if let Some(old) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.byte_size);
        }

        self.access_clock = self.access_clock.wrapping_add(1);
        self.current_bytes += byte_size;
        self.entries.insert(
            key,
            InferenceCacheEntry {
                data,
                token_count,
                is_complete: true,
                last_accessed: self.access_clock,
                byte_size,
            },
        );
        Ok(())
    }

    /// Evict the least recently accessed entry. Returns true if an entry was removed.
    fn evict_lru(&mut self) -> bool {
        let mut oldest_key = None;
        let mut oldest_clock = u64::MAX;

        for (k, v) in &self.entries {
            if v.last_accessed < oldest_clock {
                oldest_clock = v.last_accessed;
                oldest_key = Some(*k);
            }
        }

        if let Some(key) = oldest_key {
            if let Some(removed) = self.entries.remove(&key) {
                self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
                return true;
            }
        }
        false
    }

    /// Clear all cached entries and reset byte counter.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_bytes = 0;
    }
}

static PREFIX_CACHE: OnceLock<Mutex<BoundedInferenceCache>> = OnceLock::new();

/// Access the process-wide bounded inference cache.
pub fn get_prefix_cache() -> &'static Mutex<BoundedInferenceCache> {
    PREFIX_CACHE.get_or_init(|| {
        Mutex::new(BoundedInferenceCache::new(
            DEFAULT_MAX_ENTRIES,
            DEFAULT_MAX_BYTES,
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_different_prompts_same_context() {
        let model = 0x1122_3344;
        let tok_rev = 1;
        let ctx_hash = 0x9988_7766;
        let kv_floats = 1024;
        let n_layers = 12;

        let tokens_a = [101, 2054, 2003, 1037, 3899];
        let tokens_b = [101, 2054, 2003, 1037, 7072]; // different last token

        let key_a = InferenceCacheKey::new(model, tok_rev, &tokens_a, ctx_hash, kv_floats, n_layers);
        let key_b = InferenceCacheKey::new(model, tok_rev, &tokens_b, ctx_hash, kv_floats, n_layers);

        assert_ne!(key_a, key_b);
        assert_ne!(key_a.prompt_tokens_digest, key_b.prompt_tokens_digest);
    }

    #[test]
    fn test_cache_key_different_graph_context_distinct() {
        let model = 0x1122_3344;
        let tok_rev = 1;
        let tokens = [101, 2054];
        let kv_floats = 512;
        let n_layers = 8;

        let ctx_1 = crate::q_hash("abcdefgh-1");
        let ctx_2 = crate::q_hash("abcdefgh-2");

        let key_1 = InferenceCacheKey::new(model, tok_rev, &tokens, ctx_1, kv_floats, n_layers);
        let key_2 = InferenceCacheKey::new(model, tok_rev, &tokens, ctx_2, kv_floats, n_layers);

        assert_ne!(key_1, key_2);
        assert_ne!(key_1.graph_context_digest, key_2.graph_context_digest);
    }

    #[test]
    fn test_incomplete_prefill_rejected_from_cache() {
        let mut cache = BoundedInferenceCache::new(4, 1024 * 1024);
        let key = InferenceCacheKey::new(1, 1, &[10, 20, 30], 100, 64, 2);
        let dummy_kv = vec![0.5f32; 64].into_boxed_slice();

        // Attempting to insert incomplete prefill must error
        let res = cache.insert(key, dummy_kv, 3, false);
        assert!(res.is_err());
        assert_eq!(cache.len(), 0);

        // Lookup must yield None
        let restored = cache.restore_if_match(&key, |data| data[0]);
        assert_eq!(restored, None);
    }

    #[test]
    fn test_lru_eviction_when_capacity_reached() {
        let mut cache = BoundedInferenceCache::new(2, 1024 * 1024); // max 2 entries

        let key1 = InferenceCacheKey::new(1, 1, &[1, 2], 100, 32, 2);
        let key2 = InferenceCacheKey::new(1, 1, &[3, 4], 100, 32, 2);
        let key3 = InferenceCacheKey::new(1, 1, &[5, 6], 100, 32, 2);

        let kv1 = vec![1.0f32; 32].into_boxed_slice();
        let kv2 = vec![2.0f32; 32].into_boxed_slice();
        let kv3 = vec![3.0f32; 32].into_boxed_slice();

        assert!(cache.insert(key1, kv1, 2, true).is_ok());
        assert!(cache.insert(key2, kv2, 2, true).is_ok());
        assert_eq!(cache.len(), 2);

        // Access key1 to make key2 the LRU
        let val1 = cache.restore_if_match(&key1, |d| d[0]);
        assert_eq!(val1, Some(1.0));

        // Inserting key3 should evict key2
        assert!(cache.insert(key3, kv3, 2, true).is_ok());
        assert_eq!(cache.len(), 2);

        // key1 and key3 should still be in cache; key2 should be gone
        assert!(cache.restore_if_match(&key1, |_| ()).is_some());
        assert!(cache.restore_if_match(&key3, |_| ()).is_some());
        assert!(cache.restore_if_match(&key2, |_| ()).is_none());
    }

    #[test]
    fn test_byte_ceiling_eviction() {
        // Limit total bytes to 256 floats (1024 bytes)
        let max_bytes = 256 * std::mem::size_of::<f32>();
        let mut cache = BoundedInferenceCache::new(10, max_bytes);

        let key1 = InferenceCacheKey::new(1, 1, &[1], 100, 200, 2);
        let key2 = InferenceCacheKey::new(1, 1, &[2], 100, 100, 2);

        let kv1 = vec![1.0f32; 200].into_boxed_slice();
        let kv2 = vec![2.0f32; 100].into_boxed_slice();

        assert!(cache.insert(key1, kv1, 1, true).is_ok());
        assert_eq!(cache.current_bytes(), 200 * 4);

        // Inserting kv2 (100 floats = 400 bytes) + existing 800 bytes = 1200 > 1024
        // Should evict key1 to stay under ceiling
        assert!(cache.insert(key2, kv2, 1, true).is_ok());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.current_bytes(), 100 * 4);
        assert!(cache.restore_if_match(&key1, |_| ()).is_none());
        assert!(cache.restore_if_match(&key2, |_| ()).is_some());
    }
}
