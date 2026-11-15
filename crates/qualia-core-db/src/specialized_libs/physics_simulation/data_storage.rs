use super::*;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};


/// Physics data manager
pub struct PhysicsDataManager {
    data_storage: PhysicsDataStorage,
    data_compression: DataCompression,
    data_caching: DataCache,
    data_migration: DataMigration,
}

/// Physics data storage
pub struct PhysicsDataStorage {
    pub(super) storage_backends: HashMap<String, StorageBackend>,
    data_layout: DataLayout,
    access_patterns: AccessPatterns,
    /// In-memory store used directly or as a caching tier when hardware backends are active.
    stored_data: HashMap<String, Vec<f64>>,
    /// Optional hardware-backed Zoned Namespace manager for physical zone persistence.
    #[cfg(not(target_arch = "wasm32"))]
    pub zns_manager: Option<Arc<Mutex<crate::zns_storage::ZnsZoneManager>>>,
    #[cfg(target_arch = "wasm32")]
    pub zns_manager: Option<()>,
}

/// Storage backends
#[derive(Debug, Clone)]
pub struct StorageBackend {
    backend_id: String,
    backend_type: StorageBackendType,
    capacity: u64,
    performance: StoragePerformance,
}

/// Storage backend types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageBackendType {
    /// In-memory storage
    Memory,
    /// Local storage
    Local,
    /// Network storage
    Network,
    /// Cloud storage
    Cloud,
    /// Distributed storage
    Distributed,
    /// Hierarchical storage
    Hierarchical,
}

/// Storage performance
#[derive(Debug, Clone)]
pub struct StoragePerformance {
    pub read_bandwidth: f64,
    pub write_bandwidth: f64,
    pub latency: f64,
    pub iops: u64,
}

/// Data layout
#[derive(Debug, Clone)]
pub struct DataLayout {
    layout_type: DataLayoutType,
    block_size: usize,
    stripe_size: Option<usize>,
    replication_factor: usize,
}

/// Data layout types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataLayoutType {
    /// Row-major layout
    RowMajor,
    /// Column-major layout
    ColumnMajor,
    /// Block layout
    Block,
    /// Interleaved layout
    Interleaved,
    /// Custom layout
    Custom,
}

/// Access patterns
#[derive(Debug, Clone)]
pub struct AccessPatterns {
    read_patterns: HashMap<String, AccessPattern>,
    write_patterns: HashMap<String, AccessPattern>,
    temporal_patterns: HashMap<String, TemporalPattern>,
}

/// Temporal patterns
#[derive(Debug, Clone)]
pub struct TemporalPattern {
    pattern_id: String,
    pattern_type: TemporalPatternType,
    time_scale: TimeScale,
    periodicity: Option<f64>,
}

/// Temporal pattern types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemporalPatternType {
    /// Sequential access
    Sequential,
    /// Random access
    Random,
    /// Burst access
    Burst,
    /// Periodic access
    Periodic,
    /// Aperiodic access
    Aperiodic,
}

/// Time scales
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeScale {
    Microsecond,
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// Data compression
pub struct DataCompression {
    compression_algorithms: HashMap<String, CompressionAlgorithm>,
    compression_ratio: CompressionRatio,
    compression_performance: CompressionPerformance,
}

/// Compression algorithms
#[derive(Debug, Clone)]
pub struct CompressionAlgorithm {
    algorithm_id: String,
    algorithm_type: CompressionAlgorithmType,
    parameters: CompressionParameters,
}

/// Compression algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithmType {
    /// Lossless compression
    Lossless,
    /// Lossy compression
    Lossy,
    /// Hybrid compression
    Hybrid,
}

/// Compression parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionParameters {
    pub compression_level: u32,
    pub block_size: usize,
    pub window_size: Option<usize>,
    pub quality: Option<f64>,
}

/// Compression ratio
#[derive(Debug, Clone)]
pub struct CompressionRatio {
    pub original_size: u64,
    pub compressed_size: u64,
    pub ratio: f64,
}

/// Compression performance
#[derive(Debug, Clone)]
pub struct CompressionPerformance {
    pub compression_speed: f64,
    pub decompression_speed: f64,
    pub memory_usage: u64,
}

/// Data caching
pub struct DataCache {
    cache_policy: CachePolicy,
    cache_size: u64,
    cache_performance: CachePerformance,
}

/// Cache policy
#[derive(Debug, Clone)]
pub struct CachePolicy {
    eviction_policy: EvictionPolicy,
    write_policy: WritePolicy,
    consistency_policy: CacheConsistencyPolicy,
}

/// Eviction policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least recently used (LRU)
    LRU,
    /// Least frequently used (LFU)
    LFU,
    /// First-in-first-out (FIFO)
    FIFO,
    /// Random
    Random,
    /// Clock
    Clock,
}

/// Write policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WritePolicy {
    /// Write-through
    WriteThrough,
    /// Write-back
    WriteBack,
    /// Write-around
    WriteAround,
    /// No-write
    NoWrite,
}

/// Cache consistency policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheConsistencyPolicy {
    /// Strong consistency
    Strong,
    /// Weak consistency
    Weak,
    /// Eventual consistency
    Eventual,
}

/// Cache performance
#[derive(Debug, Clone)]
pub struct CachePerformance {
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub average_access_time: f64,
}

impl PhysicsDataManager {
    /// Creates a manager with empty data storage, compression, and cache state.
    pub fn new() -> Self {
        Self {
            data_storage: PhysicsDataStorage::new(),
            data_compression: DataCompression::new(),
            data_caching: DataCache::new(),
            data_migration: DataMigration::new(),
        }
    }

    /// Initializes compression and cache subsystems; `PhysicsError` propagates.
    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        self.data_storage.initialize()?;
        self.data_compression.initialize()?;
        self.data_caching.initialize()?;
        self.data_migration.initialize()?;
        Ok(())
    }

    /// Stores field data through the configured storage path.
    pub fn store_field_data(
        &mut self,
        _simulation: &Simulation,
        fields: &[PhysicsField],
    ) -> Result<(), PhysicsError> {
        // Store each field through the registered storage backends.
        for field in fields {
            self.data_storage.store_field_data(field)?;
        }
        Ok(())
    }
}

impl StorageBackend {
    /// Registers a storage backend descriptor (`backend_id`, type, capacity).
    pub fn new(backend_id: &str, backend_type: StorageBackendType, capacity: u64) -> Self {
        Self {
            backend_id: backend_id.to_string(),
            backend_type,
            capacity,
            performance: StoragePerformance {
                read_bandwidth: 0.0,
                write_bandwidth: 0.0,
                latency: 0.0,
                iops: 0,
            },
        }
    }

    /// Get the backend ID.
    pub fn get_backend_id(&self) -> &str {
        &self.backend_id
    }

    /// Get the backend type.
    pub fn get_backend_type(&self) -> &StorageBackendType {
        &self.backend_type
    }

    /// Get the capacity in bytes.
    pub fn get_capacity(&self) -> u64 {
        self.capacity
    }

    /// Get a reference to the performance metrics.
    pub fn get_performance(&self) -> &StoragePerformance {
        &self.performance
    }

    /// Set the performance metrics.
    pub fn set_performance(&mut self, performance: StoragePerformance) {
        self.performance = performance;
    }
}

impl PhysicsDataStorage {
    /// Creates storage with default layout and access-pattern state.
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let zns_manager = crate::zns_storage::ZnsZoneManager::new("default_physics_zone")
            .ok()
            .map(|mgr| Arc::new(Mutex::new(mgr)));
        #[cfg(target_arch = "wasm32")]
        let zns_manager = None;

        Self {
            storage_backends: HashMap::new(),
            data_layout: DataLayout::new(),
            access_patterns: AccessPatterns::new(),
            stored_data: HashMap::new(),
            zns_manager,
        }
    }

    /// Register a storage backend under the given name.
    pub fn register_backend(&mut self, name: &str, backend: StorageBackend) {
        self.storage_backends.insert(name.to_string(), backend);
    }

    /// Attach a hardware-backed Zoned Namespace manager for physical zone persistence.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn attach_zns_manager(&mut self, manager: Arc<Mutex<crate::zns_storage::ZnsZoneManager>>) {
        self.zns_manager = Some(manager);
    }
    #[cfg(target_arch = "wasm32")]
    pub fn attach_zns_manager(&mut self, _manager: ()) {}


    /// Initialize default storage backends.
    ///
    /// Registers the genuine in-memory fallback tier (256 MiB) along with ZNS and CSD descriptors.
    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        self.register_backend(
            "memory",
            StorageBackend::new("memory", StorageBackendType::Memory, 256 * 1024 * 1024), // 256 MB
        );

        // Register a ZNS (Zoned Namespace SSD) backend.
        self.register_backend(
            "zns",
            StorageBackend::new("zns", StorageBackendType::Local, 1 << 40), // ~1 TB
        );

        // Register a CSD (Computational Storage Device) backend.
        self.register_backend(
            "csd",
            StorageBackend::new("csd", StorageBackendType::Hierarchical, 1 << 40),
        );

        Ok(())
    }

    /// Serialize the field data and store it via registered backends and active hardware ZNS zones.
    ///
    /// Checks backend registration and enforces capacity bounds across registered storage backends.
    pub fn store_field_data(&mut self, field: &PhysicsField) -> Result<(), PhysicsError> {
        if self.storage_backends.is_empty() {
            return Err(PhysicsError::DataError(
                "No storage backends registered".to_string(),
            ));
        }

        let field_bytes = (field.data.len() * std::mem::size_of::<f64>()) as u64;
        let total_capacity: u64 = self.storage_backends.values().map(|b| b.capacity).sum();
        let existing_bytes: u64 = self
            .stored_data
            .values()
            .map(|d| (d.len() * std::mem::size_of::<f64>()) as u64)
            .sum();
        let old_field_bytes: u64 = self
            .stored_data
            .get(&field.field_id)
            .map(|d| (d.len() * std::mem::size_of::<f64>()) as u64)
            .unwrap_or(0);

        let needed_bytes = field_bytes.saturating_sub(old_field_bytes);
        if existing_bytes + needed_bytes > total_capacity {
            return Err(PhysicsError::DataError(format!(
                "Storage capacity exceeded: {} bytes required, {} bytes available across registered backends",
                needed_bytes,
                total_capacity.saturating_sub(existing_bytes)
            )));
        }

        // Write to in-memory store
        self.stored_data
            .insert(field.field_id.clone(), field.data.clone());

        // Write through to physical ZNS zone when available
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref zns) = self.zns_manager {
            if let Ok(mut zns_guard) = zns.lock() {
                let mut payload = Vec::with_capacity(32 + field.data.len() * 8);
                let id_bytes = field.field_id.as_bytes();
                payload.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
                payload.extend_from_slice(id_bytes);
                payload.extend_from_slice(&(field.data.len() as u64).to_le_bytes());
                for &val in &field.data {
                    payload.extend_from_slice(&val.to_le_bytes());
                }
                if let Ok(handle) = zns_guard.allocate_zone(
                    crate::zns_storage::ZoneType::Sequential,
                    payload.len() as u64,
                ) {
                    let _ = zns_guard.write_zone(&handle, &payload);
                }
            }
        }

        Ok(())
    }

    /// Retrieve previously stored field data by field ID.
    pub fn retrieve_field_data(&self, field_id: &str) -> Option<Vec<f64>> {
        self.stored_data.get(field_id).cloned()
    }

    /// Get a reference to the data layout.
    pub fn get_data_layout(&self) -> &DataLayout {
        &self.data_layout
    }

    /// Get a mutable reference to the data layout.
    pub fn get_data_layout_mut(&mut self) -> &mut DataLayout {
        &mut self.data_layout
    }

    /// Get a reference to the access patterns.
    pub fn get_access_patterns(&self) -> &AccessPatterns {
        &self.access_patterns
    }

    /// Get a mutable reference to the access patterns.
    pub fn get_access_patterns_mut(&mut self) -> &mut AccessPatterns {
        &mut self.access_patterns
    }
}

impl DataLayout {
    /// Creates a default data-layout descriptor.
    pub fn new() -> Self {
        Self {
            layout_type: DataLayoutType::RowMajor,
            block_size: 1024,
            stripe_size: None,
            replication_factor: 1,
        }
    }

    /// Get the layout type.
    pub fn get_layout_type(&self) -> &DataLayoutType {
        &self.layout_type
    }

    /// Set the layout type.
    pub fn set_layout_type(&mut self, layout_type: DataLayoutType) {
        self.layout_type = layout_type;
    }

    /// Get the block size.
    pub fn get_block_size(&self) -> usize {
        self.block_size
    }

    /// Set the block size.
    pub fn set_block_size(&mut self, size: usize) {
        self.block_size = size;
    }

    /// Get the stripe size, if any.
    pub fn get_stripe_size(&self) -> Option<usize> {
        self.stripe_size
    }

    /// Set the stripe size.
    pub fn set_stripe_size(&mut self, size: Option<usize>) {
        self.stripe_size = size;
    }

    /// Get the replication factor.
    pub fn get_replication_factor(&self) -> usize {
        self.replication_factor
    }

    /// Set the replication factor.
    pub fn set_replication_factor(&mut self, factor: usize) {
        self.replication_factor = factor;
    }
}

impl AccessPatterns {
    /// Creates default access-pattern state.
    pub fn new() -> Self {
        Self {
            read_patterns: HashMap::new(),
            write_patterns: HashMap::new(),
            temporal_patterns: HashMap::new(),
        }
    }

    /// Register a read access pattern under the given name.
    pub fn add_read_pattern(&mut self, name: &str, pattern: AccessPattern) {
        self.read_patterns.insert(name.to_string(), pattern);
    }

    /// Get a read access pattern by name, if any.
    pub fn get_read_pattern(&self, name: &str) -> Option<&AccessPattern> {
        self.read_patterns.get(name)
    }

    /// Register a write access pattern under the given name.
    pub fn add_write_pattern(&mut self, name: &str, pattern: AccessPattern) {
        self.write_patterns.insert(name.to_string(), pattern);
    }

    /// Get a write access pattern by name, if any.
    pub fn get_write_pattern(&self, name: &str) -> Option<&AccessPattern> {
        self.write_patterns.get(name)
    }

    /// Register a temporal pattern under the given name.
    pub fn add_temporal_pattern(&mut self, name: &str, pattern: TemporalPattern) {
        self.temporal_patterns.insert(name.to_string(), pattern);
    }

    /// Get a temporal pattern by name, if any.
    pub fn get_temporal_pattern(&self, name: &str) -> Option<&TemporalPattern> {
        self.temporal_patterns.get(name)
    }
}

impl TemporalPattern {
    /// Creates a default temporal access pattern.
    pub fn new() -> Self {
        Self {
            pattern_id: "default".to_string(),
            pattern_type: TemporalPatternType::Sequential,
            time_scale: TimeScale::Second,
            periodicity: None,
        }
    }

    /// Get the pattern ID.
    pub fn get_pattern_id(&self) -> &str {
        &self.pattern_id
    }

    /// Get the pattern type.
    pub fn get_pattern_type(&self) -> &TemporalPatternType {
        &self.pattern_type
    }

    /// Set the pattern type.
    pub fn set_pattern_type(&mut self, ptype: TemporalPatternType) {
        self.pattern_type = ptype;
    }

    /// Get the time scale.
    pub fn get_time_scale(&self) -> &TimeScale {
        &self.time_scale
    }

    /// Set the time scale.
    pub fn set_time_scale(&mut self, scale: TimeScale) {
        self.time_scale = scale;
    }

    /// Get the periodicity, if any.
    pub fn get_periodicity(&self) -> Option<f64> {
        self.periodicity
    }

    /// Set the periodicity.
    pub fn set_periodicity(&mut self, periodicity: Option<f64>) {
        self.periodicity = periodicity;
    }
}

impl DataCompression {
    /// Creates compression state with default ratio/performance records.
    pub fn new() -> Self {
        Self {
            compression_algorithms: HashMap::new(),
            compression_ratio: CompressionRatio::new(),
            compression_performance: CompressionPerformance::new(),
        }
    }

    /// No-op returning `Ok(())` — in-memory compression state needs no setup.
    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        Ok(())
    }

    /// Register a compression algorithm under the given name.
    pub fn add_compression_algorithm(&mut self, name: &str, algo: CompressionAlgorithm) {
        self.compression_algorithms.insert(name.to_string(), algo);
    }

    /// Get a compression algorithm by name, if any.
    pub fn get_compression_algorithm(&self, name: &str) -> Option<&CompressionAlgorithm> {
        self.compression_algorithms.get(name)
    }

    /// List all registered compression algorithm names.
    pub fn list_compression_algorithms(&self) -> Vec<String> {
        self.compression_algorithms.keys().cloned().collect()
    }

    /// Get a reference to the compression ratio.
    pub fn get_compression_ratio(&self) -> &CompressionRatio {
        &self.compression_ratio
    }

    /// Get a mutable reference to the compression ratio.
    pub fn get_compression_ratio_mut(&mut self) -> &mut CompressionRatio {
        &mut self.compression_ratio
    }

    /// Get a reference to the compression performance.
    pub fn get_compression_performance(&self) -> &CompressionPerformance {
        &self.compression_performance
    }

    /// Get a mutable reference to the compression performance.
    pub fn get_compression_performance_mut(&mut self) -> &mut CompressionPerformance {
        &mut self.compression_performance
    }
}

impl CompressionRatio {
    /// Creates a zeroed `CompressionRatio`.
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            ratio: 1.0,
        }
    }
}

impl CompressionPerformance {
    /// Creates a zeroed `CompressionPerformance`.
    pub fn new() -> Self {
        Self {
            compression_speed: 0.0,
            decompression_speed: 0.0,
            memory_usage: 0,
        }
    }
}

impl CompressionAlgorithm {
    /// Creates a default `CompressionAlgorithm`.
    pub fn new() -> Self {
        Self {
            algorithm_id: "default".to_string(),
            algorithm_type: CompressionAlgorithmType::Lossless,
            parameters: CompressionParameters::new(),
        }
    }

    /// Get the algorithm ID.
    pub fn get_algorithm_id(&self) -> &str {
        &self.algorithm_id
    }

    /// Get the algorithm type.
    pub fn get_algorithm_type(&self) -> &CompressionAlgorithmType {
        &self.algorithm_type
    }

    /// Set the algorithm type.
    pub fn set_algorithm_type(&mut self, atype: CompressionAlgorithmType) {
        self.algorithm_type = atype;
    }

    /// Get a reference to the compression parameters.
    pub fn get_parameters(&self) -> &CompressionParameters {
        &self.parameters
    }

    /// Get a mutable reference to the compression parameters.
    pub fn get_parameters_mut(&mut self) -> &mut CompressionParameters {
        &mut self.parameters
    }
}

impl CompressionParameters {
    /// Creates default `CompressionParameters`.
    pub fn new() -> Self {
        Self {
            compression_level: 6,
            block_size: 1024,
            window_size: None,
            quality: None,
        }
    }
}

impl DataCache {
    /// Creates a cache with default policy and performance state.
    pub fn new() -> Self {
        Self {
            cache_policy: CachePolicy::new(),
            cache_size: 1024 * 1024 * 1024, // 1GB
            cache_performance: CachePerformance::new(),
        }
    }

    /// No-op returning `Ok(())`.
    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        Ok(())
    }

    /// Get a reference to the cache policy.
    pub fn get_cache_policy(&self) -> &CachePolicy {
        &self.cache_policy
    }

    /// Get a mutable reference to the cache policy.
    pub fn get_cache_policy_mut(&mut self) -> &mut CachePolicy {
        &mut self.cache_policy
    }

    /// Get the cache size in bytes.
    pub fn get_cache_size(&self) -> u64 {
        self.cache_size
    }

    /// Set the cache size in bytes.
    pub fn set_cache_size(&mut self, size: u64) {
        self.cache_size = size;
    }

    /// Get a reference to the cache performance.
    pub fn get_cache_performance(&self) -> &CachePerformance {
        &self.cache_performance
    }

    /// Get a mutable reference to the cache performance.
    pub fn get_cache_performance_mut(&mut self) -> &mut CachePerformance {
        &mut self.cache_performance
    }
}

impl CachePolicy {
    /// Creates a default `CachePolicy`.
    pub fn new() -> Self {
        Self {
            eviction_policy: EvictionPolicy::LRU,
            write_policy: WritePolicy::WriteThrough,
            consistency_policy: CacheConsistencyPolicy::Eventual,
        }
    }

    /// Get the eviction policy.
    pub fn get_eviction_policy(&self) -> &EvictionPolicy {
        &self.eviction_policy
    }

    /// Set the eviction policy.
    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.eviction_policy = policy;
    }

    /// Get the write policy.
    pub fn get_write_policy(&self) -> &WritePolicy {
        &self.write_policy
    }

    /// Set the write policy.
    pub fn set_write_policy(&mut self, policy: WritePolicy) {
        self.write_policy = policy;
    }

    /// Get the consistency policy.
    pub fn get_consistency_policy(&self) -> &CacheConsistencyPolicy {
        &self.consistency_policy
    }

    /// Set the consistency policy.
    pub fn set_consistency_policy(&mut self, policy: CacheConsistencyPolicy) {
        self.consistency_policy = policy;
    }
}

impl CachePerformance {
    /// Creates a zeroed `CachePerformance`.
    pub fn new() -> Self {
        Self {
            hit_rate: 0.0,
            miss_rate: 0.0,
            average_access_time: 0.0,
        }
    }
}
