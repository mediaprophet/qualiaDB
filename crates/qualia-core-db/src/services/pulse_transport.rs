//! Pulse transport — process-wide broadcast channel for `pulse.publish` events.
//!
//! The 0.1 `pulse.publish(topic, payload)` binding routes through this module
//! when the Poet snapshot is `attached` to the live daemon graph. Subscribers
//! (e.g. an SSE endpoint, WebSocket relay, or collaborative sync layer) receive
//! a [`PulseEvent`] for every published pulse.
//!
//! Design mirrors [`crate::daemon_graph`]: a `OnceLock<broadcast::Sender>` so
//! the channel is lazily initialised on first use and shared across all
//! callers in the process. The channel capacity is bounded (64) to avoid
//! unbounded memory growth if no subscriber is draining.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

/// Maximum in-flight pulse events before backpressure drops to the slowest
/// subscriber. Matches the graph-revision channel capacity.
const PULSE_CHANNEL_CAPACITY: usize = 64;

/// A single pulse event emitted by `pulse.publish`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseEvent {
    /// The topic string (must match the 0.1 allowlist prefixes).
    pub topic: String,
    /// A compact textual summary of the payload value (for SSE / log display).
    pub payload_summary: String,
    /// Monotonic sequence number for this pulse (starts at 1).
    pub seq: u64,
    /// UNIX timestamp (seconds) at which the pulse was emitted.
    pub timestamp: u64,
}

static PULSE_TX: OnceLock<broadcast::Sender<PulseEvent>> = OnceLock::new();
static PULSE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn pulse_tx() -> &'static broadcast::Sender<PulseEvent> {
    PULSE_TX.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(PULSE_CHANNEL_CAPACITY);
        tx
    })
}

/// Publish a pulse event to all subscribers. Returns the sequence number
/// assigned to this event, or `0` if there are no subscribers (the event is
/// still recorded in the sequence counter for monotonicity).
///
/// This is the native transport path called by `PoetSnapshot::pulse_publish`
/// when `attached`. On WASM, the Poet host falls back to the in-memory
/// `published` vector.
pub fn publish(topic: &str, payload_summary: &str) -> u64 {
    let seq = PULSE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Release) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = PulseEvent {
        topic: topic.to_string(),
        payload_summary: payload_summary.to_string(),
        seq,
        timestamp,
    };
    let _ = pulse_tx().send(event);
    seq
}

/// Subscribe to the pulse event stream. Each subscriber gets its own
/// `broadcast::Receiver`; slow subscribers may miss events if they lag
/// behind by more than `PULSE_CHANNEL_CAPACITY`.
pub fn subscribe() -> broadcast::Receiver<PulseEvent> {
    pulse_tx().subscribe()
}

/// Current monotonic pulse sequence counter (for diagnostics / UI display).
pub fn pulse_seq() -> u64 {
    PULSE_SEQ.load(std::sync::atomic::Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_increments_seq() {
        let before = pulse_seq();
        let seq = publish("pulse/test-seq", "hello");
        assert!(seq > before, "seq {seq} should be > before {before}");
        // pulse_seq may have advanced further if another test ran in parallel;
        // we only assert monotonicity (seq >= pulse_seq is always true since
        // seq was just allocated from the same counter).
        assert!(pulse_seq() >= seq);
    }

    #[test]
    fn subscribe_receives_event() {
        let mut rx = subscribe();
        let seq = publish("pulse/test-recv", "payload-text");
        let event = rx.try_recv().expect("subscriber should receive event");
        assert_eq!(event.seq, seq);
        assert_eq!(event.topic, "pulse/test-recv");
        assert_eq!(event.payload_summary, "payload-text");
    }

    #[test]
    fn publish_without_subscriber_is_ok() {
        // Publishing with no subscriber must not panic (broadcast::send returns
        // Err, which we discard).
        let _ = publish("pulse/test-no-sub", "orphan");
    }
}
