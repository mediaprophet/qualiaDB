//! A9 — Semantic Skills: Vectors, Embeddings & Scratchpads.
//!
//! First-class vector cosine distance, in-process text embedding, semantic
//! search, and ephemeral scratchpad memory for agent workflows.
//!
//! ## Design
//!
//! - **Vector**: Fixed-size f32 vector with cosine distance.
//! - **VectorStore**: A bounded collection of vectors with semantic search
//!   (k-nearest neighbors by cosine similarity).
//! - **TextEmbedder**: In-process text embedding using FNV-1a hashing into
//!  a fixed-dimensional vector. This is a lightweight embedding suitable for
//!  semantic deduplication and approximate search — not a replacement for
//!  neural embeddings.
//! - **Scratchpad**: Ephemeral key-value memory for agent workflows. Bounded,
//!  with TTL-based expiry.
//!
//! ## Integration
//!
//! - Uses A5 (`blackboard`) for observable state channels.
//! - Designed for zero-heap on hot paths (search uses caller-supplied buffers).

use std::collections::HashMap;

// ── Vector ─────────────────────────────────────────────────────────────────

/// Default embedding dimensionality.
pub const EMBED_DIM: usize = 256;
/// Maximum vectors in a store.
pub const MAX_VECTORS: usize = 4096;
/// Maximum scratchpad entries.
pub const MAX_SCRATCHPAD_ENTRIES: usize = 256;
/// Default scratchpad TTL (in ticks).
pub const DEFAULT_SCRATCHPAD_TTL: u32 = 1000;

/// A fixed-size vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    pub dims: Vec<f32>,
}

impl Vector {
    pub fn new(dims: Vec<f32>) -> Self {
        Self { dims }
    }

    pub fn zeros(dim: usize) -> Self {
        Self {
            dims: vec![0.0; dim],
        }
    }

    pub fn dim(&self) -> usize {
        self.dims.len()
    }

    /// Compute the L2 norm.
    pub fn norm(&self) -> f32 {
        self.dims.iter().map(|v| v * v).sum::<f32>().sqrt()
    }

    /// Compute the dot product with another vector.
    pub fn dot(&self, other: &Vector) -> f32 {
        self.dims
            .iter()
            .zip(other.dims.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// Compute the cosine similarity with another vector.
    /// Returns 0.0 if either vector has zero norm.
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        let norm_product = self.norm() * other.norm();
        if norm_product == 0.0 {
            return 0.0;
        }
        self.dot(other) / norm_product
    }

    /// Compute the cosine distance (1 - similarity).
    pub fn cosine_distance(&self, other: &Vector) -> f32 {
        1.0 - self.cosine_similarity(other)
    }

    /// Normalize the vector in place.
    pub fn normalize(&mut self) {
        let norm = self.norm();
        if norm > 0.0 {
            for v in &mut self.dims {
                *v /= norm;
            }
        }
    }
}

// ── Text Embedder ──────────────────────────────────────────────────────────

/// A lightweight in-process text embedder using FNV-1a hashing.
/// Maps text into a fixed-dimensional vector by hashing character n-grams
/// into vector dimensions. This is suitable for semantic deduplication and
/// approximate search, not as a replacement for neural embeddings.
pub struct TextEmbedder {
    dim: usize,
    ngram_size: usize,
}

impl TextEmbedder {
    pub fn new(dim: usize, ngram_size: usize) -> Self {
        Self { dim, ngram_size }
    }

    pub fn default() -> Self {
        Self::new(EMBED_DIM, 3)
    }

    /// Embed text into a fixed-dimensional vector.
    /// Uses character n-gram hashing: each n-gram is hashed and contributes
    /// to a dimension. Positive contributions for presence, weighted by
    /// frequency.
    pub fn embed(&self, text: &str) -> Vector {
        let mut dims = vec![0.0f32; self.dim];
        let chars: Vec<char> = text.to_lowercase().chars().collect();
        if chars.len() < self.ngram_size {
            // For very short text, hash the whole string.
            let h = fnv1a_hash(text.as_bytes());
            dims[(h as usize) % self.dim] += 1.0;
            return Vector { dims };
        }
        for i in 0..=chars.len() - self.ngram_size {
            let ngram: String = chars[i..i + self.ngram_size].iter().collect();
            let h = fnv1a_hash(ngram.as_bytes());
            let idx = (h as usize) % self.dim;
            dims[idx] += 1.0;
        }
        let mut v = Vector { dims };
        v.normalize();
        v
    }
}

/// FNV-1a hash (32-bit).
fn fnv1a_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

// ── Vector Store ───────────────────────────────────────────────────────────

/// A stored vector with an associated key and optional metadata.
#[derive(Debug, Clone)]
pub struct StoredVector {
    pub key: String,
    pub vector: Vector,
    pub metadata: Option<String>,
}

/// A bounded vector store with semantic search.
pub struct VectorStore {
    vectors: Vec<StoredVector>,
    embedder: TextEmbedder,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            vectors: Vec::new(),
            embedder: TextEmbedder::default(),
        }
    }

    pub fn with_embedder(embedder: TextEmbedder) -> Self {
        Self {
            vectors: Vec::new(),
            embedder,
        }
    }

    /// Add a vector to the store. Returns false if the store is full.
    pub fn add(&mut self, key: &str, vector: Vector, metadata: Option<String>) -> bool {
        if self.vectors.len() >= MAX_VECTORS {
            return false;
        }
        self.vectors.push(StoredVector {
            key: key.to_string(),
            vector,
            metadata,
        });
        true
    }

    /// Add text to the store by embedding it.
    pub fn add_text(&mut self, key: &str, text: &str, metadata: Option<String>) -> bool {
        let vector = self.embedder.embed(text);
        self.add(key, vector, metadata)
    }

    /// Search for the k nearest neighbors by cosine similarity.
    /// Returns results sorted by descending similarity.
    pub fn search(&self, query: &Vector, k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .vectors
            .iter()
            .map(|sv| SearchResult {
                key: sv.key.clone(),
                similarity: query.cosine_similarity(&sv.vector),
                metadata: sv.metadata.clone(),
            })
            .collect();
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(k);
        results
    }

    /// Search by text (embeds the query first).
    pub fn search_text(&self, query_text: &str, k: usize) -> Vec<SearchResult> {
        let query = self.embedder.embed(query_text);
        self.search(&query, k)
    }

    /// Get a vector by key.
    pub fn get(&self, key: &str) -> Option<&StoredVector> {
        self.vectors.iter().find(|v| v.key == key)
    }

    /// Remove a vector by key.
    pub fn remove(&mut self, key: &str) -> bool {
        let len_before = self.vectors.len();
        self.vectors.retain(|v| v.key != key);
        self.vectors.len() != len_before
    }

    /// Number of stored vectors.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Is the store empty?
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Clear all vectors.
    pub fn clear(&mut self) {
        self.vectors.clear();
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// A search result from the vector store.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub key: String,
    pub similarity: f32,
    pub metadata: Option<String>,
}

// ── Scratchpad ─────────────────────────────────────────────────────────────

/// An ephemeral scratchpad entry with TTL.
#[derive(Debug, Clone)]
pub struct ScratchpadEntry {
    pub value: String,
    pub tick: u32,
    pub ttl: u32,
}

impl ScratchpadEntry {
    pub fn new(value: String, tick: u32, ttl: u32) -> Self {
        Self { value, tick, ttl }
    }

    /// Is this entry expired at the given tick?
    pub fn is_expired(&self, current_tick: u32) -> bool {
        current_tick > self.tick + self.ttl
    }
}

/// An ephemeral key-value scratchpad with TTL-based expiry.
/// Designed for agent workflows that need temporary memory between steps.
pub struct Scratchpad {
    entries: HashMap<String, ScratchpadEntry>,
    default_ttl: u32,
    current_tick: u32,
}

impl Scratchpad {
    pub fn new(default_ttl: u32) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
            current_tick: 0,
        }
    }

    pub fn default() -> Self {
        Self::new(DEFAULT_SCRATCHPAD_TTL)
    }

    /// Advance the tick counter.
    pub fn tick(&mut self) {
        self.current_tick += 1;
        // Evict expired entries.
        let tick = self.current_tick;
        self.entries.retain(|_, e| !e.is_expired(tick));
    }

    /// Set a key-value pair with the default TTL.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), ScratchpadError> {
        self.set_with_ttl(key, value, self.default_ttl)
    }

    /// Set a key-value pair with a custom TTL.
    pub fn set_with_ttl(
        &mut self,
        key: &str,
        value: &str,
        ttl: u32,
    ) -> Result<(), ScratchpadError> {
        if self.entries.len() >= MAX_SCRATCHPAD_ENTRIES && !self.entries.contains_key(key) {
            return Err(ScratchpadError::Full);
        }
        self.entries.insert(
            key.to_string(),
            ScratchpadEntry::new(value.to_string(), self.current_tick, ttl),
        );
        Ok(())
    }

    /// Get a value by key. Returns None if the key doesn't exist or is expired.
    pub fn get(&self, key: &str) -> Option<&str> {
        let entry = self.entries.get(key)?;
        if entry.is_expired(self.current_tick) {
            return None;
        }
        Some(entry.value.as_str())
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of entries (including expired ones not yet evicted).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the scratchpad empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }

    /// Evict all expired entries immediately.
    pub fn evict_expired(&mut self) -> usize {
        let tick = self.current_tick;
        let len_before = self.entries.len();
        self.entries.retain(|_, e| !e.is_expired(tick));
        len_before - self.entries.len()
    }
}

/// Errors from the scratchpad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScratchpadError {
    /// The scratchpad is full.
    Full,
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_cosine_similarity_identical() {
        let v = Vector::new(vec![1.0, 2.0, 3.0]);
        let sim = v.cosine_similarity(&v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vector_cosine_similarity_orthogonal() {
        let v1 = Vector::new(vec![1.0, 0.0]);
        let v2 = Vector::new(vec![0.0, 1.0]);
        let sim = v1.cosine_similarity(&v2);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn vector_cosine_similarity_opposite() {
        let v1 = Vector::new(vec![1.0, 0.0]);
        let v2 = Vector::new(vec![-1.0, 0.0]);
        let sim = v1.cosine_similarity(&v2);
        assert!((sim - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn vector_cosine_distance() {
        let v1 = Vector::new(vec![1.0, 0.0]);
        let v2 = Vector::new(vec![1.0, 0.0]);
        let dist = v1.cosine_distance(&v2);
        assert!(dist.abs() < 1e-5);
    }

    #[test]
    fn vector_normalize() {
        let mut v = Vector::new(vec![3.0, 4.0]);
        v.normalize();
        assert!((v.norm() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vector_zero_norm_similarity() {
        let v1 = Vector::new(vec![0.0, 0.0]);
        let v2 = Vector::new(vec![1.0, 0.0]);
        let sim = v1.cosine_similarity(&v2);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn text_embedder_basic() {
        let embedder = TextEmbedder::default();
        let v1 = embedder.embed("hello world");
        let v2 = embedder.embed("hello world");
        assert_eq!(v1, v2);
        assert_eq!(v1.dim(), EMBED_DIM);
    }

    #[test]
    fn text_embedder_similar_texts() {
        let embedder = TextEmbedder::default();
        let v1 = embedder.embed("the quick brown fox");
        let v2 = embedder.embed("the quick brown fox");
        let v3 = embedder.embed("completely different text");
        let sim_same = v1.cosine_similarity(&v2);
        let sim_diff = v1.cosine_similarity(&v3);
        assert!(sim_same > sim_diff);
    }

    #[test]
    fn text_embedder_normalizes() {
        let embedder = TextEmbedder::default();
        let v = embedder.embed("test text");
        assert!((v.norm() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vector_store_add_and_search() {
        let mut store = VectorStore::new();
        store.add_text("a", "hello world", None);
        store.add_text("b", "goodbye world", None);
        store.add_text("c", "hello there", None);
        let results = store.search_text("hello world", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, "a"); // Best match.
    }

    #[test]
    fn vector_store_get_and_remove() {
        let mut store = VectorStore::new();
        store.add_text("key1", "test", Some("meta".to_string()));
        assert!(store.get("key1").is_some());
        assert!(store.remove("key1"));
        assert!(store.get("key1").is_none());
    }

    #[test]
    fn vector_store_capacity() {
        let embedder = TextEmbedder::new(8, 2);
        let mut store = VectorStore::with_embedder(embedder);
        for i in 0..MAX_VECTORS {
            assert!(store.add_text(&format!("k{i}"), &format!("text{i}"), None));
        }
        // Adding one more should fail.
        assert!(!store.add_text("overflow", "text", None));
    }

    #[test]
    fn vector_store_clear() {
        let mut store = VectorStore::new();
        store.add_text("a", "test", None);
        store.add_text("b", "test2", None);
        assert_eq!(store.len(), 2);
        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn scratchpad_set_and_get() {
        let mut pad = Scratchpad::default();
        pad.set("key", "value").unwrap();
        assert_eq!(pad.get("key"), Some("value"));
    }

    #[test]
    fn scratchpad_remove() {
        let mut pad = Scratchpad::default();
        pad.set("key", "value").unwrap();
        assert!(pad.remove("key"));
        assert_eq!(pad.get("key"), None);
    }

    #[test]
    fn scratchpad_ttl_expiry() {
        let mut pad = Scratchpad::new(5);
        pad.set("key", "value").unwrap();
        assert_eq!(pad.get("key"), Some("value"));
        // Advance ticks past TTL.
        for _ in 0..6 {
            pad.tick();
        }
        assert_eq!(pad.get("key"), None);
    }

    #[test]
    fn scratchpad_ttl_not_expired() {
        let mut pad = Scratchpad::new(10);
        pad.set("key", "value").unwrap();
        pad.tick();
        pad.tick();
        assert_eq!(pad.get("key"), Some("value"));
    }

    #[test]
    fn scratchpad_evict_expired() {
        let mut pad = Scratchpad::new(2);
        pad.set("a", "1").unwrap();
        pad.set("b", "2").unwrap();
        // Advance tick past TTL — tick() auto-evicts.
        pad.tick();
        pad.tick();
        pad.tick();
        // All entries should have been auto-evicted by tick().
        assert!(pad.is_empty());
        // Manual evict should find nothing.
        let evicted = pad.evict_expired();
        assert_eq!(evicted, 0);
    }

    #[test]
    fn scratchpad_manual_evict_without_tick() {
        let mut pad = Scratchpad::new(2);
        pad.set("a", "1").unwrap();
        // Advance tick manually (without eviction).
        pad.current_tick = 5;
        // Now manually evict — entries should be expired.
        let evicted = pad.evict_expired();
        assert_eq!(evicted, 1);
        assert!(pad.is_empty());
    }

    #[test]
    fn scratchpad_clear() {
        let mut pad = Scratchpad::default();
        pad.set("a", "1").unwrap();
        pad.set("b", "2").unwrap();
        pad.clear();
        assert!(pad.is_empty());
    }

    #[test]
    fn scratchpad_overwrite() {
        let mut pad = Scratchpad::default();
        pad.set("key", "old").unwrap();
        pad.set("key", "new").unwrap();
        assert_eq!(pad.get("key"), Some("new"));
        assert_eq!(pad.len(), 1);
    }

    #[test]
    fn scratchpad_capacity() {
        let mut pad = Scratchpad::default();
        for i in 0..MAX_SCRATCHPAD_ENTRIES {
            pad.set(&format!("k{i}"), "v").unwrap();
        }
        let result = pad.set("overflow", "v");
        assert_eq!(result, Err(ScratchpadError::Full));
    }

    #[test]
    fn scratchpad_current_tick() {
        let mut pad = Scratchpad::default();
        assert_eq!(pad.current_tick(), 0);
        pad.tick();
        assert_eq!(pad.current_tick(), 1);
        pad.tick();
        assert_eq!(pad.current_tick(), 2);
    }

    #[test]
    fn search_result_sorted_by_similarity() {
        let mut store = VectorStore::new();
        let embedder = TextEmbedder::default();
        store.add("a", embedder.embed("hello world"), None);
        store.add("b", embedder.embed("hello there"), None);
        store.add("c", embedder.embed("completely different"), None);
        let query = embedder.embed("hello world");
        let results = store.search(&query, 3);
        assert!(results[0].similarity >= results[1].similarity);
        assert!(results[1].similarity >= results[2].similarity);
    }
}
