//! Cooperative cancellation state for long-running local decode jobs.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

#[derive(Clone, Debug)]
pub struct DecodeControl {
    cancelled: Arc<AtomicBool>,
    max_tokens: Arc<AtomicU32>,
    /// Reserved deliberation (`<think>`) token budget; 0 = no reservation.
    thinking_tokens: Arc<AtomicU32>,
}

impl Default for DecodeControl {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            max_tokens: Arc::new(AtomicU32::new(u32::MAX)),
            thinking_tokens: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl DecodeControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn set_token_budget(&self, max_tokens: u32) {
        self.max_tokens.store(max_tokens.max(1), Ordering::Release);
    }

    pub fn token_budget(&self) -> usize {
        self.max_tokens.load(Ordering::Acquire) as usize
    }

    /// Reserve a deliberation (`<think>`) token budget from the conditioning
    /// contract (`ConditioningBudget::thinking_token_budget`). 0 = unlimited /
    /// not a deliberation-configured decode.
    pub fn set_thinking_token_budget(&self, max_thinking_tokens: u32) {
        self.thinking_tokens
            .store(max_thinking_tokens, Ordering::Release);
    }

    /// Configured deliberation token budget in tokens; 0 = none.
    pub fn thinking_token_budget(&self) -> u32 {
        self.thinking_tokens.load(Ordering::Acquire)
    }
}
