//! Universal inference lifecycle stages (Model-native and Qualia-augmented).
//!
//! Stage kinds mirror `core-ontologies/inference-lifecycle.n3` (`stage:*`
//! individuals). Recording is fixed-capacity and allocation-free: a
//! [`StageTrace`] holds at most [`STAGE_TRACE_CAPACITY`] receipts, coalescing
//! repeat visits of the same stage kind by saturating-accumulating
//! `duration_us` and `auxiliary_code`, so per-token stages (SSM state steps,
//! MoE expert dispatches) record one receipt per kind per turn.

use serde::{Deserialize, Serialize};

/// Lifecycle stage kind. Discriminants match the ordering in
/// `inference-lifecycle.n3` §3 (Qualia stages then model-native stages).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceStageKind {
    ChatGraphGrounding = 0x01,
    OntologyRouting = 0x02,
    DeonticPreFlight = 0x03,
    ConditioningBudgeting = 0x04,
    SensoryProjection = 0x05,
    ChunkedPrefill = 0x06,
    RecurrentStateUpdate = 0x07,
    MoEExpertDispatch = 0x08,
    ThinkingDeliberation = 0x09,
    ParaconsistentIsolation = 0x0A,
    ToolCallInvocation = 0x0B,
    AutoregressiveDecode = 0x0C,
    SentinelMidDecodeGuard = 0x0D,
    QuinCommitment = 0x0E,
    OutcomeEvaluation = 0x0F,
}

/// Terminal disposition of a stage within one inference turn.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    Executed = 0,
    Bypassed = 1,
    Quarantined = 2,
    RolledBack = 3,
    Failed = 4,
}

/// Fixed-size per-stage timing and outcome record. `auxiliary_code` carries
/// stage-specific counts (e.g. routed expert count, SSM state steps, or
/// deliberation tokens produced).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTimingReceipt {
    pub stage: InferenceStageKind,
    pub status: StageStatus,
    pub duration_us: u32,
    pub auxiliary_code: u32,
}

/// Number of stage receipts an [`StageTrace`] (and an
/// [`super::receipt::ExecutionReceipt`]) can hold. There are 15 stage kinds, so
/// a per-kind-coalesced trace never overflows.
pub const STAGE_TRACE_CAPACITY: usize = 16;

/// Caller-buffered, zero-heap recorder for one inference turn's stage timing.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageTrace {
    pub receipts: [Option<StageTimingReceipt>; STAGE_TRACE_CAPACITY],
    pub len: u8,
}

impl Default for StageTrace {
    fn default() -> Self {
        Self::new()
    }
}

impl StageTrace {
    pub const fn new() -> Self {
        Self {
            receipts: [None; STAGE_TRACE_CAPACITY],
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Record a stage event. Repeat visits of the same stage kind coalesce:
    /// `status` is replaced by the latest observation while `duration_us` and
    /// `auxiliary_code` saturating-accumulate. Returns `false` only when the
    /// trace is full (unreachable for the 15 defined kinds).
    pub fn record(
        &mut self,
        stage: InferenceStageKind,
        status: StageStatus,
        duration_us: u32,
        auxiliary_code: u32,
    ) -> bool {
        for slot in self.receipts[..self.len as usize].iter_mut() {
            if let Some(receipt) = slot {
                if receipt.stage == stage {
                    receipt.status = status;
                    receipt.duration_us = receipt.duration_us.saturating_add(duration_us);
                    receipt.auxiliary_code =
                        receipt.auxiliary_code.saturating_add(auxiliary_code);
                    return true;
                }
            }
        }
        if (self.len as usize) >= STAGE_TRACE_CAPACITY {
            return false;
        }
        self.receipts[self.len as usize] = Some(StageTimingReceipt {
            stage,
            status,
            duration_us,
            auxiliary_code,
        });
        self.len += 1;
        true
    }

    /// Iterate the recorded receipts in first-seen stage order.
    pub fn iter(&self) -> impl Iterator<Item = StageTimingReceipt> + '_ {
        self.receipts[..self.len as usize]
            .iter()
            .flatten()
            .copied()
    }

    /// Fetch the coalesced receipt for a stage kind, if recorded.
    pub fn get(&self, stage: InferenceStageKind) -> Option<StageTimingReceipt> {
        self.iter().find(|r| r.stage == stage)
    }
}

// ── Process-global last-turn trace ──────────────────────────────────────────
// Mirrors the `llm_bench` counter conventions: the engine thread drains its
// trace into here at end-of-turn so cold-path consumers (receipts, UI,
// benchmarks) can attach stage timings without changing the infer tuple ABI.

static LAST_STAGE_TRACE: std::sync::Mutex<StageTrace> = std::sync::Mutex::new(StageTrace::new());

/// Publish a completed per-turn stage trace.
pub fn set_last_stage_trace(trace: StageTrace) {
    let mut guard = LAST_STAGE_TRACE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = trace;
}

/// Read the most recently published stage trace (empty before any decode).
pub fn last_stage_trace() -> StageTrace {
    *LAST_STAGE_TRACE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Read and clear the most recently published stage trace.
pub fn take_last_stage_trace() -> StageTrace {
    let mut guard = LAST_STAGE_TRACE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::mem::take(&mut *guard)
}

// ── Thinking-trace delimiter detection ──────────────────────────────────────
// Streaming `<think>` / `</think>` matcher over decoded token bytes. Stack
// state only; tags may straddle token boundaries.

pub const THINK_OPEN_TAG: &[u8] = b"<think>";
pub const THINK_CLOSE_TAG: &[u8] = b"</think>";

/// Event returned by [`ThinkingTracker::feed_token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingEvent {
    /// No delimiter boundary crossed by this token.
    None,
    /// This token completed a `<think>` open tag.
    Entered,
    /// This token completed a `</think>` close tag.
    Exited,
}

/// Zero-heap streaming detector for thinking/deliberation regions in decode
/// output, with a token count for budget enforcement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThinkingTracker {
    open_progress: u8,
    close_progress: u8,
    in_thinking: bool,
    thinking_tokens: u32,
}

impl ThinkingTracker {
    pub const fn new() -> Self {
        Self {
            open_progress: 0,
            close_progress: 0,
            in_thinking: false,
            thinking_tokens: 0,
        }
    }

    pub fn in_thinking(&self) -> bool {
        self.in_thinking
    }

    /// Tokens emitted strictly inside `<think>…</think>` (delimiter tokens
    /// themselves are not counted).
    pub fn thinking_tokens(&self) -> u32 {
        self.thinking_tokens
    }

    /// Feed the decoded bytes of one emitted token. Delimiters may span
    /// multiple tokens; partial-match progress is retained between calls.
    pub fn feed_token(&mut self, token_bytes: &[u8]) -> ThinkingEvent {
        let mut event = ThinkingEvent::None;
        for &byte in token_bytes {
            let (tag, in_region) = if self.in_thinking {
                (THINK_CLOSE_TAG, true)
            } else {
                (THINK_OPEN_TAG, false)
            };
            let progress = if in_region {
                &mut self.close_progress
            } else {
                &mut self.open_progress
            };
            if byte == tag[*progress as usize] {
                *progress += 1;
                if *progress as usize == tag.len() {
                    *progress = 0;
                    self.in_thinking = !self.in_thinking;
                    event = if in_region {
                        ThinkingEvent::Exited
                    } else {
                        ThinkingEvent::Entered
                    };
                }
            } else {
                *progress = u8::from(byte == tag[0]);
            }
        }
        // Delimiter-carrying tokens are not themselves deliberation content.
        if self.in_thinking && event != ThinkingEvent::Entered {
            self.thinking_tokens = self.thinking_tokens.saturating_add(1);
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_kind_discriminants_match_ontology_order() {
        assert_eq!(InferenceStageKind::ChatGraphGrounding as u8, 0x01);
        assert_eq!(InferenceStageKind::OutcomeEvaluation as u8, 0x0F);
        assert_eq!(InferenceStageKind::SentinelMidDecodeGuard as u8, 0x0D);
        assert_eq!(StageStatus::Executed as u8, 0);
        assert_eq!(StageStatus::RolledBack as u8, 3);
    }

    #[test]
    fn trace_records_and_coalesces_repeated_stages() {
        let mut trace = StageTrace::new();
        assert!(trace.record(
            InferenceStageKind::RecurrentStateUpdate,
            StageStatus::Executed,
            40,
            3,
        ));
        // Same kind again: coalesce, accumulating counts.
        assert!(trace.record(
            InferenceStageKind::RecurrentStateUpdate,
            StageStatus::Executed,
            30,
            3,
        ));
        assert_eq!(trace.len, 1);
        let receipt = trace
            .get(InferenceStageKind::RecurrentStateUpdate)
            .unwrap();
        assert_eq!(receipt.duration_us, 70);
        assert_eq!(receipt.auxiliary_code, 6);
        assert_eq!(receipt.status, StageStatus::Executed);

        assert!(trace.record(
            InferenceStageKind::SentinelMidDecodeGuard,
            StageStatus::RolledBack,
            5,
            1,
        ));
        assert_eq!(trace.len, 2);
        assert_eq!(trace.iter().count(), 2);
    }

    #[test]
    fn receipt_is_fixed_size_and_copy() {
        // u8 stage + u8 status + u32 duration + u32 aux, repr(C) aligned → 12 B.
        assert_eq!(std::mem::size_of::<StageTimingReceipt>(), 12);
        let receipt = StageTimingReceipt {
            stage: InferenceStageKind::MoEExpertDispatch,
            status: StageStatus::Executed,
            duration_us: 12,
            auxiliary_code: 8,
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let decoded: StageTimingReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn global_trace_publishes_and_takes() {
        let mut trace = StageTrace::new();
        trace.record(
            InferenceStageKind::AutoregressiveDecode,
            StageStatus::Executed,
            100,
            64,
        );
        set_last_stage_trace(trace);
        let seen = take_last_stage_trace();
        assert_eq!(seen.len, 1);
        assert!(take_last_stage_trace().is_empty());
    }

    #[test]
    fn thinking_tracker_detects_delimiters_across_tokens() {
        let mut tracker = ThinkingTracker::new();
        assert_eq!(tracker.feed_token(b"answer <thi"), ThinkingEvent::None);
        assert!(!tracker.in_thinking());
        assert_eq!(tracker.feed_token(b"nk> let me"), ThinkingEvent::Entered);
        assert!(tracker.in_thinking());
        assert_eq!(tracker.thinking_tokens(), 0);
        // Token inside the region counts.
        assert_eq!(tracker.feed_token(b" reason"), ThinkingEvent::None);
        assert_eq!(tracker.thinking_tokens(), 1);
        assert_eq!(tracker.feed_token(b" more"), ThinkingEvent::None);
        assert_eq!(tracker.thinking_tokens(), 2);
        // Close tag may also straddle a token boundary.
        assert_eq!(tracker.feed_token(b" </th"), ThinkingEvent::None);
        assert_eq!(tracker.thinking_tokens(), 3);
        assert_eq!(tracker.feed_token(b"ink> final"), ThinkingEvent::Exited);
        assert!(!tracker.in_thinking());
        assert_eq!(tracker.thinking_tokens(), 3);
    }
}
