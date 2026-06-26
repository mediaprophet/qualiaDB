//! Kaplan–Meier survival curve (ISL ch 11.3) — the nonparametric estimate of the
//! survival function `S(t)` from right-censored event times.
//!
//! At each distinct event time `tᵢ`, `S` drops by the factor `(1 − dᵢ/nᵢ)`, where
//! `dᵢ` is the number of events at `tᵢ` and `nᵢ` the number still at risk
//! (time ≥ `tᵢ`). Censored observations leave the risk set without an event.

use crate::solvers::learning::LearningError;

/// A fitted Kaplan–Meier estimator: a right-continuous step function.
#[derive(Debug, Clone)]
pub struct KaplanMeier {
    /// Distinct event times, ascending.
    pub event_times: Vec<f64>,
    /// Survival probability *after* each event time.
    pub survival: Vec<f64>,
    /// Number at risk at each event time.
    pub at_risk: Vec<usize>,
    /// Number of events at each event time.
    pub events: Vec<usize>,
}

impl KaplanMeier {
    /// Fit from `times` and `event` flags (`true` = event/death observed,
    /// `false` = right-censored). Fails closed on length mismatch / empty input.
    pub fn fit(times: &[f64], event: &[bool]) -> Result<Self, LearningError> {
        let n = times.len();
        if n == 0 || n != event.len() {
            return Err(LearningError::InvalidDimension);
        }
        // Order by time; ties resolved with events before censorings is not needed
        // for the standard estimator (we group by exact time).
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| times[a].partial_cmp(&times[b]).unwrap_or(core::cmp::Ordering::Equal));

        let mut event_times = Vec::new();
        let mut survival = Vec::new();
        let mut at_risk_v = Vec::new();
        let mut events_v = Vec::new();

        let mut s = 1.0;
        let mut i = 0;
        while i < n {
            let t = times[order[i]];
            // Count events and total observations at this exact time.
            let mut d = 0usize; // events at t
            let mut tied = 0usize; // total (events + censorings) at t
            let mut j = i;
            while j < n && times[order[j]] == t {
                if event[order[j]] {
                    d += 1;
                }
                tied += 1;
                j += 1;
            }
            let n_at_risk = n - i; // everyone with time ≥ t is still at risk
            if d > 0 {
                s *= 1.0 - d as f64 / n_at_risk as f64;
                event_times.push(t);
                survival.push(s);
                at_risk_v.push(n_at_risk);
                events_v.push(d);
            }
            let _ = tied;
            i = j;
        }

        Ok(Self { event_times, survival, at_risk: at_risk_v, events: events_v })
    }

    /// Estimated survival `S(t)` (right-continuous step). `1.0` before the first
    /// event time.
    pub fn survival_at(&self, t: f64) -> f64 {
        let mut s = 1.0;
        for (k, &et) in self.event_times.iter().enumerate() {
            if et <= t {
                s = self.survival[k];
            } else {
                break;
            }
        }
        s
    }

    /// Median survival time — the first event time at which `S(t) ≤ 0.5`. `None` if
    /// the curve never drops to 0.5 (heavy censoring).
    pub fn median_survival(&self) -> Option<f64> {
        self.event_times
            .iter()
            .zip(self.survival.iter())
            .find(|(_, &s)| s <= 0.5)
            .map(|(&t, _)| t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_censoring_matches_empirical() {
        // 4 events at times 1,2,3,4 → S drops 1→.75→.5→.25→0.
        let times = [1.0, 2.0, 3.0, 4.0];
        let event = [true, true, true, true];
        let km = KaplanMeier::fit(&times, &event).unwrap();
        assert!((km.survival_at(1.0) - 0.75).abs() < 1e-12);
        assert!((km.survival_at(2.0) - 0.5).abs() < 1e-12);
        assert!((km.survival_at(3.0) - 0.25).abs() < 1e-12);
        assert!((km.survival_at(0.5) - 1.0).abs() < 1e-12); // before first event
        assert_eq!(km.median_survival(), Some(2.0));
    }

    #[test]
    fn censoring_keeps_survival_higher() {
        // Times 1(event),2(censored),3(event),4(censored).
        let times = [1.0, 2.0, 3.0, 4.0];
        let event = [true, false, true, false];
        let km = KaplanMeier::fit(&times, &event).unwrap();
        // At t=1: 1 event of 4 at risk → S=0.75.
        assert!((km.survival_at(1.0) - 0.75).abs() < 1e-12);
        // At t=3: 1 event of 2 at risk (1 and 2 already gone) → S = 0.75·(1−1/2)=0.375.
        assert!((km.survival_at(3.0) - 0.375).abs() < 1e-12);
    }

    #[test]
    fn tied_events_drop_together() {
        // Two events at time 2.
        let times = [1.0, 2.0, 2.0, 4.0];
        let event = [true, true, true, true];
        let km = KaplanMeier::fit(&times, &event).unwrap();
        // t=1: S=0.75. t=2: 2 of 3 at risk → S=0.75·(1−2/3)=0.25.
        assert!((km.survival_at(2.0) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn guards() {
        assert_eq!(KaplanMeier::fit(&[], &[]).unwrap_err(), LearningError::InvalidDimension);
        assert_eq!(KaplanMeier::fit(&[1.0], &[true, false]).unwrap_err(), LearningError::InvalidDimension);
    }
}
