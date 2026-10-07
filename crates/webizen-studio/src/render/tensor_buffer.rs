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
///
/// The canonical engine wire format is `Q42*` + a 32-byte header + packed
/// 10×f32 records (40 bytes). The former Studio-only 10×f64 records (80 bytes)
/// remain readable for saved previews, but new engine buffers are decoded by
/// their header and never guessed from their total length.
pub(crate) const Q42_TENSOR_MAGIC: u32 = 0x5134_322A;
pub(crate) const Q42_TENSOR_VERSION: u16 = 1;
pub(crate) const Q42_TENSOR_HEADER_BYTES: usize = 32;
pub(crate) const Q42_TENSOR_STRIDE: usize = 40;
const LEGACY_TENSOR_STRIDE: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TensorWireFormat {
    Q42F32,
    LegacyF64,
}
#[derive(Debug, Clone, Copy)]
pub struct TensorBufferView<'a> {
    /// Raw byte buffer containing tensor data
    buffer: &'a [u8],
    /// Number of tensors in the buffer
    count: usize,
    /// Binary node index table (index -> byte offset relative to record area)
    index_table: &'a [u64],
    /// Byte offset of record zero and its encoding.
    data_offset: usize,
    format: TensorWireFormat,
}

impl<'a> TensorBufferView<'a> {
    /// Create a view from a byte buffer with an index table.
    ///
    /// Zero-heap consideration: No allocation, just creates a view struct
    /// Binary IPC: Index table enables O(1) node lookup without string parsing
    #[inline]
    pub fn new_with_index(buffer: &'a [u8], index_table: &'a [u64]) -> Self {
        Self::new_inner(buffer, index_table)
    }

    /// Create a view from a byte buffer without an index table.
    /// Reads the canonical Q42 f32 ABI and retains compatibility with legacy
    /// Studio f64 preview buffers.
    #[inline]
    pub fn new(buffer: &'a [u8]) -> Self {
        Self::new_inner(buffer, &[])
    }

    fn new_inner(buffer: &'a [u8], index_table: &'a [u64]) -> Self {
        let (format, data_offset, count) = if buffer.len() >= 4
            && u32::from_le_bytes(buffer[..4].try_into().unwrap()) == Q42_TENSOR_MAGIC
        {
            let valid = buffer.len() >= Q42_TENSOR_HEADER_BYTES
                && u16::from_le_bytes(buffer[4..6].try_into().unwrap()) == Q42_TENSOR_VERSION
                && u32::from_le_bytes(buffer[12..16].try_into().unwrap()) as usize
                    == Q42_TENSOR_STRIDE;
            if !valid {
                (TensorWireFormat::Q42F32, Q42_TENSOR_HEADER_BYTES, 0)
            } else {
                let count = u32::from_le_bytes(buffer[8..12].try_into().unwrap()) as usize;
                let required = count
                    .checked_mul(Q42_TENSOR_STRIDE)
                    .and_then(|bytes| Q42_TENSOR_HEADER_BYTES.checked_add(bytes));
                match required.filter(|required| *required <= buffer.len()) {
                    Some(_) => (TensorWireFormat::Q42F32, Q42_TENSOR_HEADER_BYTES, count),
                    None => (TensorWireFormat::Q42F32, Q42_TENSOR_HEADER_BYTES, 0),
                }
            }
        } else {
            (
                TensorWireFormat::LegacyF64,
                0,
                buffer.len() / LEGACY_TENSOR_STRIDE,
            )
        };
        Self {
            buffer,
            count,
            index_table,
            data_offset,
            format,
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
        let tensor_index = usize::try_from(index).ok()?;
        if tensor_index >= self.count || tensor_index >= self.index_table.len() {
            return None;
        }

        let Some(offset) = usize::try_from(self.index_table[tensor_index])
            .ok()
            .and_then(|offset| self.data_offset.checked_add(offset))
        else {
            return None;
        };
        self.get_by_offset(offset)
    }

    /// Get a tensor at a specific byte offset (internal method)
    ///
    /// Zero-heap consideration: Stack-allocated reconstruction, no heap
    #[inline]
    fn get_by_offset(&self, offset: usize) -> Option<Tensor10DView> {
        let stride = self.record_stride();
        let Some(end) = offset.checked_add(stride) else {
            return None;
        };
        if end > self.buffer.len() || offset < self.data_offset {
            return None;
        }
        let relative = offset.checked_sub(self.data_offset)?;
        let data_bytes = self.count.checked_mul(stride)?;
        if relative % stride != 0 || relative.checked_add(stride)? > data_bytes {
            return None;
        }

        let bytes = &self.buffer[offset..end];
        let mut values = [0.0; 10];
        match self.format {
            TensorWireFormat::Q42F32 => {
                for (i, value) in values.iter_mut().enumerate() {
                    let at = i * 4;
                    *value = f32::from_le_bytes(bytes[at..at + 4].try_into().ok()?) as f64;
                }
            }
            TensorWireFormat::LegacyF64 => {
                for (i, value) in values.iter_mut().enumerate() {
                    let at = i * 8;
                    *value = self.read_f64_le(&bytes[at..at + 8]);
                }
            }
        }
        Some(Tensor10DView::from_values(values))
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

        let offset = self
            .record_stride()
            .checked_mul(index)?
            .checked_add(self.data_offset)?;
        self.get_by_offset(offset)
    }

    #[inline]
    fn record_stride(&self) -> usize {
        match self.format {
            TensorWireFormat::Q42F32 => Q42_TENSOR_STRIDE,
            TensorWireFormat::LegacyF64 => LEGACY_TENSOR_STRIDE,
        }
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
    /// Zero-heap consideration: Returns Vec\<u64\> (heap-allocated, but this is
    /// a one-time construction cost. The actual runtime access is zero-heap.)
    /// Binary IPC: This table is sent once, then reused for O(1) lookups
    pub fn build_index_table(count: usize) -> Vec<u64> {
        (0..count)
            .map(|i| (i * LEGACY_TENSOR_STRIDE) as u64)
            .collect()
    }

    /// Build record-relative offsets for canonical Q42 `Tensor10D` buffers.
    pub fn build_q42_index_table(count: usize) -> Vec<u64> {
        (0..count).map(|i| (i * Q42_TENSOR_STRIDE) as u64).collect()
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
    /// Combined magnitude across all manifold axes (telemetry / culling helper).
    #[inline]
    pub fn manifold_energy(&self) -> f64 {
        let sum = self.q * self.q
            + self.v * self.v
            + self.w * self.w
            + self.x * self.x
            + self.y * self.y
            + self.z * self.z
            + self.t * self.t
            + self.alpha * self.alpha
            + self.mu * self.mu
            + self.sigma * self.sigma;
        sum.sqrt()
    }

    /// Return the canonical EMF spectral tuple `[α, μ, σ]`.
    #[inline]
    pub fn emf_spectrum(&self) -> [f64; 3] {
        [self.alpha, self.mu, self.sigma]
    }

    /// Human-visible convenience projection of σ. EMF `[α, μ, σ]` remains
    /// canonical; this display helper must never replace or clamp stored σ.
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

impl Tensor10DView {
    fn from_values(values: [f64; 10]) -> Self {
        Self {
            q: values[0],
            v: values[1],
            w: values[2],
            x: values[3],
            y: values[4],
            z: values[5],
            t: values[6],
            alpha: values[7],
            mu: values[8],
            sigma: values[9],
        }
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
    fn reads_canonical_q42_f32_buffer_and_emf_without_color_conversion() {
        let mut buffer = [0u8; Q42_TENSOR_HEADER_BYTES + Q42_TENSOR_STRIDE];
        buffer[0..4].copy_from_slice(&Q42_TENSOR_MAGIC.to_le_bytes());
        buffer[4..6].copy_from_slice(&Q42_TENSOR_VERSION.to_le_bytes());
        buffer[8..12].copy_from_slice(&1_u32.to_le_bytes());
        buffer[12..16].copy_from_slice(&(Q42_TENSOR_STRIDE as u32).to_le_bytes());
        let values = [0.25_f32, 2.0, 3.0, -1.0, 0.5, 4.0, 7.0, 0.8, 0.3, 1.75];
        for (i, value) in values.iter().enumerate() {
            let offset = Q42_TENSOR_HEADER_BYTES + i * 4;
            buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }

        let view = TensorBufferView::new(&buffer);
        assert_eq!(view.len(), 1);
        let tensor = view.get(0).unwrap();
        assert_eq!(tensor.q, 0.25);
        assert_eq!(tensor.x, -1.0);
        let emf = tensor.emf_spectrum();
        assert!((emf[0] - 0.8).abs() < 1e-6);
        assert!((emf[1] - 0.3).abs() < 1e-6);
        assert_eq!(emf[2], 1.75);
        assert_eq!(tensor.sigma, 1.75, "σ must not be clamped to display gamut");
    }

    #[test]
    fn q42_index_offsets_are_relative_to_record_area() {
        let mut buffer = [0u8; Q42_TENSOR_HEADER_BYTES + 2 * Q42_TENSOR_STRIDE];
        buffer[0..4].copy_from_slice(&Q42_TENSOR_MAGIC.to_le_bytes());
        buffer[4..6].copy_from_slice(&Q42_TENSOR_VERSION.to_le_bytes());
        buffer[8..12].copy_from_slice(&2_u32.to_le_bytes());
        buffer[12..16].copy_from_slice(&(Q42_TENSOR_STRIDE as u32).to_le_bytes());
        buffer[Q42_TENSOR_HEADER_BYTES + Q42_TENSOR_STRIDE + 36..]
            .copy_from_slice(&0.625_f32.to_le_bytes());
        let offsets = TensorBufferView::build_q42_index_table(2);
        let view = TensorBufferView::new_with_index(&buffer, &offsets);
        assert_eq!(view.get_by_index(1).unwrap().sigma, 0.625);
        assert!(view.get_by_index(u64::MAX).is_none());
    }

    #[test]
    fn rejects_truncated_canonical_q42_buffer() {
        let mut buffer = [0u8; Q42_TENSOR_HEADER_BYTES];
        buffer[0..4].copy_from_slice(&Q42_TENSOR_MAGIC.to_le_bytes());
        buffer[4..6].copy_from_slice(&Q42_TENSOR_VERSION.to_le_bytes());
        buffer[8..12].copy_from_slice(&1_u32.to_le_bytes());
        buffer[12..16].copy_from_slice(&(Q42_TENSOR_STRIDE as u32).to_le_bytes());
        assert!(TensorBufferView::new(&buffer).is_empty());
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
