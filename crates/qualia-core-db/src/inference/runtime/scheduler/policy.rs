//! Deterministic prefill and decode scheduling policies.
//!
//! Provides deterministic fairness rules, bounded prefill chunk sizing, and prefill-vs-decode
//! interleaving so long prefill operations cannot starve active decode requests.

/// Policy governing chunk limits and prefill-vs-decode interleaving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulingPolicy {
    /// Maximum tokens processed in a single prefill chunk.
    pub max_prefill_chunk_tokens: u32,
    /// Maximum consecutive prefill chunks permitted while decode requests are waiting.
    pub max_consecutive_prefill_chunks: u32,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl SchedulingPolicy {
    pub const DEFAULT: Self = Self {
        max_prefill_chunk_tokens: 512,
        max_consecutive_prefill_chunks: 2,
    };

    pub fn new(max_prefill_chunk_tokens: u32, max_consecutive_prefill_chunks: u32) -> Self {
        Self {
            max_prefill_chunk_tokens: max_prefill_chunk_tokens.max(1),
            max_consecutive_prefill_chunks: max_consecutive_prefill_chunks.max(1),
        }
    }

    /// Select chunk length given remaining prompt tokens.
    pub fn select_chunk_size(&self, remaining_tokens: u32) -> u32 {
        remaining_tokens.min(self.max_prefill_chunk_tokens)
    }

    /// Decide whether prefill must yield to waiting decode requests.
    pub fn should_yield_prefill_to_decode(
        &self,
        consecutive_prefill_count: u32,
        has_waiting_decodes: bool,
    ) -> bool {
        has_waiting_decodes && consecutive_prefill_count >= self.max_consecutive_prefill_chunks
    }
}

/// Tracking structure for request prefill progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefillProgress {
    pub total_input_tokens: u32,
    pub processed_tokens: u32,
}

impl PrefillProgress {
    pub fn new(total_input_tokens: u32) -> Self {
        Self {
            total_input_tokens,
            processed_tokens: 0,
        }
    }

    pub fn remaining_tokens(&self) -> u32 {
        self.total_input_tokens.saturating_sub(self.processed_tokens)
    }

    pub fn is_complete(&self) -> bool {
        self.processed_tokens >= self.total_input_tokens
    }

    pub fn advance(&mut self, chunk_tokens: u32) -> u32 {
        let actual = chunk_tokens.min(self.remaining_tokens());
        self.processed_tokens += actual;
        actual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_size_selection() {
        let policy = SchedulingPolicy::new(128, 2);
        assert_eq!(policy.select_chunk_size(500), 128);
        assert_eq!(policy.select_chunk_size(64), 64);
        assert_eq!(policy.select_chunk_size(0), 0);
    }

    #[test]
    fn test_prefill_yield_fairness() {
        let policy = SchedulingPolicy::new(128, 2);
        // Under threshold: should not yield even if decodes wait
        assert!(!policy.should_yield_prefill_to_decode(1, true));
        // At threshold with decodes waiting: must yield
        assert!(policy.should_yield_prefill_to_decode(2, true));
        // At threshold but NO decodes waiting: can continue
        assert!(!policy.should_yield_prefill_to_decode(2, false));
    }

    #[test]
    fn test_prefill_progress_advancement() {
        let mut progress = PrefillProgress::new(100);
        assert_eq!(progress.remaining_tokens(), 100);
        assert!(!progress.is_complete());

        let advanced = progress.advance(40);
        assert_eq!(advanced, 40);
        assert_eq!(progress.remaining_tokens(), 60);

        let advanced = progress.advance(70);
        assert_eq!(advanced, 60);
        assert_eq!(progress.remaining_tokens(), 0);
        assert!(progress.is_complete());
    }
}
