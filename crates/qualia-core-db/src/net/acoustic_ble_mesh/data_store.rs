//! Delay-tolerant mesh data store: stored-message store and index, buffer
//! management, priority queueing, and persistence configuration.

use super::*;

/// Mesh data store for delay-tolerant networking
pub struct MeshDataStore {
    message_store: MessageStore,
    buffer_manager: BufferManager,
    priority_queue: PriorityQueue,
    persistence_manager: PersistenceManager,
}

/// Message store
#[derive(Debug, Clone)]
pub struct MessageStore {
    pub stored_messages: HashMap<String, StoredMessage>,
    pub message_index: MessageIndex,
}

/// Stored message
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub message_id: String,
    pub source: String,
    pub destination: String,
    pub payload: Vec<u8>,
    pub priority: MessagePriority,
    pub timestamp: Instant,
    pub ttl: Duration,
    pub delivery_attempts: u32,
    pub status: MessageStatus,
}

/// Message index
#[derive(Debug, Clone)]
pub struct MessageIndex {
    pub source_index: HashMap<String, Vec<String>>,
    pub destination_index: HashMap<String, Vec<String>>,
    pub priority_index: HashMap<MessagePriority, Vec<String>>,
    pub timestamp_index: Vec<(Instant, String)>,
}

/// Buffer manager
#[derive(Debug, Clone)]
pub struct BufferManager {
    pub total_capacity: usize,
    pub used_capacity: usize,
    pub buffer_pools: HashMap<String, BufferPool>,
}

/// Buffer pool
#[derive(Debug, Clone)]
pub struct BufferPool {
    pub pool_size: usize,
    pub buffer_size: usize,
    pub available_buffers: usize,
    pub allocated_buffers: usize,
}

/// Priority queue
#[derive(Debug, Clone)]
pub struct PriorityQueue {
    pub queues: HashMap<MessagePriority, Vec<String>>,
    pub current_priority: MessagePriority,
}

/// Persistence manager
#[derive(Debug, Clone)]
pub struct PersistenceManager {
    pub storage_backend: StorageBackend,
    pub compression: CompressionType,
    pub encryption: bool,
}

/// Storage backends
#[derive(Debug, Clone, PartialEq)]
pub enum StorageBackend {
    Memory,
    File,
    Database,
    Distributed,
}

/// Compression types
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
    Custom,
}

impl MeshDataStore {
    /// Creates a store with fresh message store, buffer manager, priority queue, and persistence manager.
    pub fn new() -> Self {
        Self {
            message_store: MessageStore::new(),
            buffer_manager: BufferManager::new(),
            priority_queue: PriorityQueue::new(),
            persistence_manager: PersistenceManager::new(),
        }
    }

    /// No-op returning `Ok(())` — the in-memory store needs no setup.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        // Initialize data store
        Ok(())
    }

    /// Delegates to the inner `MessageStore`; `MeshError` propagates.
    pub fn store_message(&mut self, message: StoredMessage) -> Result<(), MeshError> {
        self.message_store.store_message(message)?;
        Ok(())
    }

    /// Count of messages held in the store.
    pub fn get_pending_message_count(&self) -> u32 {
        self.message_store.get_pending_count()
    }

    /// Shared access to the buffer manager.
    pub fn buffer_manager(&self) -> &BufferManager {
        &self.buffer_manager
    }

    /// Mutable access to the buffer manager.
    pub fn buffer_manager_mut(&mut self) -> &mut BufferManager {
        &mut self.buffer_manager
    }

    /// Shared access to the priority queue.
    pub fn priority_queue(&self) -> &PriorityQueue {
        &self.priority_queue
    }

    /// Shared access to the persistence manager.
    pub fn persistence_manager(&self) -> &PersistenceManager {
        &self.persistence_manager
    }

    /// **Scaffold** — sets `used_capacity` to 0 labeled "compaction" without actually freeing or compacting buffers; desyncs accounting if real buffers back the counter.
    pub fn optimize_buffers(&mut self) -> Result<(), MeshError> {
        // Reset buffer usage to zero (compaction).
        self.buffer_manager.used_capacity = 0;
        Ok(())
    }
}

impl MessageStore {
    /// Creates an empty store (HashMap keyed by message ID).
    pub fn new() -> Self {
        Self {
            stored_messages: HashMap::new(),
            message_index: MessageIndex::new(),
        }
    }

    /// Inserts/updates `message` keyed by `message_id`.
    pub fn store_message(&mut self, message: StoredMessage) -> Result<(), MeshError> {
        self.stored_messages
            .insert(message.message_id.clone(), message);
        Ok(())
    }

    /// Number of stored messages.
    pub fn get_pending_count(&self) -> u32 {
        self.stored_messages
            .values()
            .filter(|m| m.status == MessageStatus::Pending)
            .count() as u32
    }
}

impl MessageIndex {
    /// Creates an empty message index.
    pub fn new() -> Self {
        Self {
            source_index: HashMap::new(),
            destination_index: HashMap::new(),
            priority_index: HashMap::new(),
            timestamp_index: Vec::new(),
        }
    }
}

impl BufferManager {
    /// Creates a buffer manager with zeroed usage.
    pub fn new() -> Self {
        Self {
            total_capacity: 10 * 1024 * 1024, // 10MB
            used_capacity: 0,
            buffer_pools: HashMap::new(),
        }
    }
}

impl BufferPool {
    /// Creates an empty buffer pool.
    pub fn new() -> Self {
        Self {
            pool_size: 100,
            buffer_size: 1024,
            available_buffers: 100,
            allocated_buffers: 0,
        }
    }
}

impl PriorityQueue {
    /// Creates an empty priority queue.
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
            current_priority: MessagePriority::Normal,
        }
    }
}

impl PersistenceManager {
    /// Creates a persistence manager (in-memory scaffolding — no durable backend wired).
    pub fn new() -> Self {
        Self {
            storage_backend: StorageBackend::Memory,
            compression: CompressionType::None,
            encryption: false,
        }
    }
}
