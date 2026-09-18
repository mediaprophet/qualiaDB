//! Qwen dual-state checkpointing (Work Package F11).
//!
//! Synchronizes attention KV cache pages and GatedDeltaNet recurrent/convolution
//! states at exact committed token boundaries. Enforces fail-closed cache resumption:
//! both attention KV and recurrent states must agree before allowing prefix reuse.

use sha2::{Digest, Sha256};

/// Checkpoint of both Attention KV state and GatedDeltaNet recurrent state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QwenDualCheckpoint {
    /// Committed token sequence length.
    pub token_index: u32,
    /// Hash or root identifier of the paged KV cache tables.
    pub kv_page_hash: u64,
    /// Cryptographic digest over the active convolution and recurrent state tensors.
    pub recurrent_digest: [u8; 32],
}

impl QwenDualCheckpoint {
    /// Create a new dual-state checkpoint at a token boundary.
    pub fn new(
        token_index: u32,
        kv_page_hash: u64,
        conv_state: &[f32],
        recurrent_state: &[f32],
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(token_index.to_le_bytes());
        hasher.update(kv_page_hash.to_le_bytes());

        // Cast f32 slices into byte representations
        for &val in conv_state {
            hasher.update(val.to_le_bytes());
        }
        for &val in recurrent_state {
            hasher.update(val.to_le_bytes());
        }

        let mut recurrent_digest = [0u8; 32];
        recurrent_digest.copy_from_slice(&hasher.finalize());

        Self {
            token_index,
            kv_page_hash,
            recurrent_digest,
        }
    }

    /// Verify whether a candidate execution environment matches this checkpoint.
    ///
    /// Returns `true` only if:
    /// 1. Token count matches.
    /// 2. Paged KV cache hash matches.
    /// 3. Recurrent state digest matches exactly.
    pub fn can_resume(
        &self,
        candidate_tokens: u32,
        candidate_kv_hash: u64,
        candidate_recurrent_digest: &[u8; 32],
    ) -> bool {
        self.token_index == candidate_tokens
            && self.kv_page_hash == candidate_kv_hash
            && &self.recurrent_digest == candidate_recurrent_digest
    }
}

/// Registry of checkpoints for a multi-turn conversation or generation run.
#[derive(Debug, Default)]
pub struct QwenCheckpointRegistry {
    checkpoints: Vec<QwenDualCheckpoint>,
    max_checkpoints: usize,
}

impl QwenCheckpointRegistry {
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: Vec::with_capacity(max_checkpoints),
            max_checkpoints,
        }
    }

    /// Push a verified checkpoint.
    pub fn push(&mut self, checkpoint: QwenDualCheckpoint) {
        if self.checkpoints.len() >= self.max_checkpoints && !self.checkpoints.is_empty() {
            self.checkpoints.remove(0); // Bounded FIFO eviction
        }
        self.checkpoints.push(checkpoint);
    }

    /// Find the deepest checkpoint that satisfies prefix compatibility.
    pub fn find_compatible(
        &self,
        target_tokens: u32,
        kv_hash: u64,
        recurrent_digest: &[u8; 32],
    ) -> Option<&QwenDualCheckpoint> {
        self.checkpoints
            .iter()
            .rev()
            .find(|cp| cp.token_index <= target_tokens && cp.can_resume(cp.token_index, kv_hash, recurrent_digest))
    }

    /// Clear all registered checkpoints (e.g. on model swap or context reset).
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_checkpoint_verification() {
        let conv = [0.1f32, 0.2, 0.3];
        let rec = [1.0f32, 2.0, 3.0, 4.0];

        let cp = QwenDualCheckpoint::new(128, 0x1234_5678, &conv, &rec);
        assert_eq!(cp.token_index, 128);
        assert_eq!(cp.kv_page_hash, 0x1234_5678);

        // Exact match passes
        assert!(cp.can_resume(128, 0x1234_5678, &cp.recurrent_digest));

        // KV hash mismatch fails closed
        assert!(!cp.can_resume(128, 0x9999_9999, &cp.recurrent_digest));

        // Token count mismatch fails closed
        assert!(!cp.can_resume(127, 0x1234_5678, &cp.recurrent_digest));

        // Recurrent digest mismatch fails closed
        let dummy_digest = [0u8; 32];
        assert!(!cp.can_resume(128, 0x1234_5678, &dummy_digest));
    }

    #[test]
    fn test_checkpoint_registry_bounded_capacity() {
        let mut reg = QwenCheckpointRegistry::new(2);
        let cp1 = QwenDualCheckpoint::new(10, 1, &[0.0], &[0.0]);
        let cp2 = QwenDualCheckpoint::new(20, 2, &[0.0], &[0.0]);
        let cp3 = QwenDualCheckpoint::new(30, 3, &[0.0], &[0.0]);

        reg.push(cp1);
        reg.push(cp2);
        reg.push(cp3);

        assert_eq!(reg.checkpoints.len(), 2);
        assert_eq!(reg.checkpoints[0].token_index, 20);
        assert_eq!(reg.checkpoints[1].token_index, 30);
    }
}
