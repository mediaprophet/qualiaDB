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

    /// Delegates to the inner `MessageStore` while enforcing buffer manager capacity bounds and tracking usage.
    pub fn store_message(&mut self, message: StoredMessage) -> Result<(), MeshError> {
        let payload_len = message.payload.len();
        let existing_len = self
            .message_store
            .stored_messages
            .get(&message.message_id)
            .map(|m| m.payload.len())
            .unwrap_or(0);
        let needed = payload_len.saturating_sub(existing_len);
        if self.buffer_manager.used_capacity + needed > self.buffer_manager.total_capacity {
            return Err(MeshError::StorageError(format!(
                "Buffer capacity exceeded: {} bytes required, {} available",
                needed,
                self.buffer_manager.total_capacity.saturating_sub(self.buffer_manager.used_capacity)
            )));
        }
        self.buffer_manager.used_capacity = self
            .buffer_manager
            .used_capacity
            .saturating_add(needed)
            .saturating_sub(existing_len.saturating_sub(payload_len));

        // Update buffer pools if allocated
        for pool in self.buffer_manager.buffer_pools.values_mut() {
            if pool.buffer_size > 0 {
                let allocated = (self.buffer_manager.used_capacity + pool.buffer_size - 1) / pool.buffer_size;
                pool.allocated_buffers = allocated.min(pool.pool_size);
                pool.available_buffers = pool.pool_size.saturating_sub(pool.allocated_buffers);
            }
        }

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

    /// Performs genuine buffer compaction: prunes messages that are expired, delivered, or failed,
    /// cleans associated indices, recalculates true used buffer capacity, and adjusts buffer pools.
    pub fn optimize_buffers(&mut self) -> Result<(), MeshError> {
        let now = Instant::now();
        let pruned_ids: Vec<String> = self
            .message_store
            .stored_messages
            .values()
            .filter(|m| {
                m.status == MessageStatus::Expired
                    || m.status == MessageStatus::Delivered
                    || m.status == MessageStatus::Failed
                    || (m.ttl > Duration::ZERO && now.duration_since(m.timestamp) > m.ttl)
            })
            .map(|m| m.message_id.clone())
            .collect();

        for id in &pruned_ids {
            self.message_store.stored_messages.remove(id);
        }

        // Clean indices
        for v in self.message_store.message_index.source_index.values_mut() {
            v.retain(|id| !pruned_ids.contains(id));
        }
        for v in self.message_store.message_index.destination_index.values_mut() {
            v.retain(|id| !pruned_ids.contains(id));
        }
        for v in self.message_store.message_index.priority_index.values_mut() {
            v.retain(|id| !pruned_ids.contains(id));
        }
        self.message_store
            .message_index
            .timestamp_index
            .retain(|(_, id)| !pruned_ids.contains(id));

        // Recalculate true used capacity from active messages
        let active_bytes: usize = self
            .message_store
            .stored_messages
            .values()
            .map(|m| m.payload.len())
            .sum();
        self.buffer_manager.used_capacity = active_bytes;

        // Update buffer pool availability
        for pool in self.buffer_manager.buffer_pools.values_mut() {
            if pool.buffer_size > 0 {
                let allocated = (active_bytes + pool.buffer_size - 1) / pool.buffer_size;
                pool.allocated_buffers = allocated.min(pool.pool_size);
                pool.available_buffers = pool.pool_size.saturating_sub(pool.allocated_buffers);
            }
        }

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

    /// Inserts/updates `message` keyed by `message_id`, maintaining indices.
    pub fn store_message(&mut self, message: StoredMessage) -> Result<(), MeshError> {
        let msg_id = message.message_id.clone();
        let source = message.source.clone();
        let destination = message.destination.clone();
        let priority = message.priority;
        let timestamp = message.timestamp;

        self.stored_messages.insert(msg_id.clone(), message);

        self.message_index
            .source_index
            .entry(source)
            .or_default()
            .push(msg_id.clone());
        self.message_index
            .destination_index
            .entry(destination)
            .or_default()
            .push(msg_id.clone());
        self.message_index
            .priority_index
            .entry(priority)
            .or_default()
            .push(msg_id.clone());
        self.message_index
            .timestamp_index
            .push((timestamp, msg_id));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_message_and_optimize_buffers_compaction() {
        let mut store = MeshDataStore::new();
        // Set a small capacity for testing
        store.buffer_manager_mut().total_capacity = 100;

        let msg1 = StoredMessage {
            message_id: "msg_1".to_string(),
            source: "node_a".to_string(),
            destination: "node_b".to_string(),
            payload: vec![1u8; 40],
            priority: MessagePriority::Normal,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(60),
            delivery_attempts: 1,
            status: MessageStatus::Pending,
        };
        store.store_message(msg1).unwrap();
        assert_eq!(store.buffer_manager().used_capacity, 40);

        let msg2 = StoredMessage {
            message_id: "msg_2".to_string(),
            source: "node_a".to_string(),
            destination: "node_c".to_string(),
            payload: vec![2u8; 50],
            priority: MessagePriority::High,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(60),
            delivery_attempts: 1,
            status: MessageStatus::Delivered, // Delivered -> should be pruned on compaction
        };
        store.store_message(msg2).unwrap();
        assert_eq!(store.buffer_manager().used_capacity, 90);

        // Attempting to store another 20 bytes should exceed 100 bytes capacity
        let msg3 = StoredMessage {
            message_id: "msg_3".to_string(),
            source: "node_b".to_string(),
            destination: "node_c".to_string(),
            payload: vec![3u8; 20],
            priority: MessagePriority::Low,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(60),
            delivery_attempts: 0,
            status: MessageStatus::Pending,
        };
        assert!(store.store_message(msg3.clone()).is_err());

        // Optimize buffers - msg_2 (Delivered) should be pruned
        store.optimize_buffers().unwrap();
        // Remaining active message is msg_1 with 40 bytes
        assert_eq!(store.buffer_manager().used_capacity, 40);

        // Now msg3 can be stored successfully!
        assert!(store.store_message(msg3).is_ok());
        assert_eq!(store.buffer_manager().used_capacity, 60);
    }
}
