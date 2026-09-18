//! Model residency identity and generation tracking.
//!
//! Binds model execution plans and caches to a unique generation and content identity,
//! ensuring that model replacement or LoRA adapter changes invalidate stale plans.

use std::sync::atomic::{AtomicU64, Ordering};

/// Generation identity tracker for process-wide or engine-local model residency.
#[derive(Debug)]
pub struct ResidencyGenerationTracker {
    current_generation: AtomicU64,
}

impl ResidencyGenerationTracker {
    pub const fn new(initial_generation: u64) -> Self {
        Self {
            current_generation: AtomicU64::new(initial_generation),
        }
    }

    /// Read the current residency generation number.
    pub fn generation(&self) -> u64 {
        self.current_generation.load(Ordering::Acquire)
    }

    /// Bump generation, invalidating any previously compiled plans or cached states.
    pub fn advance_generation(&self) -> u64 {
        self.current_generation.fetch_add(1, Ordering::Release) + 1
    }
}

/// Content-bound model residency identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelResidencyIdentity {
    /// Canonical model path or DID.
    pub model_path: String,
    /// Content hash of the model weights/header (e.g. FNV-1a or SHA-256 of magic + metadata).
    pub content_hash: u64,
    /// Residency generation number assigned upon mounting/loading.
    pub generation: u64,
    /// Hash of currently attached LoRA adapter configurations (0 if none).
    pub adapter_hash: u64,
}

impl ModelResidencyIdentity {
    pub fn new(
        model_path: impl Into<String>,
        content_hash: u64,
        generation: u64,
        adapter_hash: u64,
    ) -> Self {
        Self {
            model_path: model_path.into(),
            content_hash,
            generation,
            adapter_hash,
        }
    }

    /// Returns true if `other` matches the exact generation, content, and adapter state.
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.content_hash == other.content_hash
            && self.adapter_hash == other.adapter_hash
            && self.model_path == other.model_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_tracker_advances() {
        let tracker = ResidencyGenerationTracker::new(1);
        assert_eq!(tracker.generation(), 1);
        let g2 = tracker.advance_generation();
        assert_eq!(g2, 2);
        assert_eq!(tracker.generation(), 2);
    }

    #[test]
    fn test_residency_identity_compatibility() {
        let id1 = ModelResidencyIdentity::new("models/qwen.gguf", 0x1234, 1, 0);
        let id1_clone = ModelResidencyIdentity::new("models/qwen.gguf", 0x1234, 1, 0);
        assert!(id1.is_compatible_with(&id1_clone));

        // Different generation (e.g. reload or swap) -> incompatible
        let id2 = ModelResidencyIdentity::new("models/qwen.gguf", 0x1234, 2, 0);
        assert!(!id1.is_compatible_with(&id2));

        // Different adapter -> incompatible
        let id3 = ModelResidencyIdentity::new("models/qwen.gguf", 0x1234, 1, 0xABCD);
        assert!(!id1.is_compatible_with(&id3));
    }
}

