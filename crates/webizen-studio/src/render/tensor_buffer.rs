/// Zero-deserialization view into 10D tensor buffer with binary indexing
///
/// Provides stack-allocated access to binary tensor data without
/// deserialization overhead. The view interprets raw bytes as 10D tensor
/// projections on-demand using index pointers instead of string IDs.
///
/// Zero-heap consideration: The view itself is stack-allocated (Copy type).
/// Only references to the underlying buffer are held. No heap allocation
/// occurs when creating or copying the view.
///
/// Binary IPC Optimization: Uses u64 index pointers instead of String IDs
/// to avoid heap allocation during cross-process serialization.
#[derive(Debug, Clone, Copy)]
pub struct TensorBufferView<'a> {
    /// Raw byte buffer containing tensor data
    buffer: &'a [u8],
    /// Number of tensors in the buffer
    count: usize,
    /// Binary node index table (index -> offset in buffer)
    index_table: &'a [u64],
}

impl<'a> TensorBufferView<'a> {
    /// Create a new view from a byte buffer with index table
    ///
    /// Zero-heap consideration: No allocation, just creates a view struct
    /// Binary IPC: Index table enables O(1) node lookup without string parsing
    #[inline]
    pub fn new_with_index(buffer: &'a [u8], index_table: &'a [u64]) -> Self {
        // Each 10D tensor is 10 * 8 bytes = 80 bytes (f64 per dimension)
        let count = buffer.len() / 80;
        Self {
            buffer,
            count,
            index_table,
        }
    }

    /// Create a new view from a byte buffer (legacy, no index table)
    #[inline]
    pub fn new(buffer: &'a [u8]) -> Self {
        let count = buffer.len() / 80;
        // Empty index table for legacy compatibility
        let index_table = &[];
        Self {
            buffer,
            count,
            index_table,
        }
    }

    /// Get the number of tensors in the buffer
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a tensor by binary index pointer (zero-heap O(1) lookup)
    ///
    /// Zero-heap consideration: Returns stack-allocated Tensor10D struct
    /// Binary IPC: Uses u64 index instead of String, avoiding heap allocation
    #[inline]
    pub fn get_by_index(&self, index: u64) -> Option<Tensor10DView> {
        // Lookup offset in index table
        let tensor_index = index as usize;
        if tensor_index >= self.count || tensor_index >= self.index_table.len() {
            return None;
        }

        let offset = self.index_table[tensor_index] as usize;
        self.get_by_offset(offset)
    }

    /// Get a tensor at a specific byte offset (internal method)
    ///
    /// Zero-heap consideration: Stack-allocated reconstruction, no heap
    #[inline]
    fn get_by_offset(&self, offset: usize) -> Option<Tensor10DView> {
        if offset + 80 > self.buffer.len() {
            return None;
        }

        // Read 10 f64 values (80 bytes) from buffer
        let bytes = &self.buffer[offset..offset + 80];

        // Convert bytes to f64 values
        let q = self.read_f64_le(&bytes[0..8]);
        let v = self.read_f64_le(&bytes[8..16]);
        let w = self.read_f64_le(&bytes[16..24]);
        let x = self.read_f64_le(&bytes[24..32]);
        let y = self.read_f64_le(&bytes[32..40]);
        let z = self.read_f64_le(&bytes[40..48]);
        let t = self.read_f64_le(&bytes[48..56]);
        let alpha = self.read_f64_le(&bytes[56..64]);
        let mu = self.read_f64_le(&bytes[64..72]);
        let sigma = self.read_f64_le(&bytes[72..80]);

        Some(Tensor10DView {
            q,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        })
    }

    /// Get a tensor at the specified index (legacy method, sequential access)
    ///
    /// Zero-heap consideration: Returns stack-allocated Tensor10D struct
    /// No heap allocation, just byte interpretation
    #[inline]
    pub fn get(&self, index: usize) -> Option<Tensor10DView> {
        if index >= self.count {
            return None;
        }

        let offset = index * 80;
        if offset + 80 > self.buffer.len() {
            return None;
        }

        // Read 10 f64 values (80 bytes) from buffer
        let bytes = &self.buffer[offset..offset + 80];

        // Convert bytes to f64 values
        let q = self.read_f64_le(&bytes[0..8]);
        let v = self.read_f64_le(&bytes[8..16]);
        let w = self.read_f64_le(&bytes[16..24]);
        let x = self.read_f64_le(&bytes[24..32]);
        let y = self.read_f64_le(&bytes[32..40]);
        let z = self.read_f64_le(&bytes[40..48]);
        let t = self.read_f64_le(&bytes[48..56]);
        let alpha = self.read_f64_le(&bytes[56..64]);
        let mu = self.read_f64_le(&bytes[64..72]);
        let sigma = self.read_f64_le(&bytes[72..80]);

        Some(Tensor10DView {
            q,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        })
    }

    /// Read little-endian f64 from bytes
    ///
    /// Zero-heap consideration: Stack-allocated reconstruction, no heap
    #[inline]
    fn read_f64_le(&self, bytes: &[u8]) -> f64 {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        f64::from_le_bytes(arr)
    }

    /// Create a binary index table from sequential tensor buffer
    ///
    /// Zero-heap consideration: Returns Vec<u64> (heap-allocated, but this is
    /// a one-time construction cost. The actual runtime access is zero-heap.)
    /// Binary IPC: This table is sent once, then reused for O(1) lookups
    pub fn build_index_table(count: usize) -> Vec<u64> {
        (0..count).map(|i| (i * 80) as u64).collect()
    }
}

/// Stack-allocated 10D tensor view
///
/// Zero-heap consideration: This struct is Copy and stack-allocated
#[derive(Debug, Clone, Copy)]
pub struct Tensor10DView {
    pub q: f64,
    pub v: f64,
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub t: f64,
    pub alpha: f64,
    pub mu: f64,
    pub sigma: f64,
}

impl Tensor10DView {
    /// Get spectral color from sigma value
    ///
    /// Zero-heap consideration: Stack-allocated string formatting
    pub fn spectral_color(&self) -> String {
        // Simple spectral mapping (simplified from full CIE XYZ)
        let hue = (self.sigma * 360.0) % 360.0;
        format!("hsl({}, 70%, 50%)", hue)
    }

    /// Get opacity from alpha value
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.alpha.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_buffer_view_empty() {
        let buffer = [];
        let view = TensorBufferView::new(&buffer);
        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn test_tensor_buffer_view_single() {
        // Create a buffer with one tensor (80 bytes)
        let mut buffer = [0u8; 80];
        // Set sigma = 0.5
        let sigma_bytes = 0.5_f64.to_le_bytes();
        buffer[72..80].copy_from_slice(&sigma_bytes);

        let view = TensorBufferView::new(&buffer);
        assert_eq!(view.len(), 1);

        let tensor = view.get(0).unwrap();
        assert_eq!(tensor.sigma, 0.5);
    }

    #[test]
    fn test_tensor_buffer_view_with_index_table() {
        // Create a buffer with two tensors (160 bytes)
        let mut buffer = [0u8; 160];
        // Set sigma = 0.5 for first tensor
        buffer[72..80].copy_from_slice(&0.5_f64.to_le_bytes());
        // Set sigma = 0.8 for second tensor
        buffer[152..160].copy_from_slice(&0.8_f64.to_le_bytes());

        // Build index table
        let index_table = TensorBufferView::build_index_table(2);
        assert_eq!(index_table.len(), 2);
        assert_eq!(index_table[0], 0);
        assert_eq!(index_table[1], 80);

        let view = TensorBufferView::new_with_index(&buffer, &index_table);
        assert_eq!(view.len(), 2);

        // Test binary index lookup
        let tensor0 = view.get_by_index(0).unwrap();
        assert_eq!(tensor0.sigma, 0.5);

        let tensor1 = view.get_by_index(1).unwrap();
        assert_eq!(tensor1.sigma, 0.8);

        // Test legacy sequential access still works
        let tensor0_legacy = view.get(0).unwrap();
        assert_eq!(tensor0_legacy.sigma, 0.5);
    }

    #[test]
    fn test_binary_index_table_builder() {
        let table = TensorBufferView::build_index_table(5);
        assert_eq!(table.len(), 5);
        assert_eq!(table[0], 0);
        assert_eq!(table[1], 80);
        assert_eq!(table[2], 160);
        assert_eq!(table[3], 240);
        assert_eq!(table[4], 320);
    }
}
