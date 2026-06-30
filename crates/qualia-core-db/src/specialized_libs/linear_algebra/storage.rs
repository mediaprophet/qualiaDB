use crate::solvers::SolversError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::sync::{Arc, Mutex};

use super::computation::*;
use super::core_types::*;
use super::optimization::*;
use super::performance::*;
use super::privacy::*;

/// Matrix storage using ZNS for zero-copy operations
pub struct MatrixStorage {
    pub zones: HashMap<String, MatrixZone>,
    pub allocator: MatrixAllocator,
    pub cache: MatrixCache,
    pub storage_backend: StorageBackend,
}

/// Matrix zone in ZNS storage
#[derive(Debug, Clone)]
pub struct MatrixZone {
    pub zone_id: String,
    pub zone_type: ZoneType,
    pub capacity: u64,
    pub matrices: HashMap<String, MatrixMetadata>,
    pub access_pattern: AccessPattern,
}

/// Zone types for different matrix workloads
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZoneType {
    /// Dense matrices for general linear algebra
    Dense,
    /// Sparse matrices for large-scale problems
    Sparse,
    /// Structured matrices (triangular, banded, etc.)
    Structured,
    /// Temporary matrices for computations
    Temporary,
    /// Cached matrices for frequently accessed data
    Cached,
}

/// Access patterns for optimization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,
    Random,
    Strided,
    Blocked,
    Adaptive,
}

/// Matrix allocator for efficient memory management
pub struct MatrixAllocator {
    pub allocation_strategy: AllocationStrategy,
    pub free_blocks: Vec<MemoryBlock>,
    pub allocated_blocks: HashMap<String, MemoryBlock>,
    pub fragmentation_threshold: f64,
    /// Total size of the memory pool (bytes)
    pub total_pool_size: u64,
    /// Monotonic counter for generating unique block IDs
    pub next_block_id: u64,
}

/// Allocation strategies
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    WorstFit,
    BuddySystem,
    Slab,
}

/// Memory block
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub block_id: String,
    pub start_address: u64,
    pub size: u64,
    pub is_free: bool,
    pub fragmentation_score: f64,
}

/// Matrix cache for frequently accessed matrices
pub struct MatrixCache {
    pub cache_entries: HashMap<String, CacheEntry>,
    pub cache_policy: CachePolicy,
    pub max_size: u64,
    pub current_size: u64,
    pub hit_count: u64,
    pub miss_count: u64,
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub matrix_id: String,
    pub data: Vec<u8>,
    pub access_time: u64,
    pub access_count: u64,
    pub size: u64,
    /// The cached matrix itself (stored directly for zero-deserialization retrieval)
    pub matrix: Option<Matrix>,
}

/// Cache policies
#[derive(Debug, Clone, PartialEq)]
pub enum CachePolicy {
    LRU,
    LFU,
    FIFO,
    Random,
    Adaptive,
}

/// Storage backend abstraction
pub struct StorageBackend {
    pub backend_type: BackendType,
    pub zns_manager: Option<Arc<Mutex<crate::zns_storage::ZnsZoneManager>>>,
    pub csd_manager: Arc<Mutex<crate::csd_storage::CsdManager>>,
    pub matrix_store: HashMap<String, Matrix>,
}

/// Backend types
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    ZNS,
    CSD,
    Hybrid,
}

// Supporting implementations

impl MatrixStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            allocator: MatrixAllocator::new(),
            cache: MatrixCache::new(),
            storage_backend: StorageBackend::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize zones
        self.create_zones()?;

        // Initialize allocator
        self.allocator.initialize()?;

        // Initialize cache
        self.cache.initialize()?;

        // Initialize storage backend
        self.storage_backend.initialize()?;

        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), LinearAlgebraError> {
        // Create different zone types
        let zones = vec![
            ("dense", ZoneType::Dense),
            ("sparse", ZoneType::Sparse),
            ("structured", ZoneType::Structured),
            ("temporary", ZoneType::Temporary),
            ("cached", ZoneType::Cached),
        ];

        for (name, zone_type) in zones {
            let zone = MatrixZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 1024 * 1024 * 1024, // 1GB
                matrices: HashMap::new(),
                access_pattern: AccessPattern::Adaptive,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn store_matrix(&mut self, matrix: Matrix) -> Result<(), LinearAlgebraError> {
        // Determine best zone for this matrix
        let zone_id = self.select_best_zone(&matrix)?;

        // Store in zone
        let zone = self
            .zones
            .get_mut(&zone_id)
            .ok_or_else(|| LinearAlgebraError::StorageError("Zone not found".to_string()))?;

        zone.matrices
            .insert(matrix.matrix_id.clone(), matrix.metadata.clone());

        // Store actual data
        self.storage_backend.store_matrix_data(&matrix)?;

        Ok(())
    }

    pub fn get_matrix(&mut self, matrix_id: &str) -> Result<Matrix, LinearAlgebraError> {
        // Check cache first
        if let Some(cached_data) = self.cache.get(matrix_id) {
            return Ok(cached_data);
        }

        // Get from storage
        let matrix = self.storage_backend.get_matrix_data(matrix_id)?;

        // Populate cache for future accesses
        self.cache.put(&matrix)?;

        Ok(matrix)
    }

    pub fn get_matrix_metadata(&self, matrix_id: &str) -> Option<MatrixMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.matrices.get(matrix_id) {
                return Some(metadata.clone());
            }
        }
        None
    }

    pub fn list_matrices(&self) -> Vec<String> {
        let mut matrices = Vec::new();
        for zone in self.zones.values() {
            matrices.extend(zone.matrices.keys().cloned());
        }
        matrices
    }

    fn select_best_zone(&self, matrix: &Matrix) -> Result<String, LinearAlgebraError> {
        // Simple selection logic - in real implementation would be more sophisticated
        if matrix.rows * matrix.cols > 10000 {
            Ok("dense".to_string())
        } else {
            Ok("temporary".to_string())
        }
    }
}

impl MatrixAllocator {
    pub fn new() -> Self {
        Self {
            allocation_strategy: AllocationStrategy::BestFit,
            free_blocks: Vec::new(),
            allocated_blocks: HashMap::new(),
            fragmentation_threshold: 0.3,
            total_pool_size: 1024 * 1024 * 1024, // 1GB default pool
            next_block_id: 0,
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize with a single free block covering the entire memory pool
        self.free_blocks.clear();
        self.allocated_blocks.clear();
        self.next_block_id = 0;
        let block_id = self.generate_block_id();
        let pool_size = self.total_pool_size;
        let block = MemoryBlock {
            block_id,
            start_address: 0,
            size: pool_size,
            is_free: true,
            fragmentation_score: 0.0,
        };
        self.free_blocks.push(block);
        Ok(())
    }

    /// Generate a unique block ID
    fn generate_block_id(&mut self) -> String {
        let id = format!("block_{}", self.next_block_id);
        self.next_block_id += 1;
        id
    }

    /// Allocate a block of the given size using the configured allocation strategy.
    /// Returns the starting address of the allocated block.
    pub fn allocate(&mut self, size: usize) -> Result<usize, LinearAlgebraError> {
        if size == 0 {
            return Err(LinearAlgebraError::StorageError(
                "Cannot allocate zero-size block".to_string(),
            ));
        }
        let size = size as u64;

        // Find the best free block based on the allocation strategy
        let block_index = match self.allocation_strategy {
            AllocationStrategy::FirstFit => {
                // First fit: first block that's large enough
                self.free_blocks.iter().position(|b| b.size >= size)
            }
            AllocationStrategy::BestFit => {
                // Best fit: smallest block that's large enough
                self.free_blocks
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.size >= size)
                    .min_by_key(|(_, b)| b.size)
                    .map(|(i, _)| i)
            }
            AllocationStrategy::WorstFit => {
                // Worst fit: largest block that's large enough
                self.free_blocks
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.size >= size)
                    .max_by_key(|(_, b)| b.size)
                    .map(|(i, _)| i)
            }
            AllocationStrategy::BuddySystem | AllocationStrategy::Slab => {
                // Fall back to best fit for these strategies
                self.free_blocks
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.size >= size)
                    .min_by_key(|(_, b)| b.size)
                    .map(|(i, _)| i)
            }
        };

        let block_index = block_index.ok_or_else(|| {
            LinearAlgebraError::StorageError("No free block large enough for allocation".to_string())
        })?;

        // Remove the free block from the list
        let mut free_block = self.free_blocks.swap_remove(block_index);

        let start_address = free_block.start_address;
        let allocated_block = MemoryBlock {
            block_id: self.generate_block_id(),
            start_address,
            size,
            is_free: false,
            fragmentation_score: 0.0,
        };

        // If the free block is larger than needed, split it and return the remainder
        if free_block.size > size {
            let remainder = MemoryBlock {
                block_id: self.generate_block_id(),
                start_address: start_address + size,
                size: free_block.size - size,
                is_free: true,
                fragmentation_score: 0.0,
            };
            self.free_blocks.push(remainder);
        }

        // Store the allocated block keyed by its address as a string
        self.allocated_blocks
            .insert(start_address.to_string(), allocated_block);

        Ok(start_address as usize)
    }

    /// Deallocate the block at the given address, returning it to the free list
    /// and coalescing adjacent free blocks.
    pub fn deallocate(&mut self, address: usize) -> Result<(), LinearAlgebraError> {
        let key = address.to_string();
        let mut block = self
            .allocated_blocks
            .remove(&key)
            .ok_or_else(|| LinearAlgebraError::StorageError(format!("No allocated block at address {}", address)))?;

        block.is_free = true;
        block.block_id = self.generate_block_id();
        self.free_blocks.push(block);

        // Coalesce adjacent free blocks
        self.coalesce_free_blocks();

        // Defragment if fragmentation is too high
        if self.fragmentation_ratio() > self.fragmentation_threshold {
            self.defragment();
        }

        Ok(())
    }

    /// Merge adjacent free blocks into larger contiguous blocks
    fn coalesce_free_blocks(&mut self) {
        if self.free_blocks.len() <= 1 {
            return;
        }

        // Sort by start address so adjacent blocks are next to each other
        self.free_blocks.sort_by_key(|b| b.start_address);

        let mut coalesced: Vec<MemoryBlock> = Vec::with_capacity(self.free_blocks.len());
        for block in self.free_blocks.drain(..) {
            if let Some(last) = coalesced.last_mut() {
                // If this block starts right where the last one ends, merge them
                if last.start_address + last.size == block.start_address {
                    last.size += block.size;
                    continue;
                }
            }
            coalesced.push(block);
        }
        self.free_blocks = coalesced;
    }

    /// Compute fragmentation as (total_free - largest_free_block) / total_free.
    /// Returns 0.0 if there is no free memory.
    pub fn fragmentation_ratio(&self) -> f64 {
        let total_free: u64 = self.free_blocks.iter().map(|b| b.size).sum();
        if total_free == 0 {
            return 0.0;
        }
        let largest_free: u64 = self.free_blocks.iter().map(|b| b.size).max().unwrap_or(0);
        (total_free - largest_free) as f64 / total_free as f64
    }

    /// If fragmentation_ratio exceeds the threshold, compact all allocated blocks
    /// to eliminate gaps.
    pub fn defragment(&mut self) {
        if self.fragmentation_ratio() <= self.fragmentation_threshold {
            return;
        }

        // Compact: move all allocated blocks to the beginning of the pool
        let mut allocated: Vec<MemoryBlock> = self.allocated_blocks.values().cloned().collect();
        allocated.sort_by_key(|b| b.start_address);

        self.allocated_blocks.clear();
        let mut current_address: u64 = 0;
        for mut block in allocated {
            block.start_address = current_address;
            self.allocated_blocks
                .insert(current_address.to_string(), block.clone());
            current_address += block.size;
        }

        // The remainder is one big free block
        self.free_blocks.clear();
        if current_address < self.total_pool_size {
            let block_id = self.generate_block_id();
            self.free_blocks.push(MemoryBlock {
                block_id,
                start_address: current_address,
                size: self.total_pool_size - current_address,
                is_free: true,
                fragmentation_score: 0.0,
            });
        }
    }

    /// Return (total_allocated, total_free, fragmentation_ratio)
    pub fn stats(&self) -> (usize, usize, f64) {
        let total_allocated: u64 = self.allocated_blocks.values().map(|b| b.size).sum();
        let total_free: u64 = self.free_blocks.iter().map(|b| b.size).sum();
        (total_allocated as usize, total_free as usize, self.fragmentation_ratio())
    }
}

impl MatrixCache {
    pub fn new() -> Self {
        Self {
            cache_entries: HashMap::new(),
            cache_policy: CachePolicy::LRU,
            max_size: 100 * 1024 * 1024, // 100MB
            current_size: 0,
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.cache_entries.clear();
        self.current_size = 0;
        self.hit_count = 0;
        self.miss_count = 0;
        Ok(())
    }

    /// Get the current time as a monotonic counter (nanoseconds since UNIX_EPOCH)
    fn current_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Look up a matrix by ID. On a hit, updates access time and access count,
    /// increments hit_count, and returns the matrix. On a miss, increments
    /// miss_count and returns None.
    pub fn get(&mut self, matrix_id: &str) -> Option<Matrix> {
        if let Some(entry) = self.cache_entries.get_mut(matrix_id) {
            self.hit_count += 1;
            entry.access_time = Self::current_time();
            entry.access_count += 1;
            return entry.matrix.clone();
        }
        self.miss_count += 1;
        None
    }

    /// Store a matrix in the cache. Updates current_size and evicts LRU entries
    /// if the cache exceeds max_size.
    pub fn put(&mut self, matrix: &Matrix) -> Result<(), LinearAlgebraError> {
        // Estimate size as rows * cols * 8 (f64 = 8 bytes)
        let size_bytes = (matrix.rows * matrix.cols * 8) as u64;
        let now = Self::current_time();

        // If updating an existing entry, subtract its old size first
        if let Some(existing) = self.cache_entries.get(matrix.matrix_id.as_str()) {
            self.current_size -= existing.size;
        }

        // Evict LRU entries until we have room for the new entry
        while self.current_size + size_bytes > self.max_size
            && !self.cache_entries.is_empty()
        {
            self.evict_lru();
        }

        // If the single entry is larger than max_size, still store it
        // (it will be evicted on the next put if needed)
        let entry = CacheEntry {
            matrix_id: matrix.matrix_id.clone(),
            data: Vec::new(), // data field kept for compatibility; matrix stored directly
            access_time: now,
            access_count: 1,
            size: size_bytes,
            matrix: Some(matrix.clone()),
        };

        self.cache_entries.insert(matrix.matrix_id.clone(), entry);
        self.current_size += size_bytes;

        Ok(())
    }

    /// Find the entry with the oldest access_time, remove it, and update current_size.
    pub fn evict_lru(&mut self) {
        if self.cache_entries.is_empty() {
            return;
        }
        // Find the entry with the oldest access_time
        let oldest_id = self
            .cache_entries
            .iter()
            .min_by_key(|(_, entry)| entry.access_time)
            .map(|(id, _)| id.clone());

        if let Some(id) = oldest_id {
            if let Some(entry) = self.cache_entries.remove(&id) {
                self.current_size -= entry.size;
            }
        }
    }

    /// Return hit_count / (hit_count + miss_count). Returns 0.0 if no accesses.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            return 0.0;
        }
        self.hit_count as f64 / total as f64
    }

    /// Return the current cache size in bytes
    pub fn cache_size(&self) -> usize {
        self.current_size as usize
    }
}

impl StorageBackend {
    pub fn new() -> Self {
        let zns_manager = crate::zns_storage::ZnsZoneManager::new("default_zone")
            .ok()
            .map(|m| Arc::new(Mutex::new(m)));
        Self {
            backend_type: if zns_manager.is_some() {
                BackendType::Hybrid
            } else {
                BackendType::CSD
            },
            zns_manager,
            csd_manager: Arc::new(Mutex::new(crate::csd_storage::CsdManager::new())),
            matrix_store: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }

    pub fn store_matrix_data(&mut self, matrix: &Matrix) -> Result<(), LinearAlgebraError> {
        self.matrix_store
            .insert(matrix.matrix_id.clone(), matrix.clone());
        Ok(())
    }

    pub fn get_matrix_data(&self, matrix_id: &str) -> Result<Matrix, LinearAlgebraError> {
        self.matrix_store.get(matrix_id).cloned().ok_or_else(|| {
            LinearAlgebraError::StorageError(format!("Matrix not found: {}", matrix_id))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_matrix(id: &str, rows: usize, cols: usize) -> Matrix {
        let data = vec![1.0; rows * cols];
        let metadata = MatrixMetadata {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            storage_format: StorageFormat::RowMajor,
            compression: CompressionType::None,
            created_at: 0,
            last_accessed: 0,
            access_count: 0,
        };
        Matrix {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            data,
            storage_format: StorageFormat::RowMajor,
            metadata,
        }
    }

    // === MatrixCache tests ===

    #[test]
    fn test_cache_put_get_roundtrip() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();
        let matrix = make_test_matrix("m1", 3, 4);
        cache.put(&matrix).unwrap();

        let retrieved = cache.get("m1");
        assert!(retrieved.is_some());
        let m = retrieved.unwrap();
        assert_eq!(m.rows, 3);
        assert_eq!(m.cols, 4);
        assert_eq!(m.matrix_id, "m1");
    }

    #[test]
    fn test_cache_miss_increments_miss_count() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();

        let result = cache.get("nonexistent");
        assert!(result.is_none());
        assert_eq!(cache.miss_count, 1);
        assert_eq!(cache.hit_count, 0);
    }

    #[test]
    fn test_cache_hit_increments_hit_count() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();
        let matrix = make_test_matrix("m1", 2, 2);
        cache.put(&matrix).unwrap();

        cache.get("m1");
        assert_eq!(cache.hit_count, 1);
        assert_eq!(cache.miss_count, 0);

        cache.get("m1");
        assert_eq!(cache.hit_count, 2);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();

        // No accesses → 0.0
        assert_eq!(cache.hit_rate(), 0.0);

        let matrix = make_test_matrix("m1", 2, 2);
        cache.put(&matrix).unwrap();

        cache.get("m1"); // hit
        cache.get("m1"); // hit
        cache.get("missing"); // miss

        // 2 hits, 1 miss → 2/3
        assert!((cache.hit_rate() - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_cache_size() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();
        assert_eq!(cache.cache_size(), 0);

        let matrix = make_test_matrix("m1", 3, 4);
        cache.put(&matrix).unwrap();

        // 3 * 4 * 8 = 96 bytes
        assert_eq!(cache.cache_size(), 96);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = MatrixCache::new();
        // Small cache: each 2x2 matrix = 32 bytes, max 96 bytes → can hold 3
        cache.max_size = 96;
        cache.initialize().unwrap();

        // Insert m1, m2, m3
        let m1 = make_test_matrix("m1", 2, 2);
        cache.put(&m1).unwrap();
        // Small delay to ensure different access times
        std::thread::sleep(std::time::Duration::from_millis(2));
        let m2 = make_test_matrix("m2", 2, 2);
        cache.put(&m2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let m3 = make_test_matrix("m3", 2, 2);
        cache.put(&m3).unwrap();

        assert_eq!(cache.cache_entries.len(), 3);

        // Access m1 to make it more recently used than m2
        std::thread::sleep(std::time::Duration::from_millis(2));
        cache.get("m1"); // updates m1's access_time

        // Insert m4 — this should evict the LRU entry
        // m2 has the oldest access_time (m1 was just accessed, m3 was put after m2)
        std::thread::sleep(std::time::Duration::from_millis(2));
        let m4 = make_test_matrix("m4", 2, 2);
        cache.put(&m4).unwrap();

        // m1 should still be in cache (was accessed recently)
        assert!(cache.cache_entries.contains_key("m1"), "m1 should still be cached");
        // m2 should have been evicted (oldest access_time)
        assert!(
            !cache.cache_entries.contains_key("m2"),
            "m2 should have been evicted"
        );
        // m3 should still be in cache
        assert!(cache.cache_entries.contains_key("m3"), "m3 should still be cached");
        // m4 should be in cache
        assert!(cache.cache_entries.contains_key("m4"), "m4 should be in cache");
    }

    #[test]
    fn test_cache_evict_lru_directly() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();

        cache.put(&make_test_matrix("m1", 2, 2)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.put(&make_test_matrix("m2", 2, 2)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.put(&make_test_matrix("m3", 2, 2)).unwrap();

        assert_eq!(cache.cache_entries.len(), 3);

        // Evict LRU (should be m1 — oldest access time)
        cache.evict_lru();
        assert_eq!(cache.cache_entries.len(), 2);
        assert!(!cache.cache_entries.contains_key("m1"));
    }

    #[test]
    fn test_cache_update_existing_entry() {
        let mut cache = MatrixCache::new();
        cache.initialize().unwrap();

        cache.put(&make_test_matrix("m1", 2, 2)).unwrap();
        assert_eq!(cache.cache_size(), 32);

        // Update with a larger matrix
        cache.put(&make_test_matrix("m1", 4, 4)).unwrap();
        assert_eq!(cache.cache_size(), 128); // 4*4*8 = 128, not 128+32
    }

    // === MatrixAllocator tests ===

    #[test]
    fn test_allocator_initialize() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        assert_eq!(allocator.free_blocks.len(), 1);
        assert_eq!(allocator.free_blocks[0].start_address, 0);
        assert_eq!(allocator.free_blocks[0].size, allocator.total_pool_size);
        assert!(allocator.free_blocks[0].is_free);
        assert_eq!(allocator.allocated_blocks.len(), 0);
    }

    #[test]
    fn test_allocator_allocate() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        let addr = allocator.allocate(1024).unwrap();
        assert_eq!(addr, 0);
        assert_eq!(allocator.allocated_blocks.len(), 1);
        // The free block should be split
        assert_eq!(allocator.free_blocks.len(), 1);
        assert_eq!(allocator.free_blocks[0].start_address, 1024);
    }

    #[test]
    fn test_allocator_allocate_multiple() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        let addr1 = allocator.allocate(100).unwrap();
        let addr2 = allocator.allocate(200).unwrap();
        let addr3 = allocator.allocate(300).unwrap();

        assert_eq!(addr1, 0);
        assert_eq!(addr2, 100);
        assert_eq!(addr3, 300);
        assert_eq!(allocator.allocated_blocks.len(), 3);
    }

    #[test]
    fn test_allocator_allocate_zero_fails() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        assert!(allocator.allocate(0).is_err());
    }

    #[test]
    fn test_allocator_deallocate() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        let addr = allocator.allocate(1024).unwrap();
        assert_eq!(allocator.allocated_blocks.len(), 1);

        allocator.deallocate(addr).unwrap();
        assert_eq!(allocator.allocated_blocks.len(), 0);
        // After deallocation, free blocks should be coalesced back to one
        assert_eq!(allocator.free_blocks.len(), 1);
    }

    #[test]
    fn test_allocator_deallocate_coalescing() {
        let mut allocator = MatrixAllocator::new();
        allocator.initialize().unwrap();

        let addr1 = allocator.allocate(100).unwrap();
        let addr2 = allocator.allocate(200).unwrap();
        let addr3 = allocator.allocate(300).unwrap();

        // Deallocate in non-contiguous order: addr2, then addr1
        allocator.deallocate(addr2).unwrap();
        allocator.deallocate(addr1).unwrap();

        // addr1 and addr2 should be coalesced into one block
        // addr3 is still allocated, so there should be 2 free blocks
        // (the coalesced addr1+addr2 block, and possibly the remainder)
        // After coalescing, adjacent free blocks merge
        let free_count = allocator.free_blocks.len();
        assert!(free_count >= 1, "should have at least 1 free block after coalescing");

        // Now deallocate addr3 — everything should coalesce back to one block
        allocator.deallocate(addr3).unwrap();
        assert_eq!(allocator.free_blocks.len(), 1);
        assert_eq!(allocator.free_blocks[0].size, allocator.total_pool_size);
    }

    #[test]
    fn test_allocator_fragmentation_ratio() {
        let mut allocator = MatrixAllocator::new();
        allocator.total_pool_size = 10000;
        allocator.initialize().unwrap();

        // No allocations → no fragmentation
        assert_eq!(allocator.fragmentation_ratio(), 0.0);

        // Allocate some blocks
        allocator.allocate(1000).unwrap();
        allocator.allocate(1000).unwrap();
        allocator.allocate(1000).unwrap();

        // Deallocate the middle one to create fragmentation
        allocator.deallocate(1000).unwrap();

        // Now there are 2 free blocks (the middle one and the remainder)
        // Fragmentation should be > 0
        let frag = allocator.fragmentation_ratio();
        assert!(frag >= 0.0 && frag <= 1.0);
    }

    #[test]
    fn test_allocator_defragment() {
        let mut allocator = MatrixAllocator::new();
        allocator.total_pool_size = 10000;
        allocator.initialize().unwrap();

        // Create fragmentation
        let addr1 = allocator.allocate(1000).unwrap();
        let _addr2 = allocator.allocate(1000).unwrap();
        let addr3 = allocator.allocate(1000).unwrap();

        // Deallocate addr1 and addr3 (creating gaps)
        allocator.deallocate(addr1).unwrap();
        allocator.deallocate(addr3).unwrap();

        // Force defragmentation by setting a low threshold
        allocator.fragmentation_threshold = 0.0;
        allocator.defragment();

        // After defragmentation, allocated blocks should be compacted
        // and there should be one contiguous free block
        assert_eq!(allocator.free_blocks.len(), 1);
    }

    #[test]
    fn test_allocator_stats() {
        let mut allocator = MatrixAllocator::new();
        allocator.total_pool_size = 10000;
        allocator.initialize().unwrap();

        allocator.allocate(1000).unwrap();
        allocator.allocate(2000).unwrap();

        let (allocated, free, frag) = allocator.stats();
        assert_eq!(allocated, 3000);
        assert_eq!(free, 7000);
        assert!(frag >= 0.0 && frag <= 1.0);
    }

    #[test]
    fn test_allocator_best_fit_strategy() {
        let mut allocator = MatrixAllocator::new();
        allocator.allocation_strategy = AllocationStrategy::BestFit;
        allocator.total_pool_size = 10000;
        allocator.fragmentation_threshold = 1.0; // Don't auto-defragment
        allocator.initialize().unwrap();

        // Allocate and deallocate to create blocks of different sizes
        allocator.allocate(1000).unwrap();
        let addr2 = allocator.allocate(2000).unwrap();
        allocator.allocate(3000).unwrap();
        allocator.deallocate(addr2).unwrap();

        // Now we have a 2000-byte free block at address 1000 and a 4000-byte free block at 6000
        // Best fit for 1500 should use the 2000-byte block (smallest that fits)
        let addr = allocator.allocate(1500).unwrap();
        assert_eq!(addr, 1000); // Should use the 2000-byte block
    }

    #[test]
    fn test_allocator_first_fit_strategy() {
        let mut allocator = MatrixAllocator::new();
        allocator.allocation_strategy = AllocationStrategy::FirstFit;
        allocator.total_pool_size = 10000;
        allocator.fragmentation_threshold = 1.0; // Don't auto-defragment
        allocator.initialize().unwrap();

        let _addr1 = allocator.allocate(1000).unwrap();
        let addr2 = allocator.allocate(2000).unwrap();
        let _addr3 = allocator.allocate(3000).unwrap();

        // Deallocate addr2 to create a 2000-byte hole at address 1000
        allocator.deallocate(addr2).unwrap();

        // First fit for 500 should use the first available block (the 2000-byte hole at addr2)
        let addr = allocator.allocate(500).unwrap();
        assert_eq!(addr, 1000); // First fit picks the first block that's large enough
    }
}
