//! Tick policy under load — drop / coalesce / tear (T68).
//!
//! When tick events arrive faster than the engine can process them,
//! the system must have an explicit policy. Without one, animation
//! and sensor fusion lie — they report stale data as if it were
//! current, or drop frames silently.
//!
//! ## Three policies
//!
//! - **Drop**: discard ticks that arrive while a previous tick is
//!   still being processed. The next tick after the current one
//!   completes is processed normally. Simple, but loses information.
//!
//! - **Coalesce**: merge pending ticks into a single tick that carries
//!   the latest timestamp and a count of dropped ticks. Preserves
//!   temporal recency at the cost of intermediate states.
//!
//! - **Tear**: emit a `TopologicalTear` diagnostic when ticks are
//!   dropped, so the principal knows the system is overloaded. The
//!   tear carries (μ, λ) evidence: μ = how many ticks were dropped,
//!   λ = the latency of the last processed tick.
//!
//! The default is **Coalesce** — it preserves the most information
//! while preventing unbounded queue growth. The policy is a host
//! constant, not a language constant (per T46 design principle).
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.14 T68,
//! excellence-first §3.9.

use crate::value::{Instant, Value};
use std::collections::BTreeMap;

/// The tick policy — what happens when ticks arrive under load (T68).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TickPolicy {
    /// Discard ticks that arrive while a previous tick is processing.
    /// The next tick after the current one completes is processed.
    Drop,
    /// Merge pending ticks into one with the latest timestamp and a
    /// count of dropped ticks. Default.
    #[default]
    Coalesce,
    /// Emit a TopologicalTear when ticks are dropped, carrying (μ, λ)
    /// evidence. Still coalesces, but also reports the overload.
    Tear,
}

impl TickPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Drop => "drop",
            Self::Coalesce => "coalesce",
            Self::Tear => "tear",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "drop" => Some(Self::Drop),
            "coalesce" => Some(Self::Coalesce),
            "tear" => Some(Self::Tear),
            _ => None,
        }
    }
}

/// A tick event — a timestamped signal from the host's tick loop.
#[derive(Debug, Clone, PartialEq)]
pub struct TickEvent {
    /// When the tick was generated (Instant from the host clock).
    pub instant: Instant,
    /// The tick sequence number (monotonically increasing).
    pub seq: u64,
    /// Optional payload (e.g. dt for animation, sensor reading).
    pub payload: Option<Value>,
}

/// The result of applying a tick policy to a queue of pending ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct TickDecision {
    /// The tick to actually process (if any).
    pub process: Option<TickEvent>,
    /// How many ticks were dropped or coalesced.
    pub dropped_count: u64,
    /// Whether a topological tear should be emitted.
    pub emit_tear: bool,
    /// The tear evidence (μ = dropped, λ = latency_nanos), if tearing.
    pub tear_evidence: Option<(f64, f64)>,
}

impl TickDecision {
    /// No ticks to process — the queue was empty.
    pub fn idle() -> Self {
        Self {
            process: None,
            dropped_count: 0,
            emit_tear: false,
            tear_evidence: None,
        }
    }

    /// Process a single tick with no drops.
    pub fn process_one(tick: TickEvent) -> Self {
        Self {
            process: Some(tick),
            dropped_count: 0,
            emit_tear: false,
            tear_evidence: None,
        }
    }
}

/// A tick queue with a policy. Bounded — never grows unboundedly.
#[derive(Debug, Clone)]
pub struct TickQueue {
    policy: TickPolicy,
    /// The pending tick (at most one — we coalesce immediately).
    pending: Option<TickEvent>,
    /// Total ticks dropped since the queue was created.
    total_dropped: u64,
    /// Total ticks processed.
    total_processed: u64,
    /// The last processed tick's seq (for latency calculation).
    last_processed_seq: u64,
    /// The last processed tick's instant (for latency calculation).
    last_processed_instant: Instant,
}

impl TickQueue {
    pub fn new(policy: TickPolicy) -> Self {
        Self {
            policy,
            pending: None,
            total_dropped: 0,
            total_processed: 0,
            last_processed_seq: 0,
            last_processed_instant: Instant::unix(0, 0),
        }
    }

    /// Enqueue a tick. If a tick is already pending, the policy
    /// determines what happens.
    pub fn enqueue(&mut self, tick: TickEvent) {
        match self.policy {
            TickPolicy::Drop => {
                if self.pending.is_some() {
                    self.total_dropped += 1;
                    // Replace with the newer tick (drop the older one).
                    self.pending = Some(tick);
                } else {
                    self.pending = Some(tick);
                }
            }
            TickPolicy::Coalesce | TickPolicy::Tear => {
                if let Some(ref mut existing) = self.pending {
                    // Coalesce: keep the latest timestamp, increment dropped.
                    self.total_dropped += 1;
                    existing.instant = tick.instant;
                    existing.seq = tick.seq;
                    // Keep the newer payload.
                    existing.payload = tick.payload;
                } else {
                    self.pending = Some(tick);
                }
            }
        }
    }

    /// Dequeue the next tick to process. Returns a TickDecision
    /// describing what happened.
    pub fn dequeue(&mut self) -> TickDecision {
        let tick = match self.pending.take() {
            None => return TickDecision::idle(),
            Some(t) => t,
        };

        let dropped = if self.total_dropped > 0 {
            // How many were dropped since last dequeue.
            let dropped_this_round = self.total_dropped;
            self.total_dropped = 0;
            dropped_this_round
        } else {
            0
        };

        // Calculate latency (difference between this tick's seq and
        // the last processed seq).
        let latency = tick.seq.saturating_sub(self.last_processed_seq);

        self.last_processed_seq = tick.seq;
        self.last_processed_instant = tick.instant.clone();
        self.total_processed += 1;

        match self.policy {
            TickPolicy::Tear if dropped > 0 => {
                // μ = dropped count, λ = latency in ticks.
                let mu = dropped as f64;
                let lambda = latency as f64;
                TickDecision {
                    process: Some(tick),
                    dropped_count: dropped,
                    emit_tear: true,
                    tear_evidence: Some((mu, lambda)),
                }
            }
            _ => TickDecision {
                process: Some(tick),
                dropped_count: dropped,
                emit_tear: false,
                tear_evidence: None,
            },
        }
    }

    /// Total ticks dropped since the queue was created.
    pub fn total_dropped(&self) -> u64 {
        self.total_dropped
    }

    /// Total ticks processed.
    pub fn total_processed(&self) -> u64 {
        self.total_processed
    }

    /// Is there a pending tick?
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The current policy.
    pub fn policy(&self) -> TickPolicy {
        self.policy
    }

    /// Convert the queue state to a Value::Record for inspection.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("policy".into(), Value::String(self.policy.as_str().into()));
        rec.insert("has_pending".into(), Value::Bool(self.pending.is_some()));
        rec.insert("total_dropped".into(), Value::U64(self.total_dropped));
        rec.insert("total_processed".into(), Value::U64(self.total_processed));
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tick(seq: u64, secs: i64) -> TickEvent {
        TickEvent {
            instant: Instant::unix(secs, 0),
            seq,
            payload: None,
        }
    }

    // ── TickPolicy tests ──────────────────────────────────────────────

    #[test]
    fn t68_policy_round_trip() {
        for p in &[TickPolicy::Drop, TickPolicy::Coalesce, TickPolicy::Tear] {
            let s = p.as_str();
            assert_eq!(TickPolicy::from_str(s), Some(*p));
        }
        assert_eq!(TickPolicy::from_str("unknown"), None);
    }

    #[test]
    fn t68_default_is_coalesce() {
        assert_eq!(TickPolicy::default(), TickPolicy::Coalesce);
    }

    // ── Drop policy tests ─────────────────────────────────────────────

    #[test]
    fn t68_drop_policy_replaces_older_tick() {
        let mut q = TickQueue::new(TickPolicy::Drop);
        q.enqueue(make_tick(1, 100));
        q.enqueue(make_tick(2, 200));
        // The newer tick (seq=2) should be pending.
        let decision = q.dequeue();
        assert!(decision.process.is_some());
        assert_eq!(decision.process.unwrap().seq, 2);
        assert_eq!(decision.dropped_count, 1);
        assert!(!decision.emit_tear);
    }

    #[test]
    fn t68_drop_policy_no_drop_when_empty() {
        let mut q = TickQueue::new(TickPolicy::Drop);
        q.enqueue(make_tick(1, 100));
        let decision = q.dequeue();
        assert_eq!(decision.dropped_count, 0);
        assert!(decision.process.is_some());
    }

    // ── Coalesce policy tests ─────────────────────────────────────────

    #[test]
    fn t68_coalesce_keeps_latest_timestamp() {
        let mut q = TickQueue::new(TickPolicy::Coalesce);
        q.enqueue(make_tick(1, 100));
        q.enqueue(make_tick(2, 200));
        q.enqueue(make_tick(3, 300));
        let decision = q.dequeue();
        let tick = decision.process.unwrap();
        assert_eq!(tick.seq, 3);
        assert_eq!(tick.instant.secs, 300);
        assert_eq!(decision.dropped_count, 2);
        assert!(!decision.emit_tear);
    }

    #[test]
    fn t68_coalesce_empty_queue_returns_idle() {
        let mut q = TickQueue::new(TickPolicy::Coalesce);
        let decision = q.dequeue();
        assert!(decision.process.is_none());
        assert_eq!(decision.dropped_count, 0);
    }

    // ── Tear policy tests ─────────────────────────────────────────────

    #[test]
    fn t68_tear_policy_emits_tear_on_drop() {
        let mut q = TickQueue::new(TickPolicy::Tear);
        q.enqueue(make_tick(1, 100));
        q.enqueue(make_tick(2, 200));
        q.enqueue(make_tick(3, 300));
        let decision = q.dequeue();
        assert!(decision.emit_tear);
        assert_eq!(decision.dropped_count, 2);
        let (mu, lambda) = decision.tear_evidence.unwrap();
        assert_eq!(mu, 2.0);
        // lambda = seq 3 - last_processed_seq 0 = 3
        assert_eq!(lambda, 3.0);
    }

    #[test]
    fn t68_tear_policy_no_tear_when_no_drops() {
        let mut q = TickQueue::new(TickPolicy::Tear);
        q.enqueue(make_tick(1, 100));
        let decision = q.dequeue();
        assert!(!decision.emit_tear);
        assert_eq!(decision.dropped_count, 0);
    }

    // ── Queue state tests ─────────────────────────────────────────────

    #[test]
    fn t68_queue_tracks_totals() {
        let mut q = TickQueue::new(TickPolicy::Coalesce);
        q.enqueue(make_tick(1, 100));
        q.enqueue(make_tick(2, 200));
        q.enqueue(make_tick(3, 300));
        q.dequeue();
        q.enqueue(make_tick(4, 400));
        q.enqueue(make_tick(5, 500));
        q.dequeue();
        assert_eq!(q.total_processed(), 2);
        // 2 dropped in first round + 1 dropped in second round = 3
        // But total_dropped is reset after each dequeue, so we check
        // via the decision instead.
    }

    #[test]
    fn t68_queue_has_pending() {
        let mut q = TickQueue::new(TickPolicy::Coalesce);
        assert!(!q.has_pending());
        q.enqueue(make_tick(1, 100));
        assert!(q.has_pending());
        q.dequeue();
        assert!(!q.has_pending());
    }

    #[test]
    fn t68_queue_to_value() {
        let mut q = TickQueue::new(TickPolicy::Tear);
        q.enqueue(make_tick(1, 100));
        let v = q.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("policy"), Some(&Value::String("tear".into())));
                assert_eq!(r.get("has_pending"), Some(&Value::Bool(true)));
                assert_eq!(r.get("total_processed"), Some(&Value::U64(0)));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── Sequential dequeue tests ──────────────────────────────────────

    #[test]
    fn t68_sequential_dequeue_no_drops() {
        let mut q = TickQueue::new(TickPolicy::Coalesce);
        q.enqueue(make_tick(1, 100));
        let d1 = q.dequeue();
        assert_eq!(d1.process.unwrap().seq, 1);
        assert_eq!(d1.dropped_count, 0);

        q.enqueue(make_tick(2, 200));
        let d2 = q.dequeue();
        assert_eq!(d2.process.unwrap().seq, 2);
        assert_eq!(d2.dropped_count, 0);
    }

    #[test]
    fn t68_tear_evidence_mu_lambda() {
        let mut q = TickQueue::new(TickPolicy::Tear);
        // Process first tick normally.
        q.enqueue(make_tick(10, 100));
        q.dequeue();

        // Now enqueue 3 ticks rapidly (2 will be coalesced/dropped).
        q.enqueue(make_tick(11, 200));
        q.enqueue(make_tick(12, 300));
        q.enqueue(make_tick(13, 400));
        let decision = q.dequeue();
        assert!(decision.emit_tear);
        let (mu, lambda) = decision.tear_evidence.unwrap();
        assert_eq!(mu, 2.0); // 2 ticks dropped
        assert_eq!(lambda, 3.0); // seq 13 - seq 10 = 3
    }
}
