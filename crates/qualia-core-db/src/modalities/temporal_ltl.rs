use crate::NQuin;

pub const OP_LTL_GLOBALLY: u8 = 0x40;
pub const OP_LTL_FINALLY: u8 = 0x41;
pub const OP_LTL_NEXT: u8 = 0x42;
pub const OP_LTL_UNTIL: u8 = 0x43;
pub const OP_LTL_RELEASE: u8 = 0x44;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtlFormula {
    Globally(u64),
    Finally(u64),
    Next(u64),
    Until { ante: u64, consequent: u64 },
    Release { trigger: u64, invariant: u64 },
}

#[derive(Debug)]
pub enum TemporalError {
    AbortedTimeout,
}

pub fn evaluate_lock_lease(
    lock_granted_at: u64,
    current_time: u64,
    ttl_seconds: u64,
) -> Result<(), TemporalError> {
    if current_time > lock_granted_at + ttl_seconds {
        return Err(TemporalError::AbortedTimeout);
    }
    Ok(())
}

pub fn evaluate_ltl_trace(trace: &[NQuin], formula: &LtlFormula) -> bool {
    match formula {
        LtlFormula::Globally(p) => {
            if trace.is_empty() {
                return false;
            }
            for quin in trace {
                if quin.predicate != *p {
                    return false;
                }
            }
            true
        }
        LtlFormula::Finally(p) => {
            if trace.is_empty() {
                return false;
            }
            for quin in trace {
                if quin.predicate == *p {
                    return true;
                }
            }
            false
        }
        LtlFormula::Next(p) => {
            if trace.len() < 2 {
                return false;
            }
            trace[1].predicate == *p
        }
        LtlFormula::Until { ante, consequent } => {
            if trace.is_empty() {
                return false;
            }
            for (i, quin) in trace.iter().enumerate() {
                if quin.predicate == *consequent {
                    let mut ante_held = true;
                    for j in 0..i {
                        if trace[j].predicate != *ante {
                            ante_held = false;
                            break;
                        }
                    }
                    if ante_held {
                        return true;
                    }
                }
            }
            false
        }
        LtlFormula::Release { trigger, invariant } => {
            if trace.is_empty() {
                return true;
            }
            for (i, quin) in trace.iter().enumerate() {
                if quin.predicate != *invariant {
                    let mut triggered = false;
                    for j in 0..=i {
                        if trace[j].predicate == *trigger {
                            triggered = true;
                            break;
                        }
                    }
                    if !triggered {
                        return false;
                    }
                }
            }
            true
        }
    }
}

/// Metric temporal (MTL) "within": after the EARLIEST occurrence of `trigger`
/// (each event quin carries its timestamp in `metadata`), `target` must occur at
/// some time `t1` with `t0 <= t1 <= t0 + window`. Models deadlines — e.g. "remedy
/// within 30 days of breach". Zero-heap (two linear scans; no allocation).
pub fn holds_within(trace: &[NQuin], trigger: u64, target: u64, window: u64) -> bool {
    // t0 = earliest timestamp at which the trigger holds.
    let mut t0: Option<u64> = None;
    for q in trace {
        if q.predicate == trigger {
            t0 = Some(match t0 {
                Some(prev) => prev.min(q.metadata),
                None => q.metadata,
            });
        }
    }
    let t0 = match t0 {
        Some(t) => t,
        None => return false, // trigger never occurred
    };
    let deadline = t0.saturating_add(window);
    for q in trace {
        if q.predicate == target && q.metadata >= t0 && q.metadata <= deadline {
            return true;
        }
    }
    false
}

/// MTL **bounded-globally** `G[t0, t0+window] inv`: from the earliest `trigger` timestamp `t0`,
/// the `invariant` must hold at *every* event whose timestamp lies in `[t0, t0+window]`. Returns
/// `true` vacuously if no in-window events. Zero-heap.
pub fn holds_globally_within(trace: &[NQuin], trigger: u64, invariant: u64, window: u64) -> bool {
    let mut t0: Option<u64> = None;
    for q in trace {
        if q.predicate == trigger {
            t0 = Some(t0.map_or(q.metadata, |p| p.min(q.metadata)));
        }
    }
    let t0 = match t0 {
        Some(t) => t,
        None => return false,
    };
    let deadline = t0.saturating_add(window);
    // Every event in the window must satisfy the invariant.
    for q in trace {
        if q.metadata >= t0 && q.metadata <= deadline && q.predicate != invariant && q.predicate != trigger {
            return false;
        }
    }
    true
}

// ─── Past-time LTL (Once, Historically, Since) ────────────────────────────────────

/// **Once** `O p` (past ◇): `p` held at some point in the trace (the past-time dual of `Finally`).
pub fn evaluate_once(trace: &[NQuin], p: u64) -> bool {
    trace.iter().any(|q| q.predicate == p)
}

/// **Historically** `H p` (past □): `p` held at *every* point in the trace (dual of `Globally`).
/// Vacuously true on an empty trace (nothing in the past violated it).
pub fn evaluate_historically(trace: &[NQuin], p: u64) -> bool {
    trace.iter().all(|q| q.predicate == p)
}

/// **Since** `p S q`: at the end of the trace, `q` held at some past point and `p` has held at
/// every point since (inclusive of after `q`). Retrospective legal analysis ("the duty has held
/// since the breach"). Zero-heap.
pub fn evaluate_since(trace: &[NQuin], p: u64, q: u64) -> bool {
    // Find the latest index where q held; require p at every index after it.
    let mut q_at: Option<usize> = None;
    for (i, ev) in trace.iter().enumerate() {
        if ev.predicate == q {
            q_at = Some(i);
        }
    }
    match q_at {
        None => false,
        Some(qi) => trace[qi + 1..].iter().all(|ev| ev.predicate == p),
    }
}

// ─── Büchi-style streaming monitor (continuous safety monitoring) ─────────────────

/// An incremental monitor for the safety fragment `G(p)` ("`p` always holds") — the practical
/// Büchi-automaton acceptance for stream monitoring: it stays `Safe` while every streamed event
/// satisfies `p`, and latches to `Violated` permanently on the first event that does not. Zero-heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyMonitor {
    pub invariant: u64,
    pub violated: bool,
}

impl SafetyMonitor {
    /// Start monitoring `G(invariant)`.
    pub fn new(invariant: u64) -> Self {
        Self { invariant, violated: false }
    }
    /// Feed the next streamed event predicate; returns `true` while still safe.
    pub fn step(&mut self, event_predicate: u64) -> bool {
        if event_predicate != self.invariant {
            self.violated = true;
        }
        !self.violated
    }
    /// Is the monitored safety property still holding?
    #[inline]
    pub fn is_safe(&self) -> bool {
        !self.violated
    }
}

// ─── Allen-interval bridge ────────────────────────────────────────────────────────

/// Extract the `[first, last]` timestamp interval over which `predicate` occurs in the timed
/// `trace` (each event's timestamp in `metadata`), or `None` if it never occurs. The bridge to
/// the Allen Interval Algebra (`interval_reasoning::allen_relation` relates two such intervals).
pub fn predicate_interval(trace: &[NQuin], predicate: u64) -> Option<(u64, u64)> {
    let mut lo: Option<u64> = None;
    let mut hi: Option<u64> = None;
    for q in trace {
        if q.predicate == predicate {
            lo = Some(lo.map_or(q.metadata, |v| v.min(q.metadata)));
            hi = Some(hi.map_or(q.metadata, |v| v.max(q.metadata)));
        }
    }
    Some((lo?, hi?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    fn timed(predicate: u64, t: u64) -> NQuin {
        NQuin { subject: 0, predicate, object: 0, context: 0, metadata: t, parity: 0 }
    }

    #[test]
    fn test_mtl_holds_within() {
        let breach = 1u64;
        let remedy = 2u64;
        // breach at t=10, remedy at t=25 → within window 30.
        let trace = [timed(breach, 10), timed(remedy, 25)];
        assert!(holds_within(&trace, breach, remedy, 30), "remedy 15 after breach ≤ 30 window");
        assert!(!holds_within(&trace, breach, remedy, 10), "remedy 15 after breach > 10 window");
        // remedy before the breach does not count.
        let late = [timed(remedy, 5), timed(breach, 10)];
        assert!(!holds_within(&late, breach, remedy, 30), "a remedy before the breach is not within");
        // no trigger → false.
        assert!(!holds_within(&[timed(remedy, 25)], breach, remedy, 30));
    }

    fn make_quin(predicate: u64) -> NQuin {
        NQuin {
            subject: 0,
            predicate,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn test_ltl_globally() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_p, q_p, q_p],
            &LtlFormula::Globally(p)
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_not_p, q_p],
            &LtlFormula::Globally(p)
        ));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Globally(p)));
    }

    #[test]
    fn test_ltl_finally() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_not_p, q_not_p, q_p],
            &LtlFormula::Finally(p)
        ));
        assert!(!evaluate_ltl_trace(
            &[q_not_p, q_not_p],
            &LtlFormula::Finally(p)
        ));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Finally(p)));
    }

    #[test]
    fn test_ltl_next() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(&[q_not_p, q_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[q_p, q_not_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[q_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Next(p)));
    }

    #[test]
    fn test_ltl_until() {
        let p = 100;
        let q = 200;
        let q_p = make_quin(p);
        let q_q = make_quin(q);
        let q_other = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_p, q_p, q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(evaluate_ltl_trace(
            &[q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_p, q_p],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_other, q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
    }

    #[test]
    fn test_ltl_release() {
        let trigger = 100;
        let invariant = 200;
        let q_t = make_quin(trigger);
        let q_i = make_quin(invariant);
        let q_other = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_i, q_i, q_i],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(evaluate_ltl_trace(
            &[q_i, q_t, q_other],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_i, q_other, q_t],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(evaluate_ltl_trace(
            &[],
            &LtlFormula::Release { trigger, invariant }
        ));
    }

    #[test]
    fn past_time_ltl_once_historically_since() {
        let (breach, duty, repaired) = (1u64, 2u64, 3u64);
        let trace = [make_quin(breach), make_quin(duty), make_quin(duty)];
        // Once: breach happened at some past point.
        assert!(evaluate_once(&trace, breach));
        assert!(!evaluate_once(&trace, repaired));
        // Historically: duty did NOT hold at every point (breach was first).
        assert!(!evaluate_historically(&trace, duty));
        assert!(evaluate_historically(&[make_quin(duty), make_quin(duty)], duty));
        // Since: the duty has held since the breach (every point after breach is duty).
        assert!(evaluate_since(&trace, duty, breach));
        // Not "since" if the consequent never held.
        assert!(!evaluate_since(&trace, duty, repaired));
    }

    #[test]
    fn streaming_safety_monitor_latches_on_violation() {
        let safe = 100u64;
        let mut mon = SafetyMonitor::new(safe);
        assert!(mon.step(safe) && mon.step(safe), "still safe");
        assert!(!mon.step(999), "a non-conforming event violates G(p)");
        assert!(!mon.step(safe), "violation latches permanently");
        assert!(!mon.is_safe());
    }

    #[test]
    fn metric_globally_and_allen_interval() {
        let (start, ok, bad) = (1u64, 2u64, 3u64);
        // From trigger@10, invariant `ok` holds across the window [10,40].
        let good = [timed(start, 10), timed(ok, 20), timed(ok, 35)];
        assert!(holds_globally_within(&good, start, ok, 30));
        // A bad event inside the window breaks it.
        let broken = [timed(start, 10), timed(bad, 25)];
        assert!(!holds_globally_within(&broken, start, ok, 30));
        // Allen-interval extraction: `ok` spans [20,35].
        assert_eq!(predicate_interval(&good, ok), Some((20, 35)));
        assert_eq!(predicate_interval(&good, bad), None);
    }
}
