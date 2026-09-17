//! Cooperative cancellation state for long-running local decode jobs.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

#[derive(Clone, Debug)]
pub struct DecodeControl {
    cancelled: Arc<AtomicBool>,
    max_tokens: Arc<AtomicU32>,
}

impl Default for DecodeControl {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            max_tokens: Arc::new(AtomicU32::new(u32::MAX)),
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
}
