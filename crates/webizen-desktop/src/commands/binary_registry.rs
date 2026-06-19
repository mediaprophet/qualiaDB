/// Binary Node Registry for Zero-Heap IPC
///
/// Maps string node IDs to u64 binary index pointers to avoid heap allocation
/// during cross-process serialization in Tauri IPC.
///
/// Zero-heap consideration: The registry uses HashMap<String, u64> which is
/// heap-allocated, but this is a one-time registration cost. The actual IPC
/// transfers use only u64 indices (stack-allocated), avoiding repeated heap
/// allocation during rendering loops.
///
/// Binary IPC Optimization: Instead of sending "heart_ventricle_left" (heap string)
/// across the process boundary, we send u64::MAX (stack-allocated index pointer).
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Binary node registry mapping string IDs to u64 index pointers
#[derive(Clone)]
pub struct BinaryNodeRegistry {
    /// String ID -> u64 index mapping (heap-allocated, one-time cost)
    id_to_index: Arc<Mutex<HashMap<String, u64>>>,
    /// u64 index -> String ID reverse mapping (heap-allocated, one-time cost)
    index_to_id: Arc<Mutex<HashMap<u64, String>>>,
    /// Next available index (stack-allocated)
    next_index: Arc<Mutex<u64>>,
}

impl BinaryNodeRegistry {
    /// Create a new binary node registry
    ///
    /// Zero-heap consideration: Arc<Mutex<>> is heap-allocated but necessary
    /// for thread-safe shared access across Tauri commands
    pub fn new() -> Self {
        Self {
            id_to_index: Arc::new(Mutex::new(HashMap::new())),
            index_to_id: Arc::new(Mutex::new(HashMap::new())),
            next_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Register a node ID and return its binary index
    ///
    /// Zero-heap consideration: String ID is heap-allocated but this is a
    /// one-time registration cost. The returned u64 is stack-allocated.
    ///
    /// Binary IPC: Use the returned u64 for all subsequent IPC transfers
    pub fn register(&self, node_id: &str) -> u64 {
        let mut id_map = self.id_to_index.lock().unwrap();

        // Return existing index if already registered
        if let Some(&index) = id_map.get(node_id) {
            return index;
        }

        // Allocate new index
        let mut next = self.next_index.lock().unwrap();
        let index = *next;
        *next = next.wrapping_add(1);

        // Store mapping
        id_map.insert(node_id.to_string(), index);

        let mut index_map = self.index_to_id.lock().unwrap();
        index_map.insert(index, node_id.to_string());

        index
    }

    /// Get binary index for a string ID
    ///
    /// Zero-heap consideration: Returns stack-allocated u64
    pub fn get_index(&self, node_id: &str) -> Option<u64> {
        let id_map = self.id_to_index.lock().unwrap();
        id_map.get(node_id).copied()
    }

    /// Get string ID for a binary index
    ///
    /// Zero-heap consideration: Returns heap-allocated String, but this
    /// should only be used for debugging or when string display is required
    pub fn get_id(&self, index: u64) -> Option<String> {
        let index_map = self.index_to_id.lock().unwrap();
        index_map.get(&index).cloned()
    }

    /// Get the number of registered nodes
    pub fn len(&self) -> usize {
        let id_map = self.id_to_index.lock().unwrap();
        id_map.len()
    }

    /// Clear all registrations (useful for testing or scene changes)
    pub fn clear(&self) {
        let mut id_map = self.id_to_index.lock().unwrap();
        let mut index_map = self.index_to_id.lock().unwrap();
        let mut next = self.next_index.lock().unwrap();

        id_map.clear();
        index_map.clear();
        *next = 0;
    }
}

impl Default for BinaryNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_node_registry() {
        let registry = BinaryNodeRegistry::new();

        // Register nodes
        let idx1 = registry.register("node_1");
        let idx2 = registry.register("node_2");
        let idx3 = registry.register("node_1"); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Should return same index for duplicate

        assert_eq!(registry.len(), 2);

        // Test lookups
        assert_eq!(registry.get_index("node_1"), Some(0));
        assert_eq!(registry.get_index("node_2"), Some(1));
        assert_eq!(registry.get_index("node_3"), None);

        assert_eq!(registry.get_id(0), Some("node_1".to_string()));
        assert_eq!(registry.get_id(1), Some("node_2".to_string()));
        assert_eq!(registry.get_id(2), None);
    }

    #[test]
    fn test_registry_clear() {
        let registry = BinaryNodeRegistry::new();

        registry.register("node_1");
        registry.register("node_2");
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);

        // Should allocate new indices after clear
        let idx = registry.register("node_3");
        assert_eq!(idx, 0);
    }
}
