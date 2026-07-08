//! Continuous spawn/decay α ramps (P3).
//!
//! Replaces binary temporal on/off with fade-in/out over valid-time windows.

/// Compute the visibility α ∈ [0, 1] for an asset at `now` given its valid-time
/// interval and optional ramp durations (seconds).
///
/// - Before `valid_from - onset`: α = 0
/// - During onset ramp: linear 0 → 1
/// - Between onset end and decay start: α = 1
/// - During decay ramp: linear 1 → 0
/// - After `valid_until + decay`: α = 0
pub fn spawn_decay_alpha(
    now: u64,
    valid_from: u64,
    valid_until: Option<u64>,
    onset_secs: u64,
    decay_secs: u64,
) -> f32 {
    if now < valid_from.saturating_sub(onset_secs) {
        return 0.0;
    }
    if onset_secs > 0 && now < valid_from {
        let t = (now - valid_from.saturating_sub(onset_secs)) as f64 / onset_secs as f64;
        return t.clamp(0.0, 1.0) as f32;
    }
    if let Some(until) = valid_until {
        if now > until.saturating_add(decay_secs) {
            return 0.0;
        }
        if decay_secs > 0 && now > until {
            let t = 1.0 - (now - until) as f64 / decay_secs as f64;
            return t.clamp(0.0, 1.0) as f32;
        }
    }
    1.0
}

/// Whether an asset should be considered visible at all (α > 0).
#[inline]
pub fn temporally_active(
    now: u64,
    valid_from: u64,
    valid_until: Option<u64>,
    onset_secs: u64,
    decay_secs: u64,
) -> bool {
    spawn_decay_alpha(now, valid_from, valid_until, onset_secs, decay_secs) > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle_ramps() {
        let vf = 1000u64;
        let vu = 2000u64;
        let onset = 100u64;
        let decay = 100u64;

        assert_eq!(spawn_decay_alpha(899, vf, Some(vu), onset, decay), 0.0);
        assert!((spawn_decay_alpha(950, vf, Some(vu), onset, decay) - 0.5).abs() < 0.01);
        assert_eq!(spawn_decay_alpha(1500, vf, Some(vu), onset, decay), 1.0);
        assert!((spawn_decay_alpha(2050, vf, Some(vu), onset, decay) - 0.5).abs() < 0.01);
        assert_eq!(spawn_decay_alpha(2101, vf, Some(vu), onset, decay), 0.0);
    }

    #[test]
    fn open_ended_validity() {
        assert_eq!(spawn_decay_alpha(5000, 1000, None, 0, 0), 1.0);
        assert_eq!(spawn_decay_alpha(500, 1000, None, 0, 0), 0.0);
    }
}