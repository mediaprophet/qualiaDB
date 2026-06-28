//! QKVP: Q42 Runtime Page Profile for Semantic KV and Cache Reuse
//! 
//! This module defines the page-aware structures for the KV cache,
//! allowing the engine to intelligently reuse, quantize, or evict 
//! KV memory based on cognitive and thermodynamic context rather 
//! than simple chronological LRU.

pub const QKVP_MAGIC: [u8; 4] = *b"QKVP";
pub const QKVP_VERSION: u16 = 1;

/// The root header for a KV cache page.
/// 
/// Precisely padded to 256 bytes for optimal CPU cache line (DOD) performance.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct Q42KvPageHeader {
    pub magic: [u8; 4],             // 0..4 (b"QKVP")
    pub version: u16,               // 4..6
    pub flags: u16,                 // 6..8
    
    pub model_hash: [u8; 32],       // 8..40
    
    pub token_start: u64,           // 40..48
    pub token_count: u32,           // 48..52
    pub manifold_idx_start: u32,    // 52..56 (replaces layer_start)
    pub manifold_idx_count: u32,    // 56..60 (replaces layer_count)
    pub kv_heads: u16,              // 60..62
    pub head_dim: u16,              // 62..64
    
    // -- 64 byte boundary --
    pub kv_dtype: u16,              // 64..66
    pub quant_codec: u16,           // 66..68
    pub compression_flags: u32,     // 68..72
    pub payload_offset: u64,        // 72..80
    pub payload_length: u64,        // 80..88
    
    pub parent_page_id: u64,        // 88..96
    pub next_page_id: u64,          // 96..104
    pub semantic_index_off: u64,    // 104..112
    pub manifold_index_off: u64,    // 112..120
    pub sketch_offset: u64,         // 120..128
    
    // -- 128 byte boundary --
    pub entropy_score: f32,         // 128..132
    pub attention_score: f32,       // 132..136
    pub recency_score: f32,         // 136..140
    pub confidence_score: f32,      // 140..144
    
    pub _reserved1: [u8; 80],       // 144..224
    pub checksum: [u8; 32],         // 224..256
}

/// Controls semantic chunking, boundary weights, and thermal biases.
/// Padded to exactly 64 bytes.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct Q42ChunkPolicy {
    pub max_tokens: u32,                  // 0..4
    pub semantic_shift_threshold: f32,    // 4..8
    pub discourse_boundary_weight: f32,   // 8..12
    pub attention_phase_weight: f32,      // 12..16
    pub max_entropy_drop: f32,            // 16..20
    pub thermal_pressure_bias: f32,       // 20..24
    pub reserved: [u8; 40],               // 24..64
}

/// Drives query-aware min/max selection for the KV cache pages.
/// Padded to exactly 128 bytes.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct Q42QuerySketch {
    pub k_min_offset: u64,                // 0..8
    pub k_max_offset: u64,                // 8..16
    pub centroid_offset: u64,             // 16..24
    pub semantic_hash_hi: u64,            // 24..32
    pub semantic_hash_lo: u64,            // 32..40
    pub manifold_centroid: [f32; 10],     // 40..80
    pub reserved: [u8; 48],               // 80..128
}

impl Q42KvPageHeader {
    pub fn new() -> Self {
        Self {
            magic: QKVP_MAGIC,
            version: QKVP_VERSION,
            flags: 0,
            model_hash: [0; 32],
            token_start: 0,
            token_count: 0,
            manifold_idx_start: 0,
            manifold_idx_count: 0,
            kv_heads: 0,
            head_dim: 0,
            kv_dtype: 0,
            quant_codec: 0,
            compression_flags: 0,
            payload_offset: 0,
            payload_length: 0,
            parent_page_id: 0,
            next_page_id: 0,
            semantic_index_off: 0,
            manifold_index_off: 0,
            sketch_offset: 0,
            entropy_score: 0.0,
            attention_score: 0.0,
            recency_score: 0.0,
            confidence_score: 0.0,
            _reserved1: [0; 80],
            checksum: [0; 32],
        }
    }
}

impl Default for Q42KvPageHeader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qkvp_alignment() {
        assert_eq!(std::mem::size_of::<Q42KvPageHeader>(), 256);
        assert_eq!(std::mem::size_of::<Q42ChunkPolicy>(), 64);
        assert_eq!(std::mem::size_of::<Q42QuerySketch>(), 128);
    }
}
