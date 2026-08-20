//! Deterministic replay clock — the only WASM clock (W12, T71).
//!
//! In a WASM environment, there is no wall clock — `SystemTime::now()`
//! is either unavailable or non-deterministic. The only honest clock
//! is a **replay Instant**: an Instant from a recorded timeline that
//! is fed to the engine deterministically.
//!
//! This module provides:
//! - `ReplayClock`: a deterministic clock that yields Instants from a
//!   recorded timeline. Each call to `tick()` advances to the next
//!   recorded Instant.
//! - `ReplayTimeline`: a recorded sequence of Instants with optional
//!   payloads (for replaying sensor data, animation frames, etc.).
//! - Integration with the Host trait: a WASM host uses `ReplayClock`
//!   as its `time_now` implementation.
//!
//! ## One clock story (T71)
//!
//! This is part of the "one clock story" — replacing the scattered
//! coarse Unix clocks (`time.unix()`, `asserted_time_unix: u32` in
//! wellfare-core, etc.) with a single Instant-based clock. The replay
//! clock is the WASM edge of that story: where native hosts use
//! `SystemTime::now()`, WASM hosts use `ReplayClock::tick()`.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.14 T71,
//! §8.15 W12, excellence-first §3.14, §4.

use crate::value::{Instant, TimeScale, Value};
use std::collections::BTreeMap;

/// A recorded timeline of Instants with optional payloads (W12).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayTimeline {
    /// The timeline's name/identifier.
    pub name: String,
    /// The recorded Instants in order.
    pub instants: Vec<Instant>,
    /// Optional payloads for each Instant (e.g. sensor readings).
    pub payloads: Vec<Option<Value>>,
    /// The time scale of this timeline.
    pub scale: TimeScale,
}

impl ReplayTimeline {
    /// Create a new empty timeline with the given name and scale.
    pub fn new(name: &str, scale: TimeScale) -> Self {
        Self {
            name: name.into(),
            instants: Vec::new(),
            payloads: Vec::new(),
            scale,
        }
    }

    /// Create a timeline from a sequence of (secs, nanos) pairs.
    pub fn from_secs_nanos(name: &str, scale: TimeScale, pts: &[(i64, u32)]) -> Self {
        let mut tl = Self::new(name, scale.clone());
        for &(secs, nanos) in pts {
            tl.push(
                Instant {
                    scale: scale.clone(),
                    secs,
                    nanos,
                    frame: None,
                    seal: None,
                },
                None,
            );
        }
        tl
    }

    /// Append an Instant with an optional payload.
    pub fn push(&mut self, instant: Instant, payload: Option<Value>) -> &mut Self {
        self.instants.push(instant);
        self.payloads.push(payload);
        self
    }

    /// Number of recorded Instants.
    pub fn len(&self) -> usize {
        self.instants.len()
    }

    /// Is the timeline empty?
    pub fn is_empty(&self) -> bool {
        self.instants.is_empty()
    }

    /// Get the Instant at the given index.
    pub fn get(&self, idx: usize) -> Option<&Instant> {
        self.instants.get(idx)
    }

    /// Get the payload at the given index.
    pub fn payload(&self, idx: usize) -> Option<&Value> {
        self.payloads.get(idx).and_then(|p| p.as_ref())
    }
}

/// A deterministic replay clock — the only WASM clock (W12).
///
/// Yields Instants from a recorded timeline. Each call to `tick()`
/// advances to the next recorded Instant. When the timeline is
/// exhausted, the clock returns the last Instant (or fails closed
/// if the timeline was empty).
#[derive(Debug, Clone)]
pub struct ReplayClock {
    timeline: ReplayTimeline,
    cursor: usize,
    /// What to do when the timeline is exhausted.
    exhausted_policy: ExhaustedPolicy,
    /// The last Instant returned (for repeat-after-exhaustion).
    last_instant: Option<Instant>,
}

/// What to do when the replay timeline is exhausted (W12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExhaustedPolicy {
    /// Return the last Instant indefinitely (replay loops on the last frame).
    #[default]
    RepeatLast,
    /// Fail closed with E702 (no more time data).
    FailClosed,
    /// Loop back to the beginning of the timeline.
    Loop,
}

impl ReplayClock {
    /// Create a replay clock from a timeline.
    pub fn new(timeline: ReplayTimeline) -> Self {
        Self {
            timeline,
            cursor: 0,
            exhausted_policy: ExhaustedPolicy::default(),
            last_instant: None,
        }
    }

    /// Set the exhausted policy.
    pub fn with_exhausted_policy(mut self, policy: ExhaustedPolicy) -> Self {
        self.exhausted_policy = policy;
        self
    }

    /// Advance to the next Instant in the timeline.
    /// Returns the next Instant, or an error string if exhausted and
    /// FailClosed.
    pub fn tick(&mut self) -> Result<Instant, String> {
        if self.cursor < self.timeline.len() {
            let instant = self.timeline.instants[self.cursor].clone();
            self.last_instant = Some(instant.clone());
            self.cursor += 1;
            Ok(instant)
        } else {
            match self.exhausted_policy {
                ExhaustedPolicy::RepeatLast => self
                    .last_instant
                    .clone()
                    .ok_or_else(|| "empty timeline".into()),
                ExhaustedPolicy::FailClosed => {
                    Err("replay timeline exhausted (fail-closed)".into())
                }
                ExhaustedPolicy::Loop => {
                    if self.timeline.is_empty() {
                        return Err("empty timeline".into());
                    }
                    self.cursor = 0;
                    let instant = self.timeline.instants[0].clone();
                    self.last_instant = Some(instant.clone());
                    self.cursor = 1;
                    Ok(instant)
                }
            }
        }
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Total timeline length.
    pub fn len(&self) -> usize {
        self.timeline.len()
    }

    /// Is the clock at the end of the timeline?
    pub fn is_exhausted(&self) -> bool {
        self.cursor >= self.timeline.len()
    }

    /// Get the payload at the current cursor position (before tick).
    pub fn current_payload(&self) -> Option<&Value> {
        if self.cursor < self.timeline.len() {
            self.timeline
                .payloads
                .get(self.cursor)
                .and_then(|p| p.as_ref())
        } else {
            None
        }
    }

    /// Convert the clock state to a Value::Record for inspection.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.timeline.name.clone()));
        rec.insert("cursor".into(), Value::U64(self.cursor as u64));
        rec.insert("length".into(), Value::U64(self.timeline.len() as u64));
        rec.insert("exhausted".into(), Value::Bool(self.is_exhausted()));
        rec.insert(
            "exhausted_policy".into(),
            Value::String(
                match self.exhausted_policy {
                    ExhaustedPolicy::RepeatLast => "repeat_last",
                    ExhaustedPolicy::FailClosed => "fail_closed",
                    ExhaustedPolicy::Loop => "loop",
                }
                .into(),
            ),
        );
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_timeline() -> ReplayTimeline {
        ReplayTimeline::from_secs_nanos(
            "test",
            TimeScale::Unix,
            &[
                (1000, 0),
                (1000, 500_000_000),
                (1001, 0),
                (1001, 500_000_000),
            ],
        )
    }

    // ── ReplayTimeline tests ──────────────────────────────────────────

    #[test]
    fn w12_timeline_basic() {
        let tl = make_timeline();
        assert_eq!(tl.name, "test");
        assert_eq!(tl.len(), 4);
        assert!(!tl.is_empty());
    }

    #[test]
    fn w12_timeline_get() {
        let tl = make_timeline();
        let i0 = tl.get(0).unwrap();
        assert_eq!(i0.secs, 1000);
        assert_eq!(i0.nanos, 0);
        let i1 = tl.get(1).unwrap();
        assert_eq!(i1.nanos, 500_000_000);
    }

    #[test]
    fn w12_timeline_empty() {
        let tl = ReplayTimeline::new("empty", TimeScale::Unix);
        assert!(tl.is_empty());
        assert_eq!(tl.len(), 0);
    }

    #[test]
    fn w12_timeline_with_payloads() {
        let mut tl = ReplayTimeline::new("sensors", TimeScale::Monotonic);
        tl.push(Instant::monotonic(1000), Some(Value::F64(42.0)));
        tl.push(Instant::monotonic(2000), None);
        assert_eq!(tl.len(), 2);
        assert_eq!(tl.payload(0), Some(&Value::F64(42.0)));
        assert_eq!(tl.payload(1), None);
    }

    // ── ReplayClock tests ─────────────────────────────────────────────

    #[test]
    fn w12_clock_ticks_through_timeline() {
        let mut clock = ReplayClock::new(make_timeline());
        let i0 = clock.tick().unwrap();
        assert_eq!(i0.secs, 1000);
        assert_eq!(i0.nanos, 0);
        let i1 = clock.tick().unwrap();
        assert_eq!(i1.nanos, 500_000_000);
        let i2 = clock.tick().unwrap();
        assert_eq!(i2.secs, 1001);
        assert_eq!(clock.cursor(), 3);
    }

    #[test]
    fn w12_clock_exhausted_repeat_last() {
        let mut clock = ReplayClock::new(make_timeline());
        // Tick through all 4.
        for _ in 0..4 {
            clock.tick().unwrap();
        }
        assert!(clock.is_exhausted());
        // Default policy: repeat last.
        let i = clock.tick().unwrap();
        assert_eq!(i.secs, 1001);
        assert_eq!(i.nanos, 500_000_000);
    }

    #[test]
    fn w12_clock_exhausted_fail_closed() {
        let mut clock =
            ReplayClock::new(make_timeline()).with_exhausted_policy(ExhaustedPolicy::FailClosed);
        for _ in 0..4 {
            clock.tick().unwrap();
        }
        let result = clock.tick();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fail-closed"));
    }

    #[test]
    fn w12_clock_exhausted_loop() {
        let mut clock =
            ReplayClock::new(make_timeline()).with_exhausted_policy(ExhaustedPolicy::Loop);
        for _ in 0..4 {
            clock.tick().unwrap();
        }
        assert!(clock.is_exhausted());
        // Loop back to beginning.
        let i = clock.tick().unwrap();
        assert_eq!(i.secs, 1000);
        assert_eq!(i.nanos, 0);
        assert_eq!(clock.cursor(), 1);
    }

    #[test]
    fn w12_clock_empty_timeline_fails() {
        let mut clock = ReplayClock::new(ReplayTimeline::new("empty", TimeScale::Unix));
        let result = clock.tick();
        assert!(result.is_err());
    }

    #[test]
    fn w12_clock_current_payload() {
        let mut tl = ReplayTimeline::new("sensors", TimeScale::Monotonic);
        tl.push(Instant::monotonic(1000), Some(Value::F64(42.0)));
        tl.push(Instant::monotonic(2000), Some(Value::F64(43.0)));
        let mut clock = ReplayClock::new(tl);
        // Before tick, payload at cursor 0.
        assert_eq!(clock.current_payload(), Some(&Value::F64(42.0)));
        clock.tick().unwrap();
        // After tick, payload at cursor 1.
        assert_eq!(clock.current_payload(), Some(&Value::F64(43.0)));
    }

    #[test]
    fn w12_clock_to_value() {
        let mut clock = ReplayClock::new(make_timeline());
        clock.tick().unwrap();
        let v = clock.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("test".into())));
                assert_eq!(r.get("cursor"), Some(&Value::U64(1)));
                assert_eq!(r.get("length"), Some(&Value::U64(4)));
                assert_eq!(r.get("exhausted"), Some(&Value::Bool(false)));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn w12_exhausted_policy_default() {
        assert_eq!(ExhaustedPolicy::default(), ExhaustedPolicy::RepeatLast);
    }

    #[test]
    fn w12_clock_loop_empty_timeline_fails() {
        let mut clock = ReplayClock::new(ReplayTimeline::new("empty", TimeScale::Unix))
            .with_exhausted_policy(ExhaustedPolicy::Loop);
        let result = clock.tick();
        assert!(result.is_err());
    }
}
