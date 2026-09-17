//! Default resource budgets and work counters for coreference.

use super::error::CorefError;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 42 MiB Sentinel ceiling. This operation's workspace must stay under it
/// together with the borrowed source.
pub const SENTINEL_BYTES: usize = 42 * 1024 * 1024;

/// Default source cap (transport-sized, not a latency claim).
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 256 * 1024;
/// Default mention-list cap.
pub const DEFAULT_MAX_MENTIONS: usize = 4096;
/// Default aggregate supplied mention-text cap.
pub const DEFAULT_MAX_MENTION_TEXT_BYTES: usize = 1024 * 1024;
/// Default emitted-chain cap.
pub const DEFAULT_MAX_CHAINS: usize = 4096;
/// Proper-noun candidates inspected per pronoun (search window).
pub const DEFAULT_MAX_ANTECEDENTS_PER_PRONOUN: usize = 64;
/// Total Proper-noun antecedent inspections per call (non-Proper mentions
/// are skipped without charging this counter).
pub const DEFAULT_MAX_ANTECEDENT_CHECKS: u64 = 65_536;
/// Default caller workspace for this operation.
pub const DEFAULT_MAX_WORKSPACE_BYTES: usize = 2 * 1024 * 1024;
/// Default folded-key byte budget.
pub const DEFAULT_MAX_NORMALIZATION_BYTES: u64 = 1024 * 1024;
/// Default byte-comparison budget, including long shared prefixes.
pub const DEFAULT_MAX_COMPARISON_BYTES: u64 = 8 * 1024 * 1024;

/// FNV-1a 64-bit over already-normalized key bytes.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = OFFSET;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Caller-visible limits. Lower them in tests; raising defaults needs measured
/// resource evidence recorded in the NLP plan.
#[derive(Clone, Copy)]
pub struct CorefLimits {
    pub max_source_bytes: usize,
    pub max_mentions: usize,
    pub max_mention_text_bytes: usize,
    pub max_chains: usize,
    pub max_antecedents_per_pronoun: usize,
    pub max_antecedent_checks: u64,
    pub max_workspace_bytes: usize,
    pub max_normalization_bytes: u64,
    pub max_comparison_bytes: u64,
    /// Hash of folded keys. Production uses [`fnv1a64`]; tests may force collisions.
    pub key_hash: fn(&[u8]) -> u64,
}

impl CorefLimits {
    pub const DEFAULT: Self = Self {
        max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
        max_mentions: DEFAULT_MAX_MENTIONS,
        max_mention_text_bytes: DEFAULT_MAX_MENTION_TEXT_BYTES,
        max_chains: DEFAULT_MAX_CHAINS,
        max_antecedents_per_pronoun: DEFAULT_MAX_ANTECEDENTS_PER_PRONOUN,
        max_antecedent_checks: DEFAULT_MAX_ANTECEDENT_CHECKS,
        max_workspace_bytes: DEFAULT_MAX_WORKSPACE_BYTES,
        max_normalization_bytes: DEFAULT_MAX_NORMALIZATION_BYTES,
        max_comparison_bytes: DEFAULT_MAX_COMPARISON_BYTES,
        key_hash: fnv1a64,
    };

    pub fn source_too_large(self, bytes: usize) -> CorefError {
        CorefError::SourceTooLarge {
            bytes,
            max: self.max_source_bytes,
        }
    }
}

impl Default for CorefLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Cooperative cancellation. Polls are cheap; a zero `cancel_after_polls`
/// disables auto-cancel (the production default).
#[derive(Debug)]
pub struct CancellationToken {
    cancelled: AtomicBool,
    polls: AtomicU64,
    cancel_after_polls: u64,
}

impl CancellationToken {
    pub const fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            polls: AtomicU64::new(0),
            cancel_after_polls: 0,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Test helper: become cancelled after `n` polls (n >= 1).
    pub fn cancel_after_polls(n: u64) -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            polls: AtomicU64::new(0),
            cancel_after_polls: n,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        if self.cancelled.load(Ordering::Relaxed) {
            return true;
        }
        if self.cancel_after_polls == 0 {
            return false;
        }
        let polls = self.polls.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        if polls >= self.cancel_after_polls {
            self.cancelled.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Instrumented work. These are counters, not wall-clock claims.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CorefWorkStats {
    pub mention_count: u32,
    pub chain_count: u32,
    pub normalization_bytes: u64,
    pub comparison_bytes: u64,
    pub key_comparisons: u64,
    pub antecedent_candidates: u64,
    pub antecedent_checks: u64,
    pub workspace_bytes: usize,
}

/// Bytes for `n` mention work records plus two `u32` permutations plus `norm`.
/// Cold adapters also allocate `validated`/`heads`/`ids`; those are not part
/// of this hot workspace figure and stay well under the 42 MiB Sentinel.
pub fn workspace_bytes_required(mention_count: usize, norm_bytes: usize) -> usize {
    mention_count
        .saturating_mul(core::mem::size_of::<super::MentionWork>())
        .saturating_add(
            mention_count
                .saturating_mul(core::mem::size_of::<u32>())
                .saturating_mul(2),
        )
        .saturating_add(norm_bytes)
}
